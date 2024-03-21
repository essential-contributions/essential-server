use essential_types::IntentAddress;
use memory_storage::MemoryStorage;

use test_utils::{empty_intent, sign};

use super::*;

#[tokio::test]
#[ignore]
async fn test_deploy() {
    let storage = MemoryStorage::default();
    let intent = empty_intent();
    let intent_hash = ContentAddress(utils::hash(&intent));
    let intent = sign(vec![intent]);
    let result = deploy(&storage, intent.clone()).await.unwrap();
    let address = IntentAddress {
        set: result,
        intent: intent_hash,
    };

    let result = storage.get_intent(&address).await.unwrap();
    assert_eq!(result, Some(empty_intent()));
}
