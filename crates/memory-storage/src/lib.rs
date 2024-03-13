use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
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
    intent_time_index: BTreeMap<Duration, IntentAddress>,
    permit_pool: Vec<Signed<EoaPermit>>,
    solution_pool: HashMap<Hash, Signed<Solution>>,
    solved: BTreeMap<Duration, Batch>,
    state: HashMap<IntentAddress, BTreeMap<Key, Word>>,
    eoa_state: HashMap<Eoa, BTreeMap<Key, Word>>,
}

struct IntentSet {
    storage_layout: StorageLayout,
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
    async fn insert_intent_set(
        &self,
        storage_layout: StorageLayout,
        intent: Signed<Vec<Intent>>,
    ) -> anyhow::Result<()> {
        let Signed { data, signature } = intent;
        let hash = IntentAddress(utils::hash(&data));
        let order: Vec<_> = data.iter().map(|i| IntentAddress(utils::hash(i))).collect();
        let map = order.iter().cloned().zip(data.into_iter()).collect();
        let set = IntentSet {
            storage_layout,
            order,
            data: map,
            signature,
        };
        let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        self.inner.apply(|i| {
            i.intents.insert(hash.clone(), set);
            i.intent_time_index.insert(time, hash);
        });
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
        key: &Key,
        value: Option<Word>,
    ) -> anyhow::Result<Option<Word>> {
        let v = self.inner.apply(|i| {
            let map = i.state.entry(address.clone()).or_default();
            match value {
                None => map.remove(key),
                Some(value) => map.insert(*key, value),
            }
        });
        Ok(v)
    }

    async fn update_state_range(
        &self,
        address: &IntentAddress,
        key: &essential_types::KeyRange,
        value: Option<Word>,
    ) -> anyhow::Result<Vec<Option<Word>>> {
        todo!()
    }

    async fn update_eoa_state(
        &self,
        address: &Eoa,
        key: &Key,
        value: Option<Word>,
    ) -> anyhow::Result<Option<Word>> {
        let v = self.inner.apply(|i| {
            let map = i.eoa_state.entry(*address).or_default();
            match value {
                None => map.remove(key),
                Some(value) => map.insert(*key, value),
            }
        });
        Ok(v)
    }

    async fn update_eoa_state_range(
        &self,
        address: &Eoa,
        key: &essential_types::KeyRange,
        value: Option<Word>,
    ) -> anyhow::Result<Vec<Option<Word>>> {
        todo!()
    }

    async fn get_intent(&self, address: &PersistentAddress) -> anyhow::Result<Option<Intent>> {
        let v = self.inner.apply(|i| {
            let set = i.intents.get(&address.set)?;
            let intent = set.data.get(&address.intent)?;
            Some(intent.clone())
        });
        Ok(v)
    }

    async fn get_intent_set(
        &self,
        address: &IntentAddress,
    ) -> anyhow::Result<Option<Signed<Vec<Intent>>>> {
        let v = self.inner.apply(|i| {
            let set = i.intents.get(address)?;
            let data = set
                .order
                .iter()
                .map(|i| set.data.get(i).cloned())
                .collect::<Option<Vec<_>>>()?;
            Some(Signed {
                data,
                signature: set.signature,
            })
        });
        Ok(v)
    }

    async fn list_intents(
        &self,
        time_range: impl Into<Option<std::ops::Range<std::time::Duration>>>,
        page: impl Into<Option<usize>>,
    ) -> anyhow::Result<Vec<Intent>> {
        todo!()
    }

    async fn list_solutions_pool(&self) -> anyhow::Result<Vec<Signed<Solution>>> {
        Ok(self
            .inner
            .apply(|i| i.solution_pool.values().cloned().collect()))
    }

    async fn list_permits_pool(&self) -> anyhow::Result<Vec<Signed<EoaPermit>>> {
        Ok(self.inner.apply(|i| i.permit_pool.clone()))
    }

    async fn list_winning_batches(
        &self,
        time_range: impl Into<Option<std::ops::Range<std::time::Duration>>>,
        page: impl Into<Option<usize>>,
    ) -> anyhow::Result<Vec<Batch>> {
        todo!()
    }

    async fn get_storage_layout(
        &self,
        address: &IntentAddress,
    ) -> anyhow::Result<Option<StorageLayout>> {
        let v = self.inner.apply(|i| {
            let set = i.intents.get(address)?;
            Some(set.storage_layout)
        });
        Ok(v)
    }

    async fn query_state(
        &self,
        address: &IntentAddress,
        key: &Key,
    ) -> anyhow::Result<Option<Word>> {
        let v = self.inner.apply(|i| {
            let map = i.state.get(address)?;
            let v = map.get(key)?;
            Some(*v)
        });
        Ok(v)
    }

    async fn query_state_range(
        &self,
        address: &IntentAddress,
        key: &essential_types::KeyRange,
    ) -> anyhow::Result<Vec<Option<Word>>> {
        todo!()
    }

    async fn query_eoa_state(&self, address: &Eoa, key: &Key) -> anyhow::Result<Option<Word>> {
        let v = self.inner.apply(|i| {
            let map = i.eoa_state.get(address)?;
            let v = map.get(key)?;
            Some(*v)
        });
        Ok(v)
    }

    async fn query_eoa_state_range(
        &self,
        address: &Eoa,
        key: &essential_types::KeyRange,
    ) -> anyhow::Result<Vec<Option<Word>>> {
        todo!()
    }
}
