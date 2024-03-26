use essential_types::{ContentAddress, IntentAddress, StorageLayout};
use rqlite_storage::RqliteStorage;
use std::vec;
use storage::Storage;
use test_utils::{empty_intent, empty_solution, intent_with_vars, random_keypair};
use utils::{hash, sign};

#[tokio::test]
#[ignore]
async fn test_create() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let storage_layout = StorageLayout;
    let intent = sign(vec![empty_intent()], random_keypair().0);
    storage
        .insert_intent_set(storage_layout, intent)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore]
async fn test_update_state() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let storage_layout = StorageLayout;
    let intent = sign(vec![empty_intent()], random_keypair().0);
    storage
        .insert_intent_set(storage_layout, intent)
        .await
        .unwrap();
    let address = ContentAddress(hash(&vec![empty_intent()]));
    let key = [0; 4];
    let v = storage.update_state(&address, &key, Some(1)).await.unwrap();
    assert_eq!(v, None);
    let v = storage.update_state(&address, &key, Some(2)).await.unwrap();
    assert_eq!(v, Some(1));
    let v = storage.update_state(&address, &key, None).await.unwrap();
    assert_eq!(v, Some(2));
    let v = storage.update_state(&address, &key, None).await.unwrap();
    assert_eq!(v, None);
    let v = storage.update_state(&address, &key, Some(1)).await.unwrap();
    assert_eq!(v, None);
    let v = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(v, Some(1));
}

#[tokio::test]
#[ignore]
async fn test_insert_intent_set() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let storage_layout = StorageLayout;
    let intent = sign(vec![empty_intent()], random_keypair().0);
    storage
        .insert_intent_set(storage_layout.clone(), intent)
        .await
        .unwrap();
    let intent = sign(
        vec![intent_with_vars(1), intent_with_vars(2)],
        random_keypair().0,
    );
    storage
        .insert_intent_set(storage_layout, intent)
        .await
        .unwrap();
    let intent_sets = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(
        intent_sets,
        vec![
            vec![empty_intent()],
            vec![intent_with_vars(1), intent_with_vars(2)]
        ]
    );
    let intent_set = storage
        .get_intent_set(&ContentAddress(hash(&vec![empty_intent()])))
        .await
        .unwrap();
    assert_eq!(
        intent_set,
        Some(sign(vec![empty_intent()], random_keypair().0))
    );

    let address = IntentAddress {
        set: ContentAddress(hash(&vec![empty_intent()])),
        intent: ContentAddress(hash(&empty_intent())),
    };
    let intent = storage.get_intent(&address).await.unwrap();

    assert_eq!(intent, Some(empty_intent()));
}

#[tokio::test]
#[ignore]
async fn test_insert_solution_into_pool() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let solution = sign(empty_solution(), random_keypair().0);
    storage.insert_solution_into_pool(solution).await.unwrap();
    let solutions = storage.list_solutions_pool().await.unwrap();
    assert_eq!(solutions.len(), 1);
    assert_eq!(hash(&solutions[0].data), hash(&empty_solution()));
    storage
        .move_solutions_to_solved(&[hash(&empty_solution())])
        .await
        .unwrap();
    let solutions = storage.list_solutions_pool().await.unwrap();
    assert_eq!(solutions.len(), 0);
    let batches = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(
        hash(&batches[0].batch.solutions),
        hash(&vec![sign(empty_solution(), random_keypair().0)])
    );
}
