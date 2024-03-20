use std::{ops::Range, time::Duration};

use essential_types::{
    intent::Intent, solution::Solution, ContentAddress, Hash, IntentAddress, Key, KeyRange, Word,
};

use placeholder::{Batch, Signed, StorageLayout};

pub trait Storage {
    // Updates
    async fn insert_intent_set(
        &self,
        storage_layout: StorageLayout,
        intent: Signed<Vec<Intent>>,
    ) -> anyhow::Result<()>;
    async fn insert_solution_into_pool(&self, solution: Signed<Solution>) -> anyhow::Result<()>;
    async fn move_solutions_to_solved(&self, solutions: &[Hash]) -> anyhow::Result<()>;
    async fn update_state(
        &self,
        address: &ContentAddress,
        key: &Key,
        value: Option<Word>,
    ) -> anyhow::Result<Option<Word>>;
    async fn update_state_range(
        &self,
        address: &ContentAddress,
        keys: &KeyRange,
        values: Vec<Option<Word>>,
    ) -> anyhow::Result<Vec<Option<Word>>>;
    // Reads
    async fn get_intent(&self, address: &IntentAddress) -> anyhow::Result<Option<Intent>>;
    async fn get_intent_set(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<Signed<Vec<Intent>>>>;
    /// List all intents. This will paginate the results. The page is 0-indexed.
    /// A time range can optionally be provided to filter the results.
    /// The time is duration since the Unix epoch.
    async fn list_intent_sets(
        &self,
        time_range: impl Into<Option<Range<Duration>>>,
        page: impl Into<Option<usize>>,
    ) -> anyhow::Result<Vec<Vec<Intent>>>;
    async fn list_solutions_pool(&self) -> anyhow::Result<Vec<Signed<Solution>>>;
    async fn list_winning_batches(
        &self,
        time_range: impl Into<Option<Range<Duration>>>,
        page: impl Into<Option<usize>>,
    ) -> anyhow::Result<Vec<Batch>>;
    async fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> anyhow::Result<Option<Word>>;
    async fn query_state_range(
        &self,
        address: &ContentAddress,
        keys: &KeyRange,
    ) -> anyhow::Result<Vec<Option<Word>>>;
    async fn get_storage_layout(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<StorageLayout>>;
}
