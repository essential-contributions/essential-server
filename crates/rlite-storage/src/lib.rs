use std::{collections::HashMap, sync::Arc};

use essential_types::{intent::Intent, IntentAddress, PersistentAddress};
use placeholder::{Batch, EoaPermit, Signed, StorageLayout};
use storage::Storage;
use utils::Lock;

#[derive(Clone)]
pub struct RliteStorage {
    inner: Arc<Lock<Inner>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
struct Inner {
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Lock::new(Inner::default())),
        }
    }
}

impl Storage for MemoryStorage {
    async fn insert_intent_set(&self, intent: Signed<Vec<Intent>>) -> anyhow::Result<()> {
        todo!()
    }

    async fn insert_permit_into_pool(
        &self,
        permit: Signed<EoaPermit>,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn insert_solution_into_pool(
        &self,
        solution: Signed<essential_types::solution::Solution>,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn move_solutions_to_solved(
        &self,
        solutions: &[essential_types::Hash],
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn update_state(
        &self,
        address: &IntentAddress,
        key: &[u8],
        value: Option<Vec<u8>>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        todo!()
    }

    async fn update_eoa_state(
        &self,
        address: &essential_types::Eoa,
        key: &[u8],
        value: Option<Vec<u8>>,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        todo!()
    }

    async fn get_intent(&self, address: &PersistentAddress) -> anyhow::Result<Option<Intent>> {
        todo!()
    }

    async fn get_intent_set(
        &self,
        address: &IntentAddress,
    ) -> anyhow::Result<Option<Signed<Vec<Intent>>>> {
        todo!()
    }

    async fn list_intents(
        &self,
        time_range: impl Into<Option<std::ops::Range<std::time::Duration>>>,
        page: impl Into<Option<usize>>,
    ) -> anyhow::Result<Vec<Intent>> {
        todo!()
    }

    async fn list_solutions_pool(
        &self,
    ) -> anyhow::Result<Vec<Signed<essential_types::solution::Solution>>> {
        todo!()
    }

    async fn list_permits_pool(
        &self,
    ) -> anyhow::Result<Vec<Signed<EoaPermit>>> {
        todo!()
    }

    async fn list_winning_batches(
        &self,
        time_range: impl Into<Option<std::ops::Range<std::time::Duration>>>,
        page: impl Into<Option<usize>>,
    ) -> anyhow::Result<Vec<Batch>> {
        todo!()
    }

    async fn query_state(&self, address: &IntentAddress, key: &[u8]) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    async fn query_eoa_state(
        &self,
        address: &essential_types::Eoa,
        key: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    async fn get_storage_layout(
        &self,
        address: &IntentAddress,
    ) -> anyhow::Result<StorageLayout> {
        todo!()
    }
}
