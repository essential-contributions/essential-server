use anyhow::bail;
use essential_lock::StdLock;
use essential_state_read_vm::StateRead;
use essential_storage::{
    failed_solution::{CheckOutcome, FailedSolution, SolutionFailReason, SolutionOutcome},
    key_range, QueryState, StateStorage, Storage,
};
use essential_types::{
    intent::Intent, solution::Solution, Batch, Block, ContentAddress, Hash, IntentAddress, Key,
    Signature, Signed, StorageLayout, Word,
};
use futures::future::FutureExt;
use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap, HashSet},
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
    vec,
};
use thiserror::Error;

#[cfg(test)]
mod tests;
mod values;

/// Amount of values returned in a single page.
const PAGE_SIZE: usize = 100;

#[derive(Clone)]
pub struct MemoryStorage {
    inner: Arc<StdLock<Inner>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
struct Inner {
    intents: HashMap<ContentAddress, IntentSet>,
    intent_time_index: BTreeMap<Duration, ContentAddress>,
    solution_pool: HashMap<Hash, Solution>,
    solution_time_index: BTreeMap<Duration, Vec<Hash>>,
    failed_solution_pool: HashMap<Hash, FailedSolution>,
    failed_solution_time_index: HashMap<Duration, Vec<Hash>>,
    /// Solved batches ordered by the time they were solved.
    solved: BTreeMap<Duration, Block>,
    solution_block_time_index: HashMap<Hash, Duration>,
    state: HashMap<ContentAddress, BTreeMap<Key, Vec<Word>>>,
}

#[derive(Debug)]
struct IntentSet {
    storage_layout: StorageLayout,
    order: Vec<ContentAddress>,
    data: HashMap<ContentAddress, Intent>,
    signature: Signature,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(StdLock::new(Inner::default())),
        }
    }
}

impl StateStorage for MemoryStorage {
    async fn update_state(
        &self,
        address: &ContentAddress,
        key: &Key,
        value: Vec<Word>,
    ) -> anyhow::Result<Vec<Word>> {
        self.inner.apply(|i| {
            let Some(map) = i.state.get_mut(address) else {
                bail!("No state for address, {:?}", address);
            };
            let v = if value.is_empty() {
                map.remove(key)
            } else {
                map.insert(key.clone(), value)
            };
            let v = v.unwrap_or_default();
            Ok(v)
        })
    }

    async fn update_state_batch<U>(&self, updates: U) -> anyhow::Result<Vec<Vec<Word>>>
    where
        U: IntoIterator<Item = (ContentAddress, Key, Vec<Word>)> + Send,
    {
        let v = self.inner.apply(|i| {
            updates
                .into_iter()
                .map(|(address, key, value)| {
                    let map = i.state.entry(address).or_default();
                    let v = if value.is_empty() {
                        map.remove(&key)
                    } else {
                        map.insert(key, value)
                    };
                    v.unwrap_or_default()
                })
                .collect()
        });
        Ok(v)
    }
}

impl QueryState for MemoryStorage {
    async fn query_state(&self, address: &ContentAddress, key: &Key) -> anyhow::Result<Vec<Word>> {
        let v = self.inner.apply(|i| {
            let map = i.state.get(address)?;
            let v = map.get(key)?;
            Some(v.clone())
        });
        Ok(v.unwrap_or_default())
    }
}

impl Storage for MemoryStorage {
    async fn insert_intent_set(
        &self,
        storage_layout: StorageLayout,
        intent: Signed<Vec<Intent>>,
    ) -> anyhow::Result<()> {
        let Signed { data, signature } = intent;
        // TODO: Refactor upon solving essential-contributions/essential-base#116.
        let order: Vec<_> = data
            .iter()
            .map(|intent| essential_hash::content_addr(&intent))
            .collect();
        let map = order.iter().cloned().zip(data).collect();
        let set_addr = essential_hash::intent_set_addr::from_intent_addrs(order.iter().cloned());

        let set = IntentSet {
            storage_layout,
            order,
            data: map,
            signature,
        };
        let time = SystemTime::now().duration_since(UNIX_EPOCH)?;
        self.inner.apply(|i| {
            if i.intent_time_index.contains_key(&time) {
                bail!("Two intent sets created at the same time");
            }
            let contains = i.intents.insert(set_addr.clone(), set);
            if contains.is_none() {
                i.intent_time_index.insert(time, set_addr.clone());
            }
            i.state.entry(set_addr).or_default();
            Ok(())
        })
    }

    async fn insert_solution_into_pool(&self, solution: Solution) -> anyhow::Result<()> {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
        let hash = essential_hash::hash(&solution);
        self.inner.apply(|i| {
            if i.solution_pool.insert(hash, solution).is_none() {
                i.solution_time_index
                    .entry(timestamp)
                    .or_default()
                    .push(hash);
            }
        });
        Ok(())
    }

    async fn move_solutions_to_solved(&self, solutions: &[Hash]) -> anyhow::Result<()> {
        if solutions.is_empty() {
            return Ok(());
        }
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
        self.inner.apply(|i| {
            if i.solved.contains_key(&timestamp) {
                bail!("Two blocks created at the same time");
            }
            i.solution_block_time_index
                .extend(solutions.iter().map(|h| (*h, timestamp)));
            let solutions = solutions
                .iter()
                .filter_map(|h| i.solution_pool.remove(h))
                .collect();
            let number = i.solved.len() as u64;
            let batch = Block {
                number,
                timestamp,
                batch: Batch { solutions },
            };
            i.solved.insert(timestamp, batch);
            Ok(())
        })
    }

    async fn move_solutions_to_failed(
        &self,
        solutions: &[(Hash, SolutionFailReason)],
    ) -> anyhow::Result<()> {
        let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
        let hashes: HashSet<_> = solutions.iter().map(|(h, _)| h).collect();
        self.inner.apply(|i| {
            let solutions = solutions
                .iter()
                .filter_map(|(h, r)| i.solution_pool.remove(h).map(|s| (*h, s, r.to_owned())));
            for v in i.solution_time_index.values_mut() {
                v.retain(|h| !hashes.contains(h));
            }
            i.solution_time_index.retain(|_, v| !v.is_empty());

            for (hash, solution, reason) in solutions {
                let contains = i
                    .failed_solution_pool
                    .insert(hash, FailedSolution { solution, reason });
                if contains.is_none() {
                    match i.failed_solution_time_index.entry(time) {
                        Entry::Occupied(mut occupied_entry) => {
                            occupied_entry.get_mut().push(hash);
                        }
                        Entry::Vacant(vacant_entry) => {
                            vacant_entry.insert(vec![hash]);
                        }
                    }
                }
            }

            Ok(())
        })
    }

    async fn get_intent(&self, address: &IntentAddress) -> anyhow::Result<Option<Intent>> {
        let v = self.inner.apply(|i| {
            let set = i.intents.get(&address.set)?;
            let intent = set.data.get(&address.intent)?;
            Some(intent.clone())
        });
        Ok(v)
    }

    async fn get_intent_set(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<Signed<Vec<Intent>>>> {
        let v = self.inner.apply(|i| {
            let set = i.intents.get(address)?;
            let data = set
                .order
                .iter()
                .map(|i| set.data.get(i).cloned())
                .collect::<Option<Vec<_>>>()?;
            Some(Signed {
                data,
                signature: set.signature.clone(),
            })
        });
        Ok(v)
    }

    async fn list_intent_sets(
        &self,
        time_range: Option<std::ops::Range<std::time::Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Vec<Intent>>> {
        let page = page.unwrap_or(0);
        match time_range {
            Some(range) => {
                let v = self.inner.apply(|i| {
                    values::page_intents_by_time(
                        &i.intent_time_index,
                        &i.intents,
                        range,
                        page,
                        PAGE_SIZE,
                    )
                });
                Ok(v)
            }
            None => {
                let v = self.inner.apply(|i| {
                    values::page_intents(i.intent_time_index.values(), &i.intents, page, PAGE_SIZE)
                });
                Ok(v)
            }
        }
    }

    async fn list_solutions_pool(&self) -> anyhow::Result<Vec<Solution>> {
        Ok(self.inner.apply(|i| {
            i.solution_time_index
                .values()
                .flatten()
                .filter_map(|h| i.solution_pool.get(h))
                .cloned()
                .collect()
        }))
    }

    async fn list_failed_solutions_pool(&self) -> anyhow::Result<Vec<FailedSolution>> {
        Ok(self
            .inner
            .apply(|i| i.failed_solution_pool.values().cloned().collect()))
    }

    async fn list_winning_blocks(
        &self,
        time_range: Option<std::ops::Range<std::time::Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Block>> {
        let page = page.unwrap_or(0);
        let v = self
            .inner
            .apply(|i| values::page_winning_blocks(&i.solved, time_range, page, PAGE_SIZE));
        Ok(v)
    }

    async fn get_storage_layout(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<StorageLayout>> {
        let v = self.inner.apply(|i| {
            let set = i.intents.get(address)?;
            Some(set.storage_layout.clone())
        });
        Ok(v)
    }

    async fn get_solution(&self, solution_hash: Hash) -> anyhow::Result<Option<SolutionOutcome>> {
        let r = self.inner.apply(|i| {
            i.failed_solution_pool
                .get(&solution_hash)
                .cloned()
                .map(Result::Err)
                .or_else(|| {
                    let time = i.solution_block_time_index.get(&solution_hash)?;
                    Some(Result::Ok(i.solved.get(time).cloned()?))
                })
        });

        let r = match r {
            Some(Err(failed)) => Some(SolutionOutcome {
                solution: failed.solution,
                outcome: CheckOutcome::Fail(failed.reason),
            }),
            // Do this find outside the lock to save the total amount of time the lock is held.
            Some(Ok(success)) => success
                .batch
                .solutions
                .iter()
                .find(|s| essential_hash::hash(&s) == solution_hash)
                .cloned()
                .map(|s| SolutionOutcome {
                    solution: s,
                    outcome: CheckOutcome::Success(success.number),
                }),
            None => None,
        };
        Ok(r)
    }

    async fn prune_failed_solutions(&self, older_than: Duration) -> anyhow::Result<()> {
        self.inner.apply(|i| {
            i.failed_solution_time_index.retain(|timestamp, hash| {
                let retain = *timestamp >= older_than;
                if !retain {
                    for hash in hash {
                        i.failed_solution_pool.remove(hash);
                    }
                }
                retain
            });
            Ok(())
        })
    }
}

#[derive(Debug, Error)]
pub enum MemoryStorageError {
    #[error("failed to read from memory storage")]
    ReadError(#[from] anyhow::Error),
    #[error("invalid key range")]
    KeyRangeError,
}

impl StateRead for MemoryStorage {
    type Error = MemoryStorageError;

    type Future =
        Pin<Box<dyn std::future::Future<Output = Result<Vec<Vec<Word>>, Self::Error>> + Send>>;

    fn key_range(&self, set_addr: ContentAddress, key: Key, num_words: usize) -> Self::Future {
        let storage = self.clone();
        async move { key_range(&storage, set_addr, key, num_words).await }.boxed()
    }
}
