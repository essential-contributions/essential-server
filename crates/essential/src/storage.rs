use essential_types::{intent::Intent, PersistentAddress};

use crate::signed::Signed;

pub trait Storage {
    async fn insert_intent_set(&self, intent: Signed<Vec<Intent>>) -> anyhow::Result<()>;
    async fn get_intent(&self, address: &PersistentAddress) -> anyhow::Result<Option<Intent>>;
}
