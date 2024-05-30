use anyhow::bail;
use essential_lock::StdLock;
use essential_state_read_vm::StateRead;
use essential_storage::{
    failed_solution::{CheckOutcome, FailedSolution, SolutionFailReason, SolutionOutcomes},
    key_range, CommitData, QueryState, StateStorage, Storage,
};
use essential_types::{
    intent::{self, Intent},
    solution::Solution,
    ContentAddress, Hash, IntentAddress, Key, Signature, StorageLayout, Word,
};
use futures::future::FutureExt;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
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
    intent_time_index: BTreeMap<Duration, Vec<ContentAddress>>,
    solution_pool: HashSet<Hash>,
    solution_time_index: BTreeMap<Duration, Vec<Hash>>,
    failed_solution_pool: HashMap<Hash, Vec<SolutionFailReason>>,
    failed_solution_time_index: HashMap<Duration, Vec<Hash>>,
    solutions: HashMap<Hash, Solution>,
    /// Solved batches ordered by the time they were solved.
    solved: BTreeMap<Duration, Block>,
    solution_block_time_index: HashMap<Hash, Vec<Duration>>,
    state: HashMap<ContentAddress, BTreeMap<Key, Vec<Word>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Block {
    number: u64,
    timestamp: Duration,
    hashes: Vec<Hash>,
}

#[derive(Debug)]
struct IntentSet {
    storage_layout: StorageLayout,
    data: HashMap<ContentAddress, Intent>,
    signature: Signature,
}

impl IntentSet {
    /// All intent addresses ordered by their CA.
    fn intent_addrs(&self) -> Vec<&ContentAddress> {
        let mut addrs: Vec<_> = self.data.keys().collect();
        addrs.sort();
        addrs
    }

    /// All intents in the set, ordered by their CA.
    fn intents(&self) -> impl '_ + Iterator<Item = &Intent> {
        self.intent_addrs().into_iter().map(|addr| &self.data[addr])
    }

    /// Re-construct the `intent::SignedSet`.
    ///
    /// Intents in the returned set will be ordered by their CA.
    fn signed_set(&self) -> intent::SignedSet {
        let signature = self.signature.clone();
        let set = self.intents().cloned().collect();
        intent::SignedSet { set, signature }
    }
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
        let v = self.inner.apply(|i| update_state_batch(i, updates));
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
        signed: intent::SignedSet,
    ) -> anyhow::Result<()> {
        let intent::SignedSet { set, signature } = signed;

        let data: HashMap<_, _> = set
            .into_iter()
            .map(|intent| (essential_hash::content_addr(&intent), intent))
            .collect();

        let set_addr = essential_hash::intent_set_addr::from_intent_addrs(data.keys().cloned());

        let set = IntentSet {
            storage_layout,
            data,
            signature,
        };
        let time = SystemTime::now().duration_since(UNIX_EPOCH)?;
        self.inner.apply(|i| {
            let contains = i.intents.insert(set_addr.clone(), set);
            if contains.is_none() {
                i.intent_time_index
                    .entry(time)
                    .or_default()
                    .push(set_addr.clone());
            }
            i.state.entry(set_addr).or_default();
            Ok(())
        })
    }

    async fn insert_solution_into_pool(&self, solution: Solution) -> anyhow::Result<()> {
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
        let hash = essential_hash::hash(&solution);
        self.inner.apply(|i| {
            if i.solution_pool.insert(hash) {
                i.solution_time_index
                    .entry(timestamp)
                    .or_default()
                    .push(hash);
            }
            i.solutions.insert(hash, solution);
        });
        Ok(())
    }

    async fn move_solutions_to_solved(&self, solutions: &[Hash]) -> anyhow::Result<()> {
        self.inner.apply(|i| move_solutions_to_solved(i, solutions))
    }

    async fn move_solutions_to_failed(
        &self,
        solutions: &[(Hash, SolutionFailReason)],
    ) -> anyhow::Result<()> {
        let hashes: HashSet<_> = solutions.iter().map(|(h, _)| h).collect();
        self.inner
            .apply(|i| move_solutions_to_failed(i, solutions, hashes))
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
    ) -> anyhow::Result<Option<intent::SignedSet>> {
        let v = self
            .inner
            .apply(|i| Some(i.intents.get(address)?.signed_set()));
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
                    values::page_intents(
                        i.intent_time_index.values().flatten(),
                        &i.intents,
                        page,
                        PAGE_SIZE,
                    )
                });
                Ok(v)
            }
        }
    }

    async fn list_solutions_pool(&self, page: Option<usize>) -> anyhow::Result<Vec<Solution>> {
        Ok(self.inner.apply(|i| {
            let iter = i
                .solution_time_index
                .values()
                .flatten()
                .filter(|h| i.solution_pool.contains(*h));
            values::page_solutions(
                iter,
                |h| i.solutions.get(h).cloned(),
                page.unwrap_or(0),
                PAGE_SIZE,
            )
        }))
    }

    async fn list_failed_solutions_pool(
        &self,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<FailedSolution>> {
        Ok(self.inner.apply(|i| {
            let iter = i
                .failed_solution_pool
                .iter()
                .flat_map(|(h, r)| r.iter().map(|r| (*h, r.clone())));
            values::page_solutions(
                iter,
                |(h, r)| {
                    let solution = i.solutions.get(&h).cloned()?;
                    Some(FailedSolution {
                        solution,
                        reason: r,
                    })
                },
                page.unwrap_or(0),
                PAGE_SIZE,
            )
        }))
    }

    async fn list_winning_blocks(
        &self,
        time_range: Option<std::ops::Range<std::time::Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<essential_types::Block>> {
        let page = page.unwrap_or(0);
        self.inner.apply(|i| {
            values::page_winning_blocks(&i.solved, &i.solutions, time_range, page, PAGE_SIZE)
        })
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

    async fn get_solution(&self, solution_hash: Hash) -> anyhow::Result<Option<SolutionOutcomes>> {
        let r = self.inner.apply(|i| {
            i.solutions.get(&solution_hash).cloned().map(|s| {
                let outcome = i
                    .failed_solution_pool
                    .get(&solution_hash)
                    .into_iter()
                    .flatten()
                    .cloned()
                    .map(CheckOutcome::Fail)
                    .chain(
                        i.solution_block_time_index
                            .get(&solution_hash)
                            .into_iter()
                            .flatten()
                            .filter_map(|time| {
                                let b = i.solved.get(time)?;
                                Some(CheckOutcome::Success(b.number))
                            }),
                    )
                    .collect();
                SolutionOutcomes {
                    solution: s.clone(),
                    outcome,
                }
            })
        });
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

    fn commit_block(
        &self,
        data: CommitData,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        let CommitData {
            failed,
            solved,
            state_updates,
        } = data;
        let hashes: HashSet<_> = failed.iter().map(|(h, _)| h).collect();
        let r = self.inner.apply(|i| {
            move_solutions_to_failed(i, failed, hashes)?;
            move_solutions_to_solved(i, solved)?;
            update_state_batch(i, state_updates);
            Ok(())
        });
        async { r }
    }
}

fn move_solutions_to_failed(
    i: &mut Inner,
    solutions: &[(Hash, SolutionFailReason)],
    hashes: HashSet<&Hash>,
) -> Result<(), anyhow::Error> {
    let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
    let solutions = solutions.iter().filter_map(|(h, r)| {
        if i.solution_pool.remove(h) {
            Some((*h, r.clone()))
        } else {
            None
        }
    });

    for v in i.solution_time_index.values_mut() {
        v.retain(|h| !hashes.contains(h));
    }
    i.solution_time_index.retain(|_, v| !v.is_empty());

    for (hash, reason) in solutions {
        i.failed_solution_pool.entry(hash).or_default().push(reason);
        i.failed_solution_time_index
            .entry(time)
            .or_default()
            .push(hash);
    }

    Ok(())
}

fn move_solutions_to_solved(i: &mut Inner, solutions: &[Hash]) -> Result<(), anyhow::Error> {
    if solutions.is_empty() {
        return Ok(());
    }

    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;

    if i.solved.contains_key(&timestamp) {
        bail!("Two blocks created at the same time");
    }
    for hash in solutions {
        i.solution_block_time_index
            .entry(*hash)
            .or_default()
            .push(timestamp);
    }
    let solutions = solutions
        .iter()
        .filter(|h| i.solution_pool.remove(*h))
        .cloned()
        .collect();
    let number = i.solved.len() as u64;
    let batch = Block {
        number,
        timestamp,
        hashes: solutions,
    };
    i.solved.insert(timestamp, batch);
    Ok(())
}

fn update_state_batch<U>(i: &mut Inner, updates: U) -> Vec<Vec<i64>>
where
    U: IntoIterator<Item = (ContentAddress, Key, Vec<Word>)>,
{
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
