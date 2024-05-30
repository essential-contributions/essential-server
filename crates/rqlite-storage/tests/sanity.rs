use essential_hash::hash;
use essential_rqlite_storage::RqliteStorage;
use essential_storage::{QueryState, StateStorage, Storage};
use essential_types::{
    intent::Intent, solution::Solution, ContentAddress, IntentAddress, StorageLayout,
};
use std::vec;
use test_utils::{empty::Empty, intent_with_salt, sign_intent_set_with_random_keypair};

#[tokio::test]
#[ignore]
async fn test_create() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let storage_layout = StorageLayout;
    let intent = sign_intent_set_with_random_keypair(vec![Intent::empty()]);
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
    let intent = sign_intent_set_with_random_keypair(vec![Intent::empty()]);
    storage
        .insert_intent_set(storage_layout, intent)
        .await
        .unwrap();
    let address = ContentAddress(hash(&vec![Intent::empty()]));
    let key = vec![0; 4];
    let v = storage.update_state(&address, &key, vec![1]).await.unwrap();
    assert!(v.is_empty());
    let v = storage.update_state(&address, &key, vec![2]).await.unwrap();
    assert_eq!(v, vec![1]);
    let v = storage.update_state(&address, &key, vec![]).await.unwrap();
    assert_eq!(v, vec![2]);
    let v = storage.update_state(&address, &key, vec![]).await.unwrap();
    assert!(v.is_empty());
    let v = storage.update_state(&address, &key, vec![1]).await.unwrap();
    assert!(v.is_empty());
    let v = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(v, vec![1]);

    let v = storage.query_state(&address, &vec![1; 14]).await.unwrap();
    assert!(v.is_empty());
    let v = storage
        .update_state(&address, &vec![1; 14], vec![3; 8])
        .await
        .unwrap();
    assert!(v.is_empty());
    let v = storage.query_state(&address, &vec![1; 14]).await.unwrap();
    assert_eq!(v, vec![3; 8]);
    let v = storage
        .update_state(&address, &vec![1; 14], vec![3; 2])
        .await
        .unwrap();
    assert_eq!(v, vec![3; 8]);
    let v = storage.query_state(&address, &vec![1; 14]).await.unwrap();
    assert_eq!(v, vec![3; 2]);
}

#[tokio::test]
#[ignore]
async fn test_update_state_batch() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let storage_layout = StorageLayout;
    let intent = sign_intent_set_with_random_keypair(vec![Intent::empty()]);
    storage
        .insert_intent_set(storage_layout.clone(), intent)
        .await
        .unwrap();
    let intent = sign_intent_set_with_random_keypair(vec![intent_with_salt(3)]);
    storage
        .insert_intent_set(storage_layout, intent)
        .await
        .unwrap();
    let address_0 = ContentAddress(hash(&vec![Intent::empty()]));
    let address_1 = ContentAddress(hash(&vec![intent_with_salt(3)]));
    let key = vec![0; 4];
    let v = storage
        .update_state(&address_0, &key, vec![1])
        .await
        .unwrap();
    assert!(v.is_empty());
    let v = storage
        .update_state(&address_1, &vec![1; 4], vec![2])
        .await
        .unwrap();
    assert!(v.is_empty());
    let updates = (0..10).map(|i| {
        let address = if i % 2 == 0 {
            address_0.clone()
        } else {
            address_1.clone()
        };
        (address, vec![i; 4], vec![i])
    });
    let v = storage.update_state_batch(updates).await.unwrap();
    assert_eq!(
        v,
        vec![
            vec![1],
            vec![2],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![]
        ]
    );

    let v = storage.query_state(&address_0, &vec![8; 4]).await.unwrap();
    assert_eq!(v, vec![8]);
}

#[tokio::test]
#[ignore]
async fn test_insert_intent_set() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let storage_layout = StorageLayout;
    let intent_0 = sign_intent_set_with_random_keypair(vec![Intent::empty()]);
    storage
        .insert_intent_set(storage_layout.clone(), intent_0.clone())
        .await
        .unwrap();
    let intent_1 =
        sign_intent_set_with_random_keypair(vec![intent_with_salt(1), intent_with_salt(2)]);
    storage
        .insert_intent_set(storage_layout, intent_1)
        .await
        .unwrap();
    let intent_sets = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(
        intent_sets,
        vec![
            vec![Intent::empty()],
            vec![intent_with_salt(1), intent_with_salt(2)]
        ]
    );
    let intent_set = storage
        .get_intent_set(&ContentAddress(hash(&vec![Intent::empty()])))
        .await
        .unwrap();
    assert_eq!(intent_set, Some(intent_0));

    let address = IntentAddress {
        set: ContentAddress(hash(&vec![Intent::empty()])),
        intent: ContentAddress(hash(&Intent::empty())),
    };
    let intent = storage.get_intent(&address).await.unwrap();

    assert_eq!(intent, Some(Intent::empty()));
}

#[tokio::test]
#[ignore]
async fn test_insert_solution_into_pool() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let solution = Solution::empty();
    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();
    let solutions = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(solutions.len(), 1);
    assert_eq!(hash(&solutions[0].data), hash(&Solution::empty()));
    storage
        .move_solutions_to_solved(&[hash(&Solution::empty())])
        .await
        .unwrap();
    let solutions = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(solutions.len(), 0);
    let batches = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(hash(&batches[0].batch.solutions), hash(&vec![solution]));
}
