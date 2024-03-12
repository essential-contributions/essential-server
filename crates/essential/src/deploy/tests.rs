use memory_storage::MemoryStorage;

use crate::test_utils::{empty_intent, sign};

use super::*;

#[tokio::test]
async fn test_deploy() {
    let storage = MemoryStorage::default();
    let intent = empty_intent();
    let intent = sign(vec![intent]);
    let result = deploy(&storage, intent.clone()).await.unwrap();
    let result = storage.get_intent(&result).await.unwrap();
    assert_eq!(result, Some(empty_intent()));
}
