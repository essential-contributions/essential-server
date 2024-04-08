use essential_types::{
    intent::Intent,
    solution::{PartialSolution, Solution},
    ContentAddress, IntentAddress, StorageLayout,
};
use rqlite_storage::RqliteStorage;
use std::vec;
use storage::{StateStorage, Storage};
use test_utils::{empty::Empty, instantiate::Instantiate, sign_with_random_keypair};
use utils::hash;

#[tokio::test]
#[ignore]
async fn test_create() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let storage_layout = StorageLayout;
    let intent = sign_with_random_keypair(vec![Intent::empty()]);
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
    let intent = sign_with_random_keypair(vec![Intent::empty()]);
    storage
        .insert_intent_set(storage_layout, intent)
        .await
        .unwrap();
    let address = ContentAddress(hash(&vec![Intent::empty()]));
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
async fn test_update_state_batch() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let storage_layout = StorageLayout;
    let intent = sign_with_random_keypair(vec![Intent::empty()]);
    storage
        .insert_intent_set(storage_layout.clone(), intent)
        .await
        .unwrap();
    let intent = sign_with_random_keypair(vec![Intent::with_decision_variables(3)]);
    storage
        .insert_intent_set(storage_layout, intent)
        .await
        .unwrap();
    let address_0 = ContentAddress(hash(&vec![Intent::empty()]));
    let address_1 = ContentAddress(hash(&vec![Intent::with_decision_variables(3)]));
    let key = [0; 4];
    let v = storage
        .update_state(&address_0, &key, Some(1))
        .await
        .unwrap();
    assert_eq!(v, None);
    let v = storage
        .update_state(&address_1, &[1; 4], Some(2))
        .await
        .unwrap();
    assert_eq!(v, None);
    let updates = (0..10).map(|i| {
        let address = if i % 2 == 0 {
            address_0.clone()
        } else {
            address_1.clone()
        };
        (address, [i; 4], Some(i))
    });
    let v = storage.update_state_batch(updates).await.unwrap();
    assert_eq!(
        v,
        vec![
            Some(1),
            Some(2),
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

    let v = storage.query_state(&address_0, &[8; 4]).await.unwrap();
    assert_eq!(v, Some(8));
}

#[tokio::test]
#[ignore]
async fn test_insert_intent_set() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let storage_layout = StorageLayout;
    let intent_0 = sign_with_random_keypair(vec![Intent::empty()]);
    storage
        .insert_intent_set(storage_layout.clone(), intent_0.clone())
        .await
        .unwrap();
    let intent_1 = sign_with_random_keypair(vec![
        Intent::with_decision_variables(1),
        Intent::with_decision_variables(2),
    ]);
    storage
        .insert_intent_set(storage_layout, intent_1)
        .await
        .unwrap();
    let intent_sets = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(
        intent_sets,
        vec![
            vec![Intent::empty()],
            vec![
                Intent::with_decision_variables(1),
                Intent::with_decision_variables(2)
            ]
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
    let solution = sign_with_random_keypair(Solution::empty());
    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();
    let solutions = storage.list_solutions_pool().await.unwrap();
    assert_eq!(solutions.len(), 1);
    assert_eq!(hash(&solutions[0].data), hash(&Solution::empty()));
    storage
        .move_solutions_to_solved(&[hash(&Solution::empty())])
        .await
        .unwrap();
    let solutions = storage.list_solutions_pool().await.unwrap();
    assert_eq!(solutions.len(), 0);
    let batches = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(hash(&batches[0].batch.solutions), hash(&vec![solution]));
}

#[tokio::test]
#[ignore]
async fn test_insert_partial_solutions() {
    let storage = RqliteStorage::new("http://localhost:4001").await.unwrap();
    let solution = sign_with_random_keypair(PartialSolution::empty());
    storage
        .insert_partial_solution_into_pool(solution.clone())
        .await
        .unwrap();
    let solutions = storage.list_partial_solutions_pool().await.unwrap();
    assert_eq!(solutions.len(), 1);
    assert_eq!(hash(&solutions[0].data), hash(&PartialSolution::empty()));
    let solved = storage
        .is_partial_solution_solved(&ContentAddress(hash(&PartialSolution::empty())))
        .await
        .unwrap()
        .unwrap();
    assert!(!solved);
    storage
        .move_partial_solutions_to_solved(&[hash(&PartialSolution::empty())])
        .await
        .unwrap();
    let solutions = storage.list_partial_solutions_pool().await.unwrap();
    assert_eq!(solutions.len(), 0);
    let solved = storage
        .is_partial_solution_solved(&ContentAddress(hash(&PartialSolution::empty())))
        .await
        .unwrap()
        .unwrap();
    assert!(solved);
    let result = storage
        .get_partial_solution(&ContentAddress(hash(&PartialSolution::empty())))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(hash(&result), hash(&solution));
}
