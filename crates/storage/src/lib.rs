#![deny(missing_docs)]
//! # Storage
//!
//! Trait for the storage layer of the Essential platform.

use std::{future::Future, ops::Range, time::Duration};

use essential_types::{
    intent::Intent,
    solution::{PartialSolution, Solution},
    Block, ContentAddress, Hash, IntentAddress, Key, Signed, StorageLayout, Word,
};

/// Storage trait for the Essential platform.
/// All inserts and updates are idempotent.
pub trait Storage: StateStorage {
    // Updates
    /// Insert a set of intents with their storage layout.
    fn insert_intent_set(
        &self,
        storage_layout: StorageLayout,
        intent: Signed<Vec<Intent>>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Add a solution to the pool of unsolved solutions.
    fn insert_solution_into_pool(
        &self,
        solution: Signed<Solution>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Add a partial solution to the pool of unsolved partial solutions.
    fn insert_partial_solution_into_pool(
        &self,
        solution: Signed<PartialSolution>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Move these solutions from the pool to the solved state.
    fn move_solutions_to_solved(
        &self,
        solutions: &[Hash],
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Move these partial solutions from the pool to the solved state.
    fn move_partial_solutions_to_solved(
        &self,
        partial_solutions: &[Hash],
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    // Reads
    /// Get an individual intent.
    /// Note that the same intent might be in multiple sets.
    fn get_intent(
        &self,
        address: &IntentAddress,
    ) -> impl Future<Output = anyhow::Result<Option<Intent>>> + Send;

    /// Get the entire intent set.
    fn get_intent_set(
        &self,
        address: &ContentAddress,
    ) -> impl Future<Output = anyhow::Result<Option<Signed<Vec<Intent>>>>> + Send;

    /// Get a partial solution from either the pool or the solved state.
    fn get_partial_solution(
        &self,
        address: &ContentAddress,
    ) -> impl Future<Output = anyhow::Result<Option<Signed<PartialSolution>>>> + Send;

    /// Check if a partial solution is solved or not.
    fn is_partial_solution_solved(
        &self,
        address: &ContentAddress,
    ) -> impl Future<Output = anyhow::Result<Option<bool>>> + Send;

    /// List all intents. This will paginate the results. The page is 0-indexed.
    /// A time range can optionally be provided to filter the results.
    /// The time is duration since the Unix epoch.
    fn list_intent_sets(
        &self,
        time_range: Option<Range<Duration>>,
        page: Option<usize>,
    ) -> impl Future<Output = anyhow::Result<Vec<Vec<Intent>>>> + Send;

    /// List all solutions in the pool.
    fn list_solutions_pool(
        &self,
    ) -> impl Future<Output = anyhow::Result<Vec<Signed<Solution>>>> + Send;

    /// List all partial solutions in the pool.
    fn list_partial_solutions_pool(
        &self,
    ) -> impl Future<Output = anyhow::Result<Vec<Signed<PartialSolution>>>> + Send;

    /// List all blocks of solutions that have been solved.
    fn list_winning_blocks(
        &self,
        time_range: Option<Range<Duration>>,
        page: Option<usize>,
    ) -> impl Future<Output = anyhow::Result<Vec<Block>>> + Send;

    /// Get the storage layout of a content address.
    fn get_storage_layout(
        &self,
        address: &ContentAddress,
    ) -> impl Future<Output = anyhow::Result<Option<StorageLayout>>> + Send;
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
