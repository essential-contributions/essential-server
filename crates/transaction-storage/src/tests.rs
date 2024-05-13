use essential_hash::hash;
use essential_memory_storage::MemoryStorage;
use essential_storage::{QueryState, Storage};
use essential_types::{intent::Intent, StorageLayout};
use test_utils::{empty::Empty, sign_with_random_keypair};

use super::*;

#[tokio::test]
async fn test_can_query() {
    let storage = MemoryStorage::new();
    let intent = Intent::empty();
    let address = ContentAddress(hash(&vec![intent.clone()]));
    let signed = sign_with_random_keypair(vec![intent]);
    let key = [0; 4];
    let value = Some(1);
    storage
        .insert_intent_set(StorageLayout {}, signed)
        .await
        .unwrap();
    storage.update_state(&address, &key, value).await.unwrap();

    let mut storage = storage.transaction();

    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, Some(1));

    let r = storage.update_state(&address, &key, Some(2)).await.unwrap();
    assert_eq!(r, Some(1));

    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, Some(2));

    let r = storage.storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, Some(1));

    let r = storage.update_state(&address, &key, None).await.unwrap();
    assert_eq!(r, Some(2));

    let r = storage.storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, Some(1));

    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, None);

    storage.commit().await.unwrap();

    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, None);

    let r = storage.storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, None);

    let r = storage.update_state(&address, &key, Some(3)).await.unwrap();
    assert_eq!(r, None);

    let r = storage.update_state(&address, &key, Some(4)).await.unwrap();
    assert_eq!(r, Some(3));

    storage.commit().await.unwrap();

    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, Some(4));

    let r = storage.storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, Some(4));

    storage.update_state(&address, &key, Some(5)).await.unwrap();
    storage.rollback();

    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, Some(4));

    storage.commit().await.unwrap();
    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, Some(4));

    let r = storage.storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, Some(4));
}
