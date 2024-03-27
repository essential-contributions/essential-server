use memory_storage::MemoryStorage;

use super::*;

#[tokio::test]
async fn test_can_query() {
    let storage = MemoryStorage::new();
    let address = ContentAddress([0; 32]);
    let key = [0; 4];
    let value = Some(1);
    storage.update_state(&address, &key, value).await.unwrap();

    let storage = TransactionStorage::new(storage);

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

#[tokio::test]
async fn test_update_batch() {
    let storage = MemoryStorage::new();
    let storage = TransactionStorage::new(storage);
    storage
        .storage
        .update_state(&ContentAddress([0; 32]), &[0; 4], Some(0))
        .await
        .unwrap();
    storage
        .update_state(&ContentAddress([1; 32]), &[1; 4], Some(1))
        .await
        .unwrap();
    let updates = (0..10).map(|i| (ContentAddress([i as u8; 32]), [i as i64; 4], Some(i as i64)));
    let r = storage.update_state_batch(updates).await.unwrap();
    assert_eq!(
        r,
        vec![
            Some(0),
            Some(1),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None
        ]
    );
    storage.commit().await.unwrap();
    let r = storage
        .query_state(&ContentAddress([8; 32]), &[8; 4])
        .await
        .unwrap();
    assert_eq!(r, Some(8));
}
