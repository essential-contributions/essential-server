use std::{collections::HashMap, sync::Arc};

use essential_types::{intent::Intent, PersistentAddress};

use crate::{signed::Signed, storage::Storage, utils::Lock};

#[derive(Clone)]
pub struct TestStorage {
    inner: Arc<Lock<Inner>>,
}

impl Default for TestStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
struct Inner {
    intents: HashMap<PersistentAddress, Signed<Vec<Intent>>>,
}

impl TestStorage {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Lock::new(Inner::default())),
        }
    }
}

impl Storage for TestStorage {
    async fn insert_intent_set(&self, intent: Signed<Vec<Intent>>) -> anyhow::Result<()> {
        todo!()
    }

    async fn get_intent(&self, address: &PersistentAddress) -> anyhow::Result<Option<Intent>> {
        todo!()
    }
}
