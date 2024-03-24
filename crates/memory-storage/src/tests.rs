use test_utils::{empty_intent, random_keypair};
use utils::sign;

use super::*;

#[tokio::test]
#[ignore]
async fn test_insert_intent_set() {
    let storage = MemoryStorage::new();
    let storage_layout = StorageLayout {};
    let intent = sign(vec![empty_intent()], random_keypair().0);
    storage
        .insert_intent_set(storage_layout, intent)
        .await
        .unwrap();
    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(result, vec![vec![empty_intent()]]);
}
