use memory_storage::MemoryStorage;
use storage::Storage;

use test_utils::sign;

use super::*;

#[tokio::test]
async fn test_submit_permit() {
    let storage = MemoryStorage::default();
    let permit: EoaPermit = ();
    let permit = sign(permit);
    submit_permit(&storage, permit.clone()).await.unwrap();
    let result = storage.list_permits_pool().await.unwrap();
    assert_eq!(result, vec![permit]);
}
