use anyhow::bail;
use essential_lock::StdLock;
use essential_state_read_vm::StateRead;
use essential_storage::{
    failed_solution::{CheckOutcome, FailedSolution, SolutionFailReason, SolutionOutcomes},
    key_range, CommitData, QueryState, StateStorage, Storage,
};
use essential_types::{
    contract::{Contract, SignedContract},
    predicate::Predicate,
    solution::Solution,
    ContentAddress, Hash, Key, PredicateAddress, Signature, Word,
};
use futures::{future::FutureExt, StreamExt};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

mod values;

/// Amount of values returned in a single page.
const PAGE_SIZE: usize = 100;

#[derive(Clone)]
pub struct MemoryStorage {
    inner: Arc<StdLock<Inner>>,
    streams: essential_storage::streams::Notify,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Debug)]
struct Inner {
    contracts: HashMap<ContentAddress, ContractWithAddresses>,
    predicates: HashMap<ContentAddress, Predicate>,
    contract_time_index: BTreeMap<Duration, Vec<ContentAddress>>,
    solution_pool: HashSet<Hash>,
    solution_time_index: BTreeMap<Duration, Vec<Hash>>,
    failed_solution_pool: HashMap<Hash, Vec<(SolutionFailReason, Duration)>>,
    failed_solution_time_index: BTreeMap<Duration, Vec<Hash>>,
    solutions: HashMap<Hash, Solution>,
    /// Solved batches ordered by the time they were solved.
    solved: BTreeMap<Duration, Block>,
    block_number_index: HashMap<u64, Duration>,
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
struct ContractWithAddresses {
    salt: Hash,
    data: HashSet<ContentAddress>,
    signature: Signature,
}

impl ContractWithAddresses {
    /// All predicate addresses ordered by their CA.
    fn predicate_addrs(&self) -> Vec<&ContentAddress> {
        let mut addrs: Vec<_> = self.data.iter().collect();
        addrs.sort();
        addrs
    }

    /// All predicates in the contract, ordered by their CA.
    fn predicates_owned(&self, predicates: &HashMap<ContentAddress, Predicate>) -> Vec<Predicate> {
        self.predicate_addrs()
            .into_iter()
            .filter_map(|addr| predicates.get(addr).cloned())
            .collect()
    }

    /// Re-construct the `SignedContract`.
    ///
    /// Predicates in the returned contract will be ordered by their CA.
    fn signed_contract(&self, predicates: &HashMap<ContentAddress, Predicate>) -> SignedContract {
        let signature = self.signature.clone();
        let predicates = self.predicates_owned(predicates);
        SignedContract {
            contract: Contract {
                salt: self.salt,
                predicates,
            },
            signature,
        }
    }
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(StdLock::new(Inner::default())),
            streams: essential_storage::streams::Notify::new(),
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
    async fn insert_contract(&self, signed: SignedContract) -> anyhow::Result<()> {
        let SignedContract {
            contract,
            signature,
        } = signed;

        let salt = contract.salt;

        let data: HashMap<_, _> = contract
            .predicates
            .into_iter()
            .map(|p| (essential_hash::content_addr(&p), p))
            .collect();

        let contract_addr =
            essential_hash::contract_addr::from_predicate_addrs(data.keys().cloned(), &salt);

        let contract_with_addrs = ContractWithAddresses {
            salt,
            data: data.keys().cloned().collect(),
            signature,
        };
        let time = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let r = self.inner.apply(|i| {
            i.predicates.extend(data);
            let contains = i
                .contracts
                .insert(contract_addr.clone(), contract_with_addrs);
            if contains.is_none() {
                i.contract_time_index
                    .entry(time)
                    .or_default()
                    .push(contract_addr.clone());
            }
            i.state.entry(contract_addr).or_default();
            Ok(())
        });

        // There is a new contract.
        self.streams.notify_new_contracts();
        r
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
        let hashes: HashSet<_> = solutions.iter().collect();
        let r = self
            .inner
            .apply(|i| move_solutions_to_solved(i, solutions, hashes));

        // There is a new block.
        self.streams.notify_new_blocks();
        r
    }

    async fn move_solutions_to_failed(
        &self,
        solutions: &[(Hash, SolutionFailReason)],
    ) -> anyhow::Result<()> {
        let hashes: HashSet<_> = solutions.iter().map(|(h, _)| h).collect();
        self.inner
            .apply(|i| move_solutions_to_failed(i, solutions, hashes))
    }

    async fn get_predicate(&self, address: &PredicateAddress) -> anyhow::Result<Option<Predicate>> {
        let v = self.inner.apply(|i| {
            if i.contracts
                .get(&address.contract)
                .map_or(true, |c| !c.data.contains(&address.predicate))
            {
                return None;
            }
            let predicate = i.predicates.get(&address.predicate)?;
            Some(predicate.clone())
        });
        Ok(v)
    }

    async fn get_contract(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<SignedContract>> {
        let v = self
            .inner
            .apply(|i| Some(i.contracts.get(address)?.signed_contract(&i.predicates)));
        Ok(v)
    }

    async fn list_contracts(
        &self,
        time_range: Option<std::ops::Range<std::time::Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Contract>> {
        let page = page.unwrap_or(0);
        match time_range {
            Some(range) => {
                let v = self.inner.apply(|i| {
                    values::page_contract_by_time(
                        &i.contract_time_index,
                        &i.contracts,
                        &i.predicates,
                        range,
                        page,
                        PAGE_SIZE,
                    )
                });
                Ok(v)
            }
            None => {
                let v = self.inner.apply(|i| {
                    values::page_contract(
                        i.contract_time_index.values().flatten(),
                        &i.contracts,
                        &i.predicates,
                        page,
                        PAGE_SIZE,
                    )
                });
                Ok(v)
            }
        }
    }

    fn subscribe_contracts(
        self,
        start_time: Option<Duration>,
        start_page: Option<usize>,
    ) -> impl futures::Stream<Item = anyhow::Result<Contract>> + Send + 'static {
        let new_contracts = self.streams.subscribe_contracts();
        let init = essential_storage::streams::StreamState::new(start_page, start_time, None);
        futures::stream::unfold(init, move |state| {
            let storage = self.clone();
            essential_storage::streams::next_data(
                // List contracts expects a Range not a RangeFrom so we give it a range from
                // start till the end of time.
                move |get| {
                    let storage = storage.clone();
                    async move {
                        storage
                            .list_contracts(get.time.map(|s| s..Duration::MAX), Some(get.page))
                            .await
                    }
                },
                new_contracts.clone(),
                state,
                PAGE_SIZE,
            )
        })
        .flat_map(futures::stream::iter)
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
            let iter = i.failed_solution_time_index.values().flat_map(|hashes| {
                hashes.iter().flat_map(|h| {
                    i.failed_solution_pool
                        .get(h)
                        .into_iter()
                        .flatten()
                        .map(|r| (*h, r.clone()))
                })
            });
            values::page_solutions(
                iter,
                |(h, r)| {
                    let solution = i.solutions.get(&h).cloned()?;
                    Some(FailedSolution {
                        solution,
                        reason: r.0,
                    })
                },
                page.unwrap_or(0),
                PAGE_SIZE,
            )
        }))
    }

    async fn list_blocks(
        &self,
        time_range: Option<std::ops::Range<std::time::Duration>>,
        block_number: Option<u64>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<essential_types::Block>> {
        let page = page.unwrap_or(0);
        self.inner.apply(|i| {
            values::page_blocks(
                &i.solved,
                &i.solutions,
                &i.block_number_index,
                time_range,
                block_number,
                page,
                PAGE_SIZE,
            )
        })
    }

    fn subscribe_blocks(
        self,
        start_time: Option<Duration>,
        block_number: Option<u64>,
        start_page: Option<usize>,
    ) -> impl futures::Stream<Item = anyhow::Result<essential_types::Block>> + Send + 'static {
        let new_blocks = self.streams.subscribe_blocks();
        let init =
            essential_storage::streams::StreamState::new(start_page, start_time, block_number);
        futures::stream::unfold(init, move |state| {
            let storage = self.clone();
            essential_storage::streams::next_data(
                // List blocks expects a Range not a RangeFrom so we give it a range from
                // start till the end of time.
                move |get| {
                    let storage = storage.clone();
                    async move {
                        storage
                            .list_blocks(
                                get.time.map(|s| s..Duration::MAX),
                                get.number,
                                Some(get.page),
                            )
                            .await
                    }
                },
                new_blocks.clone(),
                state,
                PAGE_SIZE,
            )
        })
        .flat_map(futures::stream::iter)
    }

    async fn get_solution(&self, solution_hash: Hash) -> anyhow::Result<Option<SolutionOutcomes>> {
        let r = self.inner.apply(|i| {
            i.solutions.get(&solution_hash).cloned().map(|s| {
                let mut outcomes: Vec<_> = i
                    .failed_solution_pool
                    .get(&solution_hash)
                    .into_iter()
                    .flatten()
                    .cloned()
                    .map(|(r, t)| (t, CheckOutcome::Fail(r)))
                    .chain(
                        i.solution_block_time_index
                            .get(&solution_hash)
                            .into_iter()
                            .flatten()
                            .filter_map(|time| {
                                let b = i.solved.get(time)?;
                                Some((*time, CheckOutcome::Success(b.number)))
                            }),
                    )
                    .collect();
                outcomes.sort_by_key(|(t, _)| *t);
                let outcome = outcomes.into_iter().map(|(_, o)| o).collect();
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
        let solved_hashes: HashSet<_> = solved.iter().collect();
        let r = self.inner.apply(|i| {
            move_solutions_to_failed(i, failed, hashes)?;
            move_solutions_to_solved(i, solved, solved_hashes)?;
            update_state_batch(i, state_updates);
            Ok(())
        });

        // There is a new block.
        self.streams.notify_new_blocks();
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
        i.failed_solution_pool
            .entry(hash)
            .or_default()
            .push((reason, time));
        i.failed_solution_time_index
            .entry(time)
            .or_default()
            .push(hash);
    }

    Ok(())
}

fn move_solutions_to_solved(
    i: &mut Inner,
    solutions: &[Hash],
    hashes: HashSet<&Hash>,
) -> Result<(), anyhow::Error> {
    if solutions.is_empty() {
        return Ok(());
    }

    if solutions.iter().all(|s| !i.solution_pool.contains(s)) {
        return Ok(());
    }

    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;

    if i.solved.contains_key(&timestamp) {
        bail!("Two blocks created at the same time");
    }

    for v in i.solution_time_index.values_mut() {
        v.retain(|h| !hashes.contains(h));
    }
    i.solution_time_index.retain(|_, v| !v.is_empty());

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
    let block = Block {
        number,
        timestamp,
        hashes: solutions,
    };
    i.solved.insert(timestamp, block);
    i.block_number_index.insert(number, timestamp);
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

    fn key_range(&self, contract_addr: ContentAddress, key: Key, num_words: usize) -> Self::Future {
        let storage = self.clone();
        async move { key_range(&storage, contract_addr, key, num_words).await }.boxed()
    }
}
