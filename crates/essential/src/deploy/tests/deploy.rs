use crate::test_utils::{deploy_intent, sanity_intent};
use storage::Storage;

#[tokio::test]
async fn test_deploy() {
    let intent = sanity_intent();
    let (address, storage) = deploy_intent(intent.clone()).await;
    let result = storage.get_intent(&address).await.unwrap();
    assert_eq!(result, Some(intent));
}
