use super::*;
use crate::tests::deploy_intent;
use memory_storage::MemoryStorage;
use test_utils::empty::Empty;

#[tokio::test]
async fn test_deploy() {
    let storage = MemoryStorage::default();
    let intent = Intent::empty();
    let address = deploy_intent(&storage, intent).await;
    let result = storage.get_intent(&address).await.unwrap();
    assert_eq!(result, Some(Intent::empty()));
}
