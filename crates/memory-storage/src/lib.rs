use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};

use essential_types::{
    intent::Intent, solution::Solution, Eoa, Hash, IntentAddress, Key, PersistentAddress, Word,
};
use placeholder::{Batch, EoaPermit, Signature, Signed, StorageLayout};
use storage::Storage;
use utils::Lock;

#[derive(Clone)]
pub struct MemoryStorage {
    inner: Arc<Lock<Inner>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
struct Inner {
    intents: HashMap<IntentAddress, IntentSet>,
    permit_pool: Vec<Signed<EoaPermit>>,
    solution_pool: HashMap<Hash, Signed<Solution>>,
    solved: BTreeMap<Duration, Batch>,
    state: HashMap<IntentAddress, BTreeMap<Key, Word>>,
    eoa_state: HashMap<Eoa, BTreeMap<Key, Word>>,
    // TODO: Add other storage data.
}

struct IntentSet {
    order: Vec<IntentAddress>,
    data: HashMap<IntentAddress, Intent>,
    signature: Signature,
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
        let Signed { data, signature } = intent;
        let hash = IntentAddress(utils::hash(&data));
        let order: Vec<_> = data.iter().map(|i| IntentAddress(utils::hash(i))).collect();
        let map = order.iter().cloned().zip(data.into_iter()).collect();
        let set = IntentSet {
            order,
            data: map,
            signature,
        };
        self.inner.apply(|i| i.intents.insert(hash, set));
        Ok(())
    }

    async fn insert_permit_into_pool(&self, permit: Signed<EoaPermit>) -> anyhow::Result<()> {
        self.inner.apply(|i| i.permit_pool.push(permit));
        Ok(())
    }

    async fn insert_solution_into_pool(&self, solution: Signed<Solution>) -> anyhow::Result<()> {
        let hash = utils::hash(&solution.data);
        self.inner.apply(|i| i.solution_pool.insert(hash, solution));
        Ok(())
    }

    async fn move_solutions_to_solved(&self, solutions: &[Hash]) -> anyhow::Result<()> {
        self.inner.apply(|i| {
            let solutions = solutions
                .iter()
                .filter_map(|h| i.solution_pool.remove(h))
                .collect();
            let batch = Batch { solutions };
            i.solved.insert(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap(),
                batch,
            );
        });
        Ok(())
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
        address: &Eoa,
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

    async fn list_permits_pool(&self) -> anyhow::Result<Vec<Signed<EoaPermit>>> {
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

    async fn get_storage_layout(&self, address: &IntentAddress) -> anyhow::Result<StorageLayout> {
        todo!()
    }
}
