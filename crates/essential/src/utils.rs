use crate::deploy::deploy;
use essential_types::{intent::Intent, ContentAddress, IntentAddress};
use memory_storage::MemoryStorage;
use storage::Storage;
use test_utils::{empty::Empty, sign_with_random_keypair};

pub async fn deploy_intent_with_storage<S: Storage>(storage: &S, intent: Intent) -> IntentAddress {
    let intent_hash = ContentAddress(utils::hash(&intent));
    let intent = sign_with_random_keypair(vec![intent]);
    let result = deploy(storage, intent).await.unwrap();
    IntentAddress {
        set: result,
        intent: intent_hash,
    }
}

pub async fn deploy_intent(intent: Intent) -> (IntentAddress, MemoryStorage) {
    let storage = MemoryStorage::default();
    (deploy_intent_with_storage(&storage, intent).await, storage)
}

pub async fn deploy_empty_intent() -> (IntentAddress, MemoryStorage) {
    deploy_intent(Intent::empty()).await
}

pub fn solution_with_data() {
    todo!()
}
