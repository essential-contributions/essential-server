use crate::utils::deploy_empty_intent;
use essential_types::intent::Intent;
use storage::Storage;
use test_utils::empty::Empty;

#[tokio::test]
async fn test_deploy() {
    let (address, storage) = deploy_empty_intent().await;
    let result = storage.get_intent(&address).await.unwrap();
    assert_eq!(result, Some(Intent::empty()));
}
