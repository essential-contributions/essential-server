use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use essential_types::{
    intent::Intent,
    solution::{PartialSolution, Solution},
    Batch, Block, ContentAddress, Hash, IntentAddress, Key, Signature, Signed, StorageLayout, Word,
};
use storage::{StateStorage, Storage};
use utils::Lock;

#[cfg(test)]
mod tests;

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
    // TODO: Is it possible that multiple intent sets are created at the
    // exact same time? This is nanosecond precision.
    intent_time_index: BTreeMap<Duration, ContentAddress>,
    solution_pool: HashMap<Hash, Signed<Solution>>,
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
        let v = self.inner.apply(|i| {
            let map = i.state.entry(address.clone()).or_default();
            match value {
                None => map.remove(key),
                Some(value) => map.insert(*key, value),
            }
        });
        Ok(v)
    }

    async fn update_state_batch<U>(&self, updates: U) -> anyhow::Result<Vec<Option<Word>>>
    where
        U: IntoIterator<Item = (ContentAddress, Key, Option<Word>)>,
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
        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        self.inner.apply(|i| {
            i.intents.insert(hash.clone(), set);
            i.intent_time_index.insert(time, hash);
        });
        Ok(())
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
        self.inner.apply(|i| {
            let solutions = solutions
                .iter()
                .filter_map(|h| i.solution_pool.remove(h))
                .collect();
            let number = i.solved.len() as u64;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap();
            let batch = Block {
                number,
                timestamp,
                batch: Batch { solutions },
            };
            i.solved.insert(timestamp, batch);
        });
        Ok(())
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
                    let start = page * PAGE_SIZE;
                    i.intent_time_index
                        .range(range)
                        .skip(start)
                        // TODO: Should this be silent when the intent set is missing?
                        // By construction it shouldn't ever be but still maybe it's better
                        // to check?
                        .filter_map(|(_, v)| {
                            Some(i.intents.get(v)?.data.values().cloned().collect())
                        })
                        .take(PAGE_SIZE)
                        .collect()
                });
                Ok(v)
            }
            None => {
                let v = self.inner.apply(|i| {
                    let start = page * PAGE_SIZE;
                    i.intent_time_index
                        .iter()
                        .skip(start)
                        // TODO: Should this be silent when the intent set is missing?
                        // By construction it shouldn't ever be but still maybe it's better
                        // to check?
                        .filter_map(|(_, v)| {
                            Some(i.intents.get(v)?.data.values().cloned().collect())
                        })
                        .take(PAGE_SIZE)
                        .collect()
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
        match time_range {
            Some(range) => {
                let v = self.inner.apply(|i| {
                    let start = page * PAGE_SIZE;
                    i.solved
                        .range(range)
                        .skip(start)
                        .map(|(_, v)| v.clone())
                        .take(PAGE_SIZE)
                        .collect()
                });
                Ok(v)
            }
            None => {
                let v = self.inner.apply(|i| {
                    let start = page * PAGE_SIZE;
                    i.solved
                        .iter()
                        .skip(start)
                        .map(|(_, v)| v.clone())
                        .take(PAGE_SIZE)
                        .collect()
                });
                Ok(v)
            }
        }
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
}
