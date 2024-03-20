use std::{ops::Range, time::Duration};

use essential_types::{
    intent::Intent, solution::Solution, ContentAddress, Hash, IntentAddress, Key, KeyRange, Word,
};
use placeholder::{Batch, EoaPermit, Signed, StorageLayout};
use storage::Storage;

mod deploy;
mod permit;
mod run;
mod solution;

#[derive(Clone)]
pub struct Essential<S>
where
    S: Storage + Clone,
{
    storage: S,
}

impl<S> Essential<S>
where
    S: Storage + Clone,
{
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        run::run(&self.storage).await
    }

    pub async fn deploy_intent_set(
        &self,
        intents: Signed<Vec<Intent>>,
    ) -> anyhow::Result<ContentAddress> {
        deploy::deploy(&self.storage, intents).await
    }

    pub async fn check_solution(&self, solution: Solution) -> anyhow::Result<f64> {
        solution::check_solution(&self.storage, solution).await
    }

    pub async fn submit_solution(&self, solution: Signed<Solution>) -> anyhow::Result<Hash> {
        solution::submit_solution(&self.storage, solution).await
    }

    pub async fn submit_permit(&self, permit: Signed<EoaPermit>) -> anyhow::Result<()> {
        permit::submit_permit(&self.storage, permit).await
    }

    pub async fn get_intent(&self, address: &IntentAddress) -> anyhow::Result<Option<Intent>> {
        self.storage.get_intent(address).await
    }

    pub async fn get_intent_set(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<Signed<Vec<Intent>>>> {
        self.storage.get_intent_set(address).await
    }

    pub async fn list_intent_sets(
        &self,
        time_range: impl Into<Option<Range<Duration>>>,
        page: impl Into<Option<usize>>,
    ) -> anyhow::Result<Vec<Vec<Intent>>> {
        self.storage.list_intent_sets(time_range, page).await
    }

    pub async fn list_solutions_pool(&self) -> anyhow::Result<Vec<Signed<Solution>>> {
        self.storage.list_solutions_pool().await
    }

    pub async fn list_permits_pool(&self) -> anyhow::Result<Vec<Signed<EoaPermit>>> {
        self.storage.list_permits_pool().await
    }

    pub async fn list_winning_batches(
        &self,
        time_range: impl Into<Option<Range<Duration>>>,
        page: impl Into<Option<usize>>,
    ) -> anyhow::Result<Vec<Batch>> {
        self.storage.list_winning_batches(time_range, page).await
    }

    pub async fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> anyhow::Result<Option<Word>> {
        self.storage.query_state(address, key).await
    }

    pub async fn query_state_range(
        &self,
        address: &ContentAddress,
        key: &KeyRange,
    ) -> anyhow::Result<Vec<Option<Word>>> {
        self.storage.query_state_range(address, key).await
    }

    pub async fn get_storage_layout(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<StorageLayout>> {
        self.storage.get_storage_layout(address).await
    }
}
