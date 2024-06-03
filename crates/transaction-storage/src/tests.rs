use essential_memory_storage::MemoryStorage;
use essential_storage::{QueryState, Storage};
use essential_types::intent::Intent;
use test_utils::{empty::Empty, sign_intent_set_with_random_keypair};

use super::*;

#[tokio::test]
async fn test_can_query() {
    let storage = MemoryStorage::new();
    let intent = Intent::empty();
    let address = essential_hash::intent_set_addr::from_intents(&vec![intent.clone()]);
    let signed = sign_intent_set_with_random_keypair(vec![intent]);
    let key = vec![0; 4];
    let value = vec![1];
    storage.insert_intent_set(signed).await.unwrap();
    storage.update_state(&address, &key, value).await.unwrap();

    let mut storage = storage.transaction();

    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, vec![1]);

    let r = storage
        .update_state(&address, key.clone(), vec![2])
        .await
        .unwrap();
    assert_eq!(r, vec![1]);

    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, vec![2]);

    let r = storage.storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, vec![1]);

    let r = storage
        .update_state(&address, key.clone(), vec![])
        .await
        .unwrap();
    assert_eq!(r, vec![2]);

    let r = storage.storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, vec![1]);

    let r = storage.query_state(&address, &key).await.unwrap();
    assert!(r.is_empty());

    storage.commit().await.unwrap();

    let r = storage.query_state(&address, &key).await.unwrap();
    assert!(r.is_empty());

    let r = storage.storage.query_state(&address, &key).await.unwrap();
    assert!(r.is_empty());

    let r = storage
        .update_state(&address, key.clone(), vec![3])
        .await
        .unwrap();
    assert!(r.is_empty());

    let r = storage
        .update_state(&address, key.clone(), vec![4])
        .await
        .unwrap();
    assert_eq!(r, vec![3]);

    storage.commit().await.unwrap();

    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, vec![4]);

    let r = storage.storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, vec![4]);

    storage
        .update_state(&address, key.clone(), vec![5])
        .await
        .unwrap();
    storage.rollback();

    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, vec![4]);

    storage.commit().await.unwrap();
    let r = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, vec![4]);

    let r = storage.storage.query_state(&address, &key).await.unwrap();
    assert_eq!(r, vec![4]);
}
