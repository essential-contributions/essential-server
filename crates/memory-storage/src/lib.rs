use anyhow::bail;
use essential_state_read_vm::StateRead;
use essential_types::{
    intent::Intent,
    solution::{PartialSolution, Solution},
    Batch, Block, ContentAddress, Hash, IntentAddress, Key, Signature, Signed, StorageLayout, Word,
};
use futures::future::FutureExt;
use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap},
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use storage::{
    failed_solution::{FailedSolution, SolutionFailReason},
    word_range, StateStorage, Storage,
};
use thiserror::Error;
use utils::Lock;

#[cfg(test)]
mod tests;
mod values;

/// Amount of values returned in a single page.
const PAGE_SIZE: usize = 100;

#[derive(Clone)]
pub struct MemoryStorage {
    inner: Arc<Lock<Inner>>,
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
    solution_pool: HashMap<Hash, Signed<Solution>>,
    failed_solution_pool: HashMap<Hash, FailedSolution>,
    failed_solution_time_index: HashMap<Duration, Vec<Hash>>,
    partial_solution_pool: HashMap<Hash, Signed<PartialSolution>>,
    partial_solution_solved: HashMap<ContentAddress, Signed<PartialSolution>>,
    /// Solved batches ordered by the time they were solved.
    solved: BTreeMap<Duration, Block>,
    state: HashMap<ContentAddress, BTreeMap<Key, Word>>,
}

struct IntentSet {
    storage_layout: StorageLayout,
    order: Vec<ContentAddress>,
    data: HashMap<ContentAddress, Intent>,
    signature: Signature,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Lock::new(Inner::default())),
        }
    }
}

impl StateStorage for MemoryStorage {
    async fn update_state(
        &self,
        address: &ContentAddress,
        key: &Key,
        value: Option<Word>,
    ) -> anyhow::Result<Option<Word>> {
        self.inner.apply(|i| {
            let Some(map) = i.state.get_mut(address) else {
                bail!("No state for address, {:?}", address);
            };
            let v = match value {
                None => map.remove(key),
                Some(value) => map.insert(*key, value),
            };
            Ok(v)
        })
    }

    async fn update_state_batch<U>(&self, updates: U) -> anyhow::Result<Vec<Option<Word>>>
    where
        U: IntoIterator<Item = (ContentAddress, Key, Option<Word>)> + Send,
    {
        let v = self.inner.apply(|i| {
            updates
                .into_iter()
                .map(|(address, key, value)| {
                    let map = i.state.entry(address).or_default();
                    match value {
                        None => map.remove(&key),
                        Some(value) => map.insert(key, value),
                    }
                })
                .collect()
        });
        Ok(v)
    }

    async fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> anyhow::Result<Option<Word>> {
        let v = self.inner.apply(|i| {
            let map = i.state.get(address)?;
            let v = map.get(key)?;
            Some(*v)
        });
        Ok(v)
    }
}

impl Storage for MemoryStorage {
    async fn insert_intent_set(
        &self,
        storage_layout: StorageLayout,
        intent: Signed<Vec<Intent>>,
    ) -> anyhow::Result<()> {
        let Signed { data, signature } = intent;
        let hash = ContentAddress(utils::hash(&data));
        let order: Vec<_> = data
            .iter()
            .map(|i| ContentAddress(utils::hash(i)))
            .collect();
        let map = order.iter().cloned().zip(data.into_iter()).collect();
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
            let contains = i.intents.insert(hash.clone(), set);
            if contains.is_none() {
                i.intent_time_index.insert(time, hash.clone());
            }
            i.state.entry(hash).or_default();
            Ok(())
        })
    }

    async fn insert_solution_into_pool(&self, solution: Signed<Solution>) -> anyhow::Result<()> {
        let hash = utils::hash(&solution.data);
        self.inner.apply(|i| i.solution_pool.insert(hash, solution));
        Ok(())
    }

    async fn insert_partial_solution_into_pool(
        &self,
        solution: Signed<essential_types::solution::PartialSolution>,
    ) -> anyhow::Result<()> {
        let hash = utils::hash(&solution.data);
        self.inner
            .apply(|i| i.partial_solution_pool.insert(hash, solution));
        Ok(())
    }

    async fn move_solutions_to_solved(&self, solutions: &[Hash]) -> anyhow::Result<()> {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
        self.inner.apply(|i| {
            if i.solved.contains_key(&timestamp) {
                bail!("Two blocks created at the same time");
            }
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
        self.inner.apply(|i| {
            let solutions = solutions
                .iter()
                .filter_map(|(h, r)| i.solution_pool.remove(h).map(|s| (*h, s, r.to_owned())));
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

    async fn move_partial_solutions_to_solved(
        &self,
        partial_solutions: &[Hash],
    ) -> anyhow::Result<()> {
        self.inner.apply(|i| {
            let solutions = partial_solutions.iter().filter_map(|h| {
                i.partial_solution_pool
                    .remove(h)
                    .map(|s| (ContentAddress(*h), s))
            });
            for (hash, solution) in solutions {
                i.partial_solution_solved.insert(hash, solution);
            }
        });
        Ok(())
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

    async fn get_partial_solution(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<Signed<essential_types::solution::PartialSolution>>> {
        let v = self.inner.apply(|i| {
            i.partial_solution_pool
                .get(&address.0)
                .cloned()
                .or_else(|| i.partial_solution_solved.get(address).cloned())
        });
        Ok(v)
    }

    async fn is_partial_solution_solved(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<bool>> {
        let v = self.inner.apply(|i| {
            let in_solved = i.partial_solution_solved.contains_key(address);
            if in_solved {
                Some(true)
            } else {
                let in_pool = i.partial_solution_pool.contains_key(&address.0);
                if in_pool {
                    Some(false)
                } else {
                    None
                }
            }
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

    async fn list_solutions_pool(&self) -> anyhow::Result<Vec<Signed<Solution>>> {
        Ok(self
            .inner
            .apply(|i| i.solution_pool.values().cloned().collect()))
    }

    async fn list_failed_solutions_pool(&self) -> anyhow::Result<Vec<FailedSolution>> {
        Ok(self
            .inner
            .apply(|i| i.failed_solution_pool.values().cloned().collect()))
    }

    async fn list_partial_solutions_pool(
        &self,
    ) -> anyhow::Result<Vec<Signed<essential_types::solution::PartialSolution>>> {
        Ok(self
            .inner
            .apply(|i| i.partial_solution_pool.values().cloned().collect()))
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

    async fn get_failed_solution(
        &self,
        solution_hash: Hash,
    ) -> anyhow::Result<Option<FailedSolution>> {
        Ok(self
            .inner
            .apply(|i| i.failed_solution_pool.get(&solution_hash).cloned()))
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
        Pin<Box<dyn std::future::Future<Output = Result<Vec<Option<Word>>, Self::Error>> + Send>>;

    fn word_range(&self, set_addr: ContentAddress, key: Key, num_words: usize) -> Self::Future {
        let storage = self.clone();
        async move { word_range(&storage, set_addr, key, num_words).await }.boxed()
    }
}
