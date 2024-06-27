use crate::test_utils::deploy_predicate;
use essential_storage::Storage;
use essential_types::predicate::Predicate;
use test_utils::empty::Empty;

#[tokio::test]
async fn test_deploy() {
    let predicate = Predicate::empty();
    let (address, storage) = deploy_predicate(predicate.clone()).await;
    let result = storage.get_predicate(&address).await.unwrap();
    assert_eq!(result, Some(predicate));
}
