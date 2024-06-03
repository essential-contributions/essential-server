use common::create_test;
use essential_hash::hash;
use essential_memory_storage::MemoryStorage;
use essential_storage::{
    failed_solution::{CheckOutcome, SolutionFailReason},
    Storage,
};
use essential_types::IntentAddress;
use test_utils::{
    intent_with_salt, sign_intent_set_with_random_keypair, solution_with_decision_variables,
};

mod common;
#[cfg(feature = "rqlite")]
mod rqlite;

create_test!(insert_intent_set);

async fn insert_intent_set<S: Storage>(storage: S) {
    let mut intent_sets = [
        sign_intent_set_with_random_keypair(vec![
            intent_with_salt(0),
            intent_with_salt(1),
            intent_with_salt(2),
        ]),
        sign_intent_set_with_random_keypair(vec![
            intent_with_salt(2),
            intent_with_salt(3),
            intent_with_salt(4),
        ]),
    ];

    // Order intents by their CA, as that's how `list_intent_sets` will return them.
    for signed in &mut intent_sets {
        signed.set.sort_by_key(essential_hash::content_addr);
    }

    storage
        .insert_intent_set(intent_sets[0].clone())
        .await
        .unwrap();

    storage
        .insert_intent_set(intent_sets[0].clone())
        .await
        .unwrap();

    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(result, vec![intent_sets[0].set.clone()]);

    storage
        .insert_intent_set(intent_sets[1].clone())
        .await
        .unwrap();

    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(
        result,
        vec![intent_sets[0].set.clone(), intent_sets[1].set.clone()]
    );

    for intent_set in &intent_sets {
        for intent in &intent_set.set {
            let address = IntentAddress {
                set: essential_hash::intent_set_addr::from_intents(&intent_set.set),
                intent: essential_hash::content_addr(intent),
            };
            let result = storage.get_intent(&address).await.unwrap().unwrap();
            assert_eq!(&result, intent);
        }
    }
}

create_test!(solutions);

async fn solutions<S: Storage>(storage: S) {
    let solution = solution_with_decision_variables(0);
    let solution2 = solution_with_decision_variables(1);
    let solution3 = solution_with_decision_variables(2);
    let solution4 = solution_with_decision_variables(3);

    // Idempotent insert
    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();
    storage
        .insert_solution_into_pool(solution.clone())
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result, vec![solution.clone()]);

    storage
        .insert_solution_into_pool(solution2.clone())
        .await
        .unwrap();
    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 2);
    assert!(result.contains(&solution));
    assert!(result.contains(&solution2));

    storage
        .move_solutions_to_solved(&[hash(&solution)])
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert!(result.contains(&solution2));

    let result = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].batch.solutions, vec![solution.clone()]);

    storage
        .insert_solution_into_pool(solution3.clone())
        .await
        .unwrap();

    storage
        .insert_solution_into_pool(solution4.clone())
        .await
        .unwrap();

    storage
        .move_solutions_to_solved(&[hash(&solution2), hash(&solution3)])
        .await
        .unwrap();

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert!(result.contains(&solution4));

    let solution4_hash = hash(&solution4);
    let solution4_fail_reason = SolutionFailReason::NotComposable;
    storage
        .move_solutions_to_failed(&[(solution4_hash, solution4_fail_reason.clone())])
        .await
        .unwrap();

    let result = storage.list_failed_solutions_pool(None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].solution, solution4);

    let result = storage.get_solution(solution4_hash).await.unwrap().unwrap();
    assert_eq!(
        result.outcome,
        vec![CheckOutcome::Fail(solution4_fail_reason)]
    );

    let result = storage.list_solutions_pool(None).await.unwrap();
    assert!(result.is_empty());

    let result = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].batch.solutions, vec![solution.clone()]);
    assert_eq!(
        result[1].batch.solutions,
        vec![solution2.clone(), solution3.clone()]
    );

    storage
        .prune_failed_solutions(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                + std::time::Duration::from_secs(10),
        )
        .await
        .unwrap();

    let result = storage.list_failed_solutions_pool(None).await.unwrap();
    assert!(result.is_empty());
}

create_test!(update_and_query_state);

async fn update_and_query_state<S: Storage>(storage: S) {
    let intent_set = sign_intent_set_with_random_keypair(vec![intent_with_salt(0)]);
    let address = essential_hash::intent_set_addr::from_intents(&intent_set.set);
    let key = vec![0; 4];
    let word = vec![42];

    // Test updating the state without an intent set
    storage
        .update_state(&address, &key, word.clone())
        .await
        .unwrap_err();

    // Test querying the state
    let query_result = storage.query_state(&address, &key).await.unwrap();
    assert!(query_result.is_empty());

    storage.insert_intent_set(intent_set.clone()).await.unwrap();

    // Test updating the state
    let old = storage
        .update_state(&address, &key, word.clone())
        .await
        .unwrap();
    assert!(old.is_empty());

    // Test querying the state
    let query_result = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(query_result, word);

    // Test updating the state
    let old = storage.update_state(&address, &key, vec![1]).await.unwrap();
    assert_eq!(old, word);

    // Test querying the state
    let query_result = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(query_result, vec![1]);

    // Test querying empty state
    let query_result = storage.query_state(&address, &vec![1; 4]).await.unwrap();
    assert!(query_result.is_empty());
}
