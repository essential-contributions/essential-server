use std::{ops::Range, time::Duration};

use essential_types::{
    intent::Intent, solution::Solution, Eoa, Hash, IntentAddress, PersistentAddress,
};

use placeholder::{Batch, EoaPermit, Signed, StorageLayout};

pub trait Storage {
    // Updates
    async fn insert_intent_set(&self, intent: Signed<Vec<Intent>>) -> anyhow::Result<()>;
    async fn insert_permit_into_pool(&self, permit: Signed<EoaPermit>) -> anyhow::Result<()>;
    async fn insert_solution_into_pool(&self, solution: Signed<Solution>) -> anyhow::Result<()>;
    async fn move_solutions_to_solved(&self, solutions: &[Hash]) -> anyhow::Result<()>;
    async fn update_state(
        &self,
        address: &IntentAddress,
        key: &[u8],
        value: Option<Vec<u8>>,
    ) -> anyhow::Result<Option<Vec<u8>>>;
    async fn update_eoa_state(
        &self,
        address: &Eoa,
        key: &[u8],
        value: Option<Vec<u8>>,
    ) -> anyhow::Result<Option<Vec<u8>>>;
    // Reads
    async fn get_intent(&self, address: &PersistentAddress) -> anyhow::Result<Option<Intent>>;
    async fn get_intent_set(
        &self,
        address: &IntentAddress,
    ) -> anyhow::Result<Option<Signed<Vec<Intent>>>>;
    /// List all intents. This will paginate the results. The page is 0-indexed.
    /// A time range can optionally be provided to filter the results.
    /// The time is duration since the Unix epoch.
    async fn list_intents(
        &self,
        time_range: impl Into<Option<Range<Duration>>>,
        page: impl Into<Option<usize>>,
    ) -> anyhow::Result<Vec<Intent>>;
    async fn list_solutions_pool(&self) -> anyhow::Result<Vec<Signed<Solution>>>;
    async fn list_permits_pool(&self) -> anyhow::Result<Vec<Signed<EoaPermit>>>;
    async fn list_winning_batches(
        &self,
        time_range: impl Into<Option<Range<Duration>>>,
        page: impl Into<Option<usize>>,
    ) -> anyhow::Result<Vec<Batch>>;
    async fn query_state(&self, address: &IntentAddress, key: &[u8]) -> anyhow::Result<Vec<u8>>;
    async fn query_eoa_state(&self, address: &Eoa, key: &[u8]) -> anyhow::Result<Vec<u8>>;
    async fn get_storage_layout(&self, address: &IntentAddress) -> anyhow::Result<StorageLayout>;
}
