#![deny(missing_docs)]
//! # Storage
//!
//! Trait for the storage layer of the Essential platform.

use std::{future::Future, ops::Range, time::Duration};

use essential_types::{
    contract::{Contract, SignedContract},
    predicate::Predicate,
    solution::Solution,
    Block, ContentAddress, Hash, Key, PredicateAddress, Word,
};
use failed_solution::{FailedSolution, SolutionFailReason, SolutionOutcomes};

/// Module for failed solution struct.
pub mod failed_solution;
/// Module for streams.
pub mod streams;

/// Data to commit after a block has been built.
/// This data should all be committed atomically.
pub struct CommitData<'a> {
    /// Failed solutions
    pub failed: &'a [(Hash, SolutionFailReason)],
    /// Solved solutions
    pub solved: &'a [Hash],
    /// State updates
    pub state_updates: Box<dyn Iterator<Item = (ContentAddress, Key, Vec<Word>)> + 'a>,
}

/// Storage trait for the Essential platform.
/// All inserts and updates are idempotent.
pub trait Storage: StateStorage {
    // Updates
    /// Insert a contract with their storage layout.
    fn insert_contract(
        &self,
        predicate: SignedContract,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Add a solution to the pool of unsolved solutions.
    fn insert_solution_into_pool(
        &self,
        solution: Solution,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Move these solutions from the pool to the solved state.
    fn move_solutions_to_solved(
        &self,
        solutions: &[Hash],
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Move these solutions from the pool to the failed state.
    fn move_solutions_to_failed(
        &self,
        solutions: &[(Hash, SolutionFailReason)],
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;

    // Reads
    /// Get an individual predicate.
    /// Note that the same predicate might be in multiple contracts.
    fn get_predicate(
        &self,
        address: &PredicateAddress,
    ) -> impl Future<Output = anyhow::Result<Option<Predicate>>> + Send;

    /// Get the entire contract.
    fn get_contract(
        &self,
        address: &ContentAddress,
    ) -> impl Future<Output = anyhow::Result<Option<SignedContract>>> + Send;

    /// List all contracts. This will paginate the results. The page is 0-indexed.
    /// A time range can optionally be provided to filter the results.
    /// The time is duration since the Unix epoch.
    fn list_contracts(
        &self,
        time_range: Option<Range<Duration>>,
        page: Option<usize>,
    ) -> impl Future<Output = anyhow::Result<Vec<Contract>>> + Send;

    /// Subscribe to new contracts from a given start page or start time.
    /// This will return all the contracts from that point then continue to stream
    /// as new contracts are added.
    fn subscribe_contracts(
        self,
        start_time: Option<Duration>,
        start_page: Option<usize>,
    ) -> impl futures::Stream<Item = anyhow::Result<Contract>> + Send + 'static;

    /// List all solutions in the pool.
    fn list_solutions_pool(
        &self,
        page: Option<usize>,
    ) -> impl Future<Output = anyhow::Result<Vec<Solution>>> + Send;

    /// List all failed solutions in the pool.
    fn list_failed_solutions_pool(
        &self,
        page: Option<usize>,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<FailedSolution>>> + Send;

    /// List all blocks of solutions that have been solved.
    fn list_blocks(
        &self,
        time_range: Option<Range<Duration>>,
        page: Option<usize>,
    ) -> impl Future<Output = anyhow::Result<Vec<Block>>> + Send;

    /// Subscribe to new blocks from a given block number or start page or start time.
    /// This will return all the blocks from that point then continue to stream
    /// as new blocks are added.
    fn subscribe_blocks(
        self,
        start_time: Option<Duration>,
        block_number: Option<u64>,
        start_page: Option<usize>,
    ) -> impl futures::Stream<Item = anyhow::Result<Block>> + Send + 'static;

    /// Get failed solution and its failing reason.
    fn get_solution(
        &self,
        solution_hash: Hash,
    ) -> impl std::future::Future<Output = anyhow::Result<Option<SolutionOutcomes>>> + Send;

    /// Prune failed solutions that failed before the provided duration.
    fn prune_failed_solutions(
        &self,
        older_than: Duration,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;

    /// Commit block data atomically.
    fn commit_block(
        &self,
        data: CommitData,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

/// Storage trait just for state reads and writes.
pub trait StateStorage: QueryState {
    /// Update the state of a content address.
    fn update_state(
        &self,
        address: &ContentAddress,
        key: &Key,
        value: Vec<Word>,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<Word>>> + Send;

    /// Update a batch of state in one transaction.
    fn update_state_batch<U>(
        &self,
        updates: U,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<Vec<Word>>>> + Send
    where
        U: IntoIterator<Item = (ContentAddress, Key, Vec<Word>)> + Send;
}

/// Storage trait for reading state.
pub trait QueryState {
    /// Query the state of a content address.
    fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<Word>>> + Send;
}

/// Get a range of words from the state.
pub async fn key_range<S, E>(
    storage: &S,
    contract_addr: ContentAddress,
    mut key: Key,
    num_words: usize,
) -> Result<Vec<Vec<Word>>, E>
where
    S: QueryState + Send,
    E: From<anyhow::Error>,
{
    let mut words = vec![];
    for _ in 0..num_words {
        let slot = storage.query_state(&contract_addr, &key).await?;
        words.push(slot);
        key = next_key(key).ok_or_else(|| anyhow::anyhow!("Failed to find next key"))?
    }
    Ok(words)
}

/// Calculate the next key.
pub fn next_key(mut key: Key) -> Option<Key> {
    for w in key.iter_mut().rev() {
        match *w {
            Word::MAX => *w = Word::MIN,
            _ => {
                *w += 1;
                return Some(key);
            }
        }
    }
    None
}
