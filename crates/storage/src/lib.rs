#![deny(missing_docs)]
//! # Storage
//!
//! Trait for the storage layer of the Essential platform.

use std::{ops::Range, time::Duration};

use essential_types::{
    intent::Intent, solution::Solution, Block, ContentAddress, Eoa, Hash, IntentAddress, Key,
    Signed, StorageLayout, Word,
};

// TODO: Maybe this warning is right,
// we will find out when we try to use this trait
// with tokio.
#[allow(async_fn_in_trait)]
/// Storage trait for the Essential platform.
/// All inserts and updates are idempotent.
pub trait Storage {
    // Updates
    /// Insert a set of intents with their storage layout.
    async fn insert_intent_set(
        &self,
        storage_layout: StorageLayout,
        intent: Signed<Vec<Intent>>,
    ) -> anyhow::Result<()>;

    /// Add an EOA account that state can be added too.
    async fn insert_eoa(&self, eoa: Eoa) -> anyhow::Result<()>;

    /// Add a solution to the pool of unsolved solutions.
    async fn insert_solution_into_pool(&self, solution: Signed<Solution>) -> anyhow::Result<()>;

    /// Move these solutions from the pool to the solved state.
    async fn move_solutions_to_solved(&self, solutions: &[Hash]) -> anyhow::Result<()>;

    /// Update the state of a content address.
    async fn update_state(
        &self,
        address: &ContentAddress,
        key: &Key,
        value: Option<Word>,
    ) -> anyhow::Result<Option<Word>>;

    /// Update the state of an EOA.
    async fn update_eoa_state(
        &self,
        address: &Eoa,
        key: &Key,
        value: Option<Word>,
    ) -> anyhow::Result<Option<Word>>;

    // Reads
    /// Get an individual intent.
    /// Note that the same intent might be in multiple sets.
    async fn get_intent(&self, address: &IntentAddress) -> anyhow::Result<Option<Intent>>;

    /// Get the entire intent set.
    async fn get_intent_set(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<Signed<Vec<Intent>>>>;

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

    /// List all blocks of solutions that have been solved.
    async fn list_winning_blocks(
        &self,
        time_range: Option<Range<Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Block>>;

    /// Query the state of a content address.
    async fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> anyhow::Result<Option<Word>>;

    /// Query the state of an EOA.
    async fn query_eoa_state(&self, address: &Eoa, key: &Key) -> anyhow::Result<Option<Word>>;

    /// Get the storage layout of a content address.
    async fn get_storage_layout(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<StorageLayout>>;
}
