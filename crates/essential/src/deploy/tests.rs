use super::*;
use essential_types::IntentAddress;
use memory_storage::MemoryStorage;
use test_utils::{empty::Empty, sign_with_random_keypair};

#[tokio::test]
#[ignore]
async fn test_deploy() {
    let storage = MemoryStorage::default();
    let intent = Intent::empty();
    let intent_hash = ContentAddress(utils::hash(&intent));
    let intent = sign_with_random_keypair(vec![intent]);
    let result = deploy(&storage, intent.clone()).await.unwrap();
    let address = IntentAddress {
        set: result,
        intent: intent_hash,
    };

    let result = storage.get_intent(&address).await.unwrap();
    assert_eq!(result, Some(Intent::empty()));
}
