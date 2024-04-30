#![deny(missing_docs)]
//! # Storage
//!
//! Trait for the storage layer of the Essential platform.

use essential_types::{
    intent::Intent,
    solution::{PartialSolution, Solution},
    Block, ContentAddress, Hash, IntentAddress, Key, Signed, StorageLayout, Word,
};
use failed_solution::{FailedSolution, SolutionFailReason};
use std::{ops::Range, time::Duration};

/// Module for failed solution struct.
pub mod failed_solution;
/// Module for state write trait.
pub mod state_write;

// TODO: Maybe this warning is right,
// we will find out when we try to use this trait
// with tokio.
#[allow(async_fn_in_trait)]
/// Storage trait for the Essential platform.
/// All inserts and updates are idempotent.
pub trait Storage: StateStorage {
    // Updates
    /// Insert a set of intents with their storage layout.
    async fn insert_intent_set(
        &self,
        storage_layout: StorageLayout,
        intent: Signed<Vec<Intent>>,
    ) -> anyhow::Result<()>;

    /// Add a solution to the pool of unsolved solutions.
    async fn insert_solution_into_pool(&self, solution: Signed<Solution>) -> anyhow::Result<()>;

    /// Add a partial solution to the pool of unsolved partial solutions.
    async fn insert_partial_solution_into_pool(
        &self,
        solution: Signed<PartialSolution>,
    ) -> anyhow::Result<()>;

    /// Move these solutions from the pool to the solved state.
    async fn move_solutions_to_solved(&self, solutions: &[Hash]) -> anyhow::Result<()>;

    /// Move these solutions from the pool to the failed state.
    async fn move_solutions_to_failed(
        &self,
        solutions: &[(Hash, SolutionFailReason)],
    ) -> anyhow::Result<()>;

    /// Move these partial solutions from the pool to the solved state.
    async fn move_partial_solutions_to_solved(
        &self,
        partial_solutions: &[Hash],
    ) -> anyhow::Result<()>;

    // Reads
    /// Get an individual intent.
    /// Note that the same intent might be in multiple sets.
    async fn get_intent(&self, address: &IntentAddress) -> anyhow::Result<Option<Intent>>;

    /// Get the entire intent set.
    async fn get_intent_set(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<Signed<Vec<Intent>>>>;

    /// Get a partial solution from either the pool or the solved state.
    async fn get_partial_solution(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<Signed<PartialSolution>>>;

    /// Check if a partial solution is solved or not.
    async fn is_partial_solution_solved(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<bool>>;

    /// List all intents. This will paginate the results. The page is 0-indexed.
    /// A time range can optionally be provided to filter the results.
    /// The time is duration since the Unix epoch.
    async fn list_intent_sets(
        &self,
        time_range: Option<Range<Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Vec<Intent>>>;

    /// List all solutions in the pool.
    async fn list_solutions_pool(&self) -> anyhow::Result<Vec<Signed<Solution>>>;

    /// List all failed solutions in the pool.
    async fn list_failed_solutions_pool(&self) -> anyhow::Result<Vec<FailedSolution>>;

    /// List all partial solutions in the pool.
    async fn list_partial_solutions_pool(&self) -> anyhow::Result<Vec<Signed<PartialSolution>>>;

    /// List all blocks of solutions that have been solved.
    async fn list_winning_blocks(
        &self,
        time_range: Option<Range<Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Block>>;

    /// Get the storage layout of a content address.
    async fn get_storage_layout(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<StorageLayout>>;

    /// Get failed solution and its failing reason.
    async fn get_failed_solution(
        &self,
        solution_hash: Hash,
    ) -> anyhow::Result<Option<FailedSolution>>;

    /// Prune failed solutions that failed before the provided duration.
    async fn prune_failed_solutions(&self, older_than: Duration) -> anyhow::Result<()>;
}

/// Storage trait just for state reads and writes.
pub trait StateStorage {
    /// Update the state of a content address.
    fn update_state(
        &self,
        address: &ContentAddress,
        key: &Key,
        value: Option<Word>,
    ) -> impl std::future::Future<Output = anyhow::Result<Option<Word>>> + Send;

    /// Update a batch of state in one transaction.
    fn update_state_batch<U>(
        &self,
        updates: U,
    ) -> impl std::future::Future<Output = anyhow::Result<Vec<Option<Word>>>> + Send
    where
        U: IntoIterator<Item = (ContentAddress, Key, Option<Word>)> + Send;

    /// Query the state of a content address.
    fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> impl std::future::Future<Output = anyhow::Result<Option<Word>>> + Send;
}
