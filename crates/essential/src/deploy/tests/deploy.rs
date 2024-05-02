use crate::test_utils::deploy_intent;
use essential_types::intent::Intent;
use storage::Storage;
use test_utils::empty::Empty;

#[tokio::test]
async fn test_deploy() {
    let intent = Intent::empty();
    let (address, storage) = deploy_intent(intent.clone()).await;
    let result = storage.get_intent(&address).await.unwrap();
    assert_eq!(result, Some(intent));
}
