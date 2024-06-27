use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};

use essential_constraint_vm::{Access, SolutionAccess, StateSlots};
use essential_lock::StdLock;
use essential_server_types::{
    QueryStateReads, QueryStateReadsOutput, Slots, SlotsRequest, StateReadRequestType,
};
use essential_state_read_vm::{asm::Op, GasLimit, StateRead};
use essential_storage::{next_key, QueryState, StateStorage};
use essential_transaction_storage::TransactionStorage;
use essential_types::{ContentAddress, Key, Value};
use futures::FutureExt;

use crate::solution::create_post_state;

#[cfg(test)]
mod tests;

/// A wrapper around a state storage that records the keys and values read.
#[derive(Clone)]
struct Recorder<S> {
    inner: Arc<Inner<S>>,
    enabled: bool,
}

struct Inner<S> {
    storage: TransactionStorage<S>,
    record: StdLock<Recording>,
}

/// A recording of keys and values read at each contract address.
struct Recording(BTreeMap<ContentAddress, BTreeMap<Key, Value>>);

impl<S> Recorder<S> {
    fn new(storage: TransactionStorage<S>, enable: bool) -> Self {
        let i = Inner {
            storage,
            record: StdLock::new(Recording(BTreeMap::new())),
        };
        Self {
            inner: Arc::new(i),
            enabled: enable,
        }
    }

    /// Record new key value pairs at a contract address.
    fn record(&self, contract: ContentAddress, f: impl FnOnce(&mut BTreeMap<Key, Value>)) {
        self.inner.record.apply(|r| {
            f(r.0.entry(contract).or_default());
        });
    }

    /// Get the inner recording.
    fn into_recording(self) -> Recording {
        self.inner
            .record
            .apply(|r| Recording(std::mem::take(&mut r.0)))
    }
}

/// Run a state read query.
///
/// This will execute the state read programs provided on
/// the pre and post state.
/// The output varies depending on the request type.
/// The output can include:
/// - The keys and values that were read on the pre state run.
/// - The pre and post state slots.
pub(crate) async fn query_state_reads<S>(
    storage: TransactionStorage<S>,
    query: QueryStateReads,
) -> anyhow::Result<QueryStateReadsOutput>
where
    S: StateStorage + Clone + Send + Sync + 'static,
{
    let QueryStateReads {
        state_read,
        solution,
        index,
        request_type,
    } = query;

    // Get the transient data and mutable keys.
    let transient_data = essential_constraint_vm::transient_data(&solution);
    let mutable_keys = essential_constraint_vm::mut_keys_set(&solution, index);

    // Create empty slots.
    let mut slots = Slots::default();

    // Apply the mutations.
    let post_state = create_post_state(&storage, &solution)?;

    // Wrap the pre state storage with a recorder.
    // Enable the recorder if the request type is not slots.
    let pre_state = Recorder::new(
        storage,
        !matches!(request_type, StateReadRequestType::Slots(_)),
    );

    // Get a view of the post state storage.
    let post_state = post_state.view();

    // Run each state read program on the pre and post state.
    for read in state_read {
        // Create the solution access.
        let solution = SolutionAccess::new(&solution, index, &mutable_keys, &transient_data);

        // Update the post slots with the read values.
        slots
            .pre
            .extend(read_state(&pre_state, solution, &slots, read.clone()).await?);

        // Update the post slots with the read values.
        slots
            .post
            .extend(read_state(&post_state, solution, &slots, read).await?);
    }

    // Get the key, value pair recording.
    let read_kvs = pre_state.into_recording().0;

    // Return the requested output.
    let out = match request_type {
        StateReadRequestType::All(SlotsRequest::All) => QueryStateReadsOutput::All(read_kvs, slots),
        StateReadRequestType::All(SlotsRequest::Pre) => {
            slots.post.clear();
            QueryStateReadsOutput::All(read_kvs, slots)
        }
        StateReadRequestType::All(SlotsRequest::Post) => {
            slots.pre.clear();
            QueryStateReadsOutput::All(read_kvs, slots)
        }
        StateReadRequestType::Reads => QueryStateReadsOutput::Reads(read_kvs),
        StateReadRequestType::Slots(SlotsRequest::All) => QueryStateReadsOutput::Slots(slots),
        StateReadRequestType::Slots(SlotsRequest::Pre) => {
            slots.post.clear();
            QueryStateReadsOutput::Slots(slots)
        }
        StateReadRequestType::Slots(SlotsRequest::Post) => {
            slots.pre.clear();
            QueryStateReadsOutput::Slots(slots)
        }
    };

    Ok(out)
}

async fn read_state<S>(
    state: &S,
    solution: SolutionAccess<'_>,
    slots: &Slots,
    read: Vec<u8>,
) -> anyhow::Result<Vec<Value>>
where
    S: StateRead,
    <S as StateRead>::Error: Send + Sync + 'static,
{
    // Create the access using the updated slots.
    let access = Access {
        solution,
        state_slots: StateSlots {
            pre: &slots.pre,
            post: &slots.post,
        },
    };

    // Create a fresh state read vm.
    let mut vm = essential_state_read_vm::Vm::default();

    // Run the state read program on the post state.
    vm.exec_bytecode_iter(read, access, state, &|_: &Op| 1, GasLimit::UNLIMITED)
        .await?;

    Ok(vm.into_state_slots())
}

impl<S> StateRead for Recorder<S>
where
    S: StateStorage + Clone + Send + Sync + 'static,
{
    type Error = anyhow::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Vec<Value>, Self::Error>> + Send>>;

    fn key_range(
        &self,
        contract_addr: essential_types::ContentAddress,
        key: essential_types::Key,
        num_values: usize,
    ) -> Self::Future {
        let s = self.clone();
        let mut out = Vec::with_capacity(num_values);
        async move {
            // Read the keys and values.
            let values =
                key_range(&s.inner.storage, contract_addr.clone(), key, num_values).await?;

            // Record the keys and values if enabled
            // otherwise just return the values.
            if s.enabled {
                s.record(contract_addr, |r| {
                    for (k, v) in values {
                        out.push(v.clone());
                        r.insert(k, v);
                    }
                });
            } else {
                out = values.into_iter().map(|(_, v)| v).collect();
            }
            Ok(out)
        }
        .boxed()
    }
}

// Key range query that collects the keys and values read.
async fn key_range<S>(
    storage: &TransactionStorage<S>,
    contract_addr: ContentAddress,
    mut key: Key,
    num_words: usize,
) -> anyhow::Result<Vec<(Key, Value)>>
where
    S: QueryState + Send,
{
    let mut words = Vec::with_capacity(num_words);
    for _ in 0..num_words {
        let slot = storage.query_state(&contract_addr, &key).await?;
        words.push((key.clone(), slot));
        key = next_key(key).ok_or_else(|| anyhow::anyhow!("Failed to find next key"))?
    }
    Ok(words)
}
