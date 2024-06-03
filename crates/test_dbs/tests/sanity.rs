use common::create_test;
use essential_hash::hash;
use essential_memory_storage::MemoryStorage;
use essential_storage::Storage;
use essential_types::{intent::Intent, solution::Solution, IntentAddress, StorageLayout};
use std::vec;
use test_utils::{empty::Empty, intent_with_salt, sign_intent_set_with_random_keypair};

mod common;
#[cfg(feature = "rqlite")]
mod rqlite;

create_test!(update_state);

async fn update_state<S: Storage>(storage: S) {
    let storage_layout = StorageLayout;
    let intent = sign_intent_set_with_random_keypair(vec![Intent::empty()]);
    storage
        .insert_intent_set(storage_layout, intent)
        .await
        .unwrap();
    let address = essential_hash::intent_set_addr::from_intents(&vec![Intent::empty()]);
    let key = vec![0; 4];
    let v = storage.update_state(&address, &key, vec![1]).await.unwrap();
    assert!(v.is_empty());
    let v = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(v, vec![1]);
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

create_test!(update_state_batch);

async fn update_state_batch<S: Storage>(storage: S) {
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
    let address_0 = essential_hash::intent_set_addr::from_intents(&vec![Intent::empty()]);
    let address_1 = essential_hash::intent_set_addr::from_intents(&vec![intent_with_salt(3)]);
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

create_test!(insert_intent_set);

async fn insert_intent_set<S: Storage>(storage: S) {
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
    let mut s = vec![intent_with_salt(1), intent_with_salt(2)];
    s.sort_by_key(essential_hash::content_addr);
    assert_eq!(intent_sets, vec![vec![Intent::empty()], s]);
    let address = essential_hash::intent_set_addr::from_intents(&vec![Intent::empty()]);
    let intent_set = storage.get_intent_set(&address).await.unwrap();
    assert_eq!(intent_set, Some(intent_0));

    let address = IntentAddress {
        set: essential_hash::intent_set_addr::from_intents(&vec![Intent::empty()]),
        intent: essential_hash::content_addr(&Intent::empty()),
    };
    let intent = storage.get_intent(&address).await.unwrap();

    assert_eq!(intent, Some(Intent::empty()));
}

create_test!(insert_solution_into_pool);

async fn insert_solution_into_pool<S: Storage>(storage: S) {
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
