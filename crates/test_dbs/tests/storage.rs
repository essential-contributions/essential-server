use common::create_test;
use essential_memory_storage::MemoryStorage;
use essential_storage::{failed_solution::SolutionFailReason, Storage};
use essential_types::{ContentAddress, IntentAddress, StorageLayout};
use pretty_assertions::assert_eq;
use test_utils::{
    empty::Empty, intent_with_salt, intent_with_salt_and_state,
    sign_intent_set_with_random_keypair, solution_with_all_inputs,
};

mod common;
#[cfg(feature = "rqlite")]
mod rqlite;

create_test!(insert_intent_set);

async fn insert_intent_set<S: Storage>(storage: S) {
    // Double insert is a idempotent
    let mut set = sign_intent_set_with_random_keypair(vec![
        intent_with_salt(0),
        intent_with_salt(1),
        intent_with_salt(2),
    ]);
    set.set.sort_by_key(essential_hash::content_addr);

    storage
        .insert_intent_set(StorageLayout {}, set.clone())
        .await
        .unwrap();

    storage
        .insert_intent_set(StorageLayout {}, set.clone())
        .await
        .unwrap();

    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(result, vec![set.set.clone()]);

    // Insert many sets
    let sets: Vec<_> = (0..10)
        .map(|i| {
            let mut set = vec![
                intent_with_salt(i),
                intent_with_salt(i + 1),
                intent_with_salt(i + 2),
            ];
            set.sort_by_key(essential_hash::content_addr);
            sign_intent_set_with_random_keypair(set)
        })
        .collect();

    let mut expected: Vec<_> = sets.iter().map(|s| s.set.clone()).collect();

    for set in &sets {
        storage
            .insert_intent_set(StorageLayout {}, set.clone())
            .await
            .unwrap();
    }

    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(result, expected);

    // Insert empty set
    storage
        .insert_intent_set(
            StorageLayout {},
            sign_intent_set_with_random_keypair(vec![]),
        )
        .await
        .unwrap();

    // Insert sets with storage
    let storage_sets: Vec<_> = (0..10)
        .map(|i| {
            let mut set = vec![
                intent_with_salt_and_state(i, i),
                intent_with_salt_and_state(i + 1, i + 1),
                intent_with_salt_and_state(i + 2, i + 2),
            ];
            set.sort_by_key(essential_hash::content_addr);
            sign_intent_set_with_random_keypair(set)
        })
        .collect();

    expected.extend(storage_sets.iter().map(|s| s.set.clone()));

    for set in &storage_sets {
        storage
            .insert_intent_set(StorageLayout {}, set.clone())
            .await
            .unwrap();
    }

    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(result, expected);
}

create_test!(insert_solution_into_pool);

async fn insert_solution_into_pool<S: Storage>(storage: S) {
    // Double insert is a idempotent
    let solution = solution_with_all_inputs(0);
    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();
    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result, vec![solution]);

    // Insert many solutions
    let solutions: Vec<_> = (0..10).map(solution_with_all_inputs).collect();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result, solutions);
}

create_test!(move_solutions_to_solved);

async fn move_solutions_to_solved<S: Storage>(storage: S) {
    let solutions: Vec<_> = (0..10).map(solution_with_all_inputs).collect();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    // Move none
    storage.move_solutions_to_solved(&[]).await.unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 10);
    let result = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 0);

    let hashes: Vec<_> = solutions.iter().map(essential_hash::hash).collect();

    // Move some
    storage
        .move_solutions_to_solved(&hashes[3..5])
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 8);
    assert_eq!(&result[0..3], &solutions[0..3]);
    assert_eq!(&result[3..8], &solutions[5..10]);

    let result = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(&result[0].batch.solutions[..], &solutions[3..5]);

    // Move missing hash is noop
    let hash = essential_hash::hash(&solution_with_all_inputs(11));
    storage.move_solutions_to_solved(&[hash]).await.unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 8);
    let result = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 1);

    // Move some with missing hash
    storage
        .move_solutions_to_solved(&[hashes[9], hash])
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 7);
    assert_eq!(&result[0..2], &solutions[0..2]);
    assert_eq!(&result[3..7], &solutions[5..9]);

    let result = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(&result[0].batch.solutions[..], &solutions[3..5]);
    assert_eq!(&result[1].batch.solutions[..], &solutions[9..10]);

    // Move all
    storage.move_solutions_to_solved(&hashes).await.unwrap();
    let result = storage.list_solutions_pool(None).await.unwrap();
    assert!(result.is_empty());
    let result = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(&result[0].batch.solutions[..], &solutions[3..5]);
    assert_eq!(&result[1].batch.solutions[..], &solutions[9..10]);
    assert_eq!(result[2].batch.solutions.len(), 7);
    assert_eq!(&result[2].batch.solutions[0..3], &solutions[0..3]);
    assert_eq!(&result[2].batch.solutions[3..7], &solutions[5..9]);

    // Move all again is noop
    storage.move_solutions_to_solved(&hashes).await.unwrap();
    let result = storage.list_solutions_pool(None).await.unwrap();
    assert!(result.is_empty());
    let result = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(&result[0].batch.solutions[..], &solutions[3..5]);
    assert_eq!(&result[1].batch.solutions[..], &solutions[9..10]);
    assert_eq!(result[2].batch.solutions.len(), 7);
    assert_eq!(&result[2].batch.solutions[0..3], &solutions[0..3]);
    assert_eq!(&result[2].batch.solutions[3..7], &solutions[5..9]);

    assert_eq!(result[0].number, 0);
    assert_eq!(result[1].number, 1);
    assert_eq!(result[2].number, 2);
}

create_test!(get_intent);

async fn get_intent<S: Storage>(storage: S) {
    let mut set = sign_intent_set_with_random_keypair(vec![
        intent_with_salt(0),
        intent_with_salt(1),
        intent_with_salt(2),
    ]);
    set.set.sort_by_key(essential_hash::content_addr);

    storage
        .insert_intent_set(StorageLayout {}, set.clone())
        .await
        .unwrap();

    let mut set2 =
        sign_intent_set_with_random_keypair(vec![intent_with_salt(0), intent_with_salt(1)]);
    set2.set.sort_by_key(essential_hash::content_addr);

    storage
        .insert_intent_set(StorageLayout {}, set2.clone())
        .await
        .unwrap();

    let set_address = essential_hash::intent_set_addr::from_intents(&set.set);
    let set_address2 = essential_hash::intent_set_addr::from_intents(&set2.set);

    // Get existing intent
    for intent in &set.set {
        let address = IntentAddress {
            set: set_address.clone(),
            intent: essential_hash::content_addr(intent),
        };
        let result = storage.get_intent(&address).await.unwrap().unwrap();
        assert_eq!(result, *intent);
    }

    for intent in &set2.set {
        let address = IntentAddress {
            set: set_address2.clone(),
            intent: essential_hash::content_addr(intent),
        };
        let result = storage.get_intent(&address).await.unwrap().unwrap();
        assert_eq!(result, *intent);
    }

    // Get missing intent
    let address = IntentAddress::empty();
    let result = storage.get_intent(&address).await.unwrap();
    assert!(result.is_none());

    let address = IntentAddress {
        set: set_address.clone(),
        intent: ContentAddress::empty(),
    };
    let result = storage.get_intent(&address).await.unwrap();
    assert!(result.is_none());

    let address = IntentAddress {
        set: ContentAddress::empty(),
        intent: essential_hash::content_addr(&set.set[0]),
    };
    let result = storage.get_intent(&address).await.unwrap();
    assert!(result.is_none());

    // Wrong set
    let address = IntentAddress {
        set: set_address2.clone(),
        intent: essential_hash::content_addr(&intent_with_salt(2)),
    };
    let result = storage.get_intent(&address).await.unwrap();
    assert!(result.is_none());
}

create_test!(get_intent_set);

async fn get_intent_set<S: Storage>(storage: S) {
    let mut sets = vec![];
    for i in 0..2 {
        let mut set = sign_intent_set_with_random_keypair(vec![
            intent_with_salt(i),
            intent_with_salt(i + 1),
            intent_with_salt(i + 2),
        ]);
        set.set.sort_by_key(essential_hash::content_addr);

        storage
            .insert_intent_set(StorageLayout {}, set.clone())
            .await
            .unwrap();
        sets.push(set);
    }

    // Get existing sets
    for set in &sets {
        let address = essential_hash::intent_set_addr::from_intents(&set.set);
        let result = storage.get_intent_set(&address).await.unwrap().unwrap();
        assert_eq!(result.signature, set.signature);
        assert_eq!(result.set, set.set);
    }

    // Get missing set
    let result = storage
        .get_intent_set(&ContentAddress::empty())
        .await
        .unwrap();
    assert!(result.is_none());
}

create_test!(list_intent_sets);

async fn list_intent_sets<S: Storage>(storage: S) {
    // List empty
    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert!(result.is_empty());

    let mut sets = vec![];
    for i in 0..102 {
        let mut set = sign_intent_set_with_random_keypair(vec![
            intent_with_salt(i),
            intent_with_salt(i + 1),
            intent_with_salt(i + 2),
        ]);
        set.set.sort_by_key(essential_hash::content_addr);

        storage
            .insert_intent_set(StorageLayout {}, set.clone())
            .await
            .unwrap();
        sets.push(set.set);
    }

    // List up to page size
    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(&result[..], &sets[0..100]);

    // List first page
    let result = storage.list_intent_sets(None, Some(0)).await.unwrap();
    assert_eq!(&result[..], &sets[0..100]);

    // List second page
    let result = storage.list_intent_sets(None, Some(1)).await.unwrap();
    assert_eq!(&result[..], &sets[100..102]);

    // List empty third page
    let result = storage.list_intent_sets(None, Some(2)).await.unwrap();
    assert!(result.is_empty());

    // List within time
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let start = time - std::time::Duration::from_secs(100);
    let end = time + std::time::Duration::from_secs(100);
    let result = storage
        .list_intent_sets(Some(start..end), None)
        .await
        .unwrap();
    assert_eq!(&result[..], &sets[0..100]);

    // List within time and page
    let result = storage
        .list_intent_sets(Some(start..end), Some(1))
        .await
        .unwrap();
    assert_eq!(&result[..], &sets[100..102]);

    // List within time and empty page
    let result = storage
        .list_intent_sets(Some(start..end), Some(2))
        .await
        .unwrap();
    assert!(result.is_empty());

    // List outside time
    let end = time - std::time::Duration::from_secs(80);
    let result = storage
        .list_intent_sets(Some(start..end), None)
        .await
        .unwrap();
    assert!(result.is_empty());
}

create_test!(list_solutions_pool);

async fn list_solutions_pool<S: Storage>(storage: S) {
    let solutions: Vec<_> = (0..102).map(solution_with_all_inputs).collect();

    for solution in &solutions {
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    // List up to page size
    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(&result[..], &solutions[0..100]);

    // List first page
    let result = storage.list_solutions_pool(Some(0)).await.unwrap();
    assert_eq!(&result[..], &solutions[0..100]);

    // List second page
    let result = storage.list_solutions_pool(Some(1)).await.unwrap();
    assert_eq!(&result[..], &solutions[100..102]);

    // List empty third page
    let result = storage.list_solutions_pool(Some(2)).await.unwrap();
    assert!(result.is_empty());
}

create_test!(list_failed_solutions_pool);

async fn list_failed_solutions_pool<S: Storage>(storage: S) {
    let solutions: Vec<_> = (0..102).map(solution_with_all_inputs).collect();

    let mut hashes = vec![];

    for solution in &solutions {
        hashes.push((
            essential_hash::hash(solution),
            SolutionFailReason::NotComposable,
        ));
        storage
            .insert_solution_into_pool(solution.clone())
            .await
            .unwrap();
    }

    storage.move_solutions_to_failed(&hashes).await.unwrap();
}
