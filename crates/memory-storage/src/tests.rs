use super::*;
use essential_hash::hash;
use std::vec;
use test_utils::{
    intent_with_decision_variables, sign_with_random_keypair, solution_with_decision_variables,
};

#[tokio::test]
async fn test_insert_intent_set() {
    let storage = MemoryStorage::new();
    let storage_layout = StorageLayout {};
    let intent_sets = [
        sign_with_random_keypair(vec![
            intent_with_decision_variables(0),
            intent_with_decision_variables(1),
            intent_with_decision_variables(2),
        ]),
        sign_with_random_keypair(vec![
            intent_with_decision_variables(2),
            intent_with_decision_variables(3),
            intent_with_decision_variables(4),
        ]),
    ];
    storage
        .insert_intent_set(storage_layout.clone(), intent_sets[0].clone())
        .await
        .unwrap();

    storage
        .insert_intent_set(storage_layout.clone(), intent_sets[0].clone())
        .await
        .unwrap();

    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(result, vec![intent_sets[0].data.clone()]);

    storage
        .insert_intent_set(storage_layout, intent_sets[1].clone())
        .await
        .unwrap();

    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(
        result,
        vec![intent_sets[0].data.clone(), intent_sets[1].data.clone()]
    );

    for intent_set in &intent_sets {
        for intent in &intent_set.data {
            let address = IntentAddress {
                set: essential_hash::intent_set_addr::from_intents(&intent_set.data),
                intent: essential_hash::content_addr(intent),
            };
            let result = storage.get_intent(&address).await.unwrap().unwrap();
            assert_eq!(&result, intent);
        }
    }

    let result = storage
        .get_storage_layout(&essential_hash::intent_set_addr::from_intents(
            &intent_sets[0].data,
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result, StorageLayout {});
}

#[tokio::test]
async fn test_solutions() {
    let storage = MemoryStorage::new();
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

    let result = storage.list_solutions_pool().await.unwrap();
    assert_eq!(result, vec![solution.clone()]);

    storage
        .insert_solution_into_pool(solution2.clone())
        .await
        .unwrap();
    let result = storage.list_solutions_pool().await.unwrap();
    assert_eq!(result.len(), 2);
    assert!(result.contains(&solution));
    assert!(result.contains(&solution2));

    storage
        .move_solutions_to_solved(&[hash(&solution)])
        .await
        .unwrap();

    let result = storage.list_solutions_pool().await.unwrap();
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

    let result = storage.list_solutions_pool().await.unwrap();
    assert_eq!(result.len(), 1);
    assert!(result.contains(&solution4));

    let solution4_hash = hash(&solution4);
    let solution4_fail_reason = SolutionFailReason::NotComposable;
    storage
        .move_solutions_to_failed(&[(solution4_hash, solution4_fail_reason.clone())])
        .await
        .unwrap();

    let result = storage.list_failed_solutions_pool().await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].solution, solution4);

    let result = storage.get_solution(solution4_hash).await.unwrap().unwrap();
    assert_eq!(result.outcome, CheckOutcome::Fail(solution4_fail_reason));

    let result = storage.list_solutions_pool().await.unwrap();
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
                .unwrap(),
        )
        .await
        .unwrap();

    let result = storage.list_failed_solutions_pool().await.unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_update_and_query_state() {
    let storage = MemoryStorage::new();

    let intent_set = sign_with_random_keypair(vec![intent_with_decision_variables(0)]);
    let address = essential_hash::intent_set_addr::from_intents(&intent_set.data);
    let key = [0; 4];
    let word = Some(42);

    // Test updating the state without an intent set
    storage
        .update_state(&address, &key, word)
        .await
        .unwrap_err();

    // Test querying the state
    let query_result = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(query_result, None);

    storage
        .insert_intent_set(StorageLayout {}, intent_set.clone())
        .await
        .unwrap();

    // Test updating the state
    let old = storage.update_state(&address, &key, word).await.unwrap();
    assert_eq!(old, None);

    // Test querying the state
    let query_result = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(query_result, word);

    // Test updating the state
    let old = storage.update_state(&address, &key, Some(1)).await.unwrap();
    assert_eq!(old, word);

    // Test querying the state
    let query_result = storage.query_state(&address, &key).await.unwrap();
    assert_eq!(query_result, Some(1));

    // Test querying empty state
    let query_result = storage.query_state(&address, &[1; 4]).await.unwrap();
    assert_eq!(query_result, None);
}
