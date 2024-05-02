use super::*;
use std::vec;
use test_utils::{
    intent_with_decision_variables, partial_solution_with_decision_variables,
    sign_with_random_keypair, solution_with_decision_variables,
};
use utils::hash;

#[tokio::test]
async fn test_insert_intent_set() {
    let storage = MemoryStorage::new();
    let storage_layout = StorageLayout {};
    let intents = [
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
        .insert_intent_set(storage_layout.clone(), intents[0].clone())
        .await
        .unwrap();

    storage
        .insert_intent_set(storage_layout.clone(), intents[0].clone())
        .await
        .unwrap();

    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(result, vec![intents[0].data.clone()]);

    storage
        .insert_intent_set(storage_layout, intents[1].clone())
        .await
        .unwrap();

    let result = storage.list_intent_sets(None, None).await.unwrap();
    assert_eq!(
        result,
        vec![intents[0].data.clone(), intents[1].data.clone()]
    );

    for intent in &intents {
        for j in 0..3 {
            let address = IntentAddress {
                set: ContentAddress(hash(&intent.data)),
                intent: ContentAddress(hash(&intent.data[j])),
            };

            let result = storage.get_intent(&address).await.unwrap().unwrap();

            assert_eq!(result, intent.data[j]);
        }
    }

    let result = storage
        .get_storage_layout(&ContentAddress(hash(&intents[0].data)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result, StorageLayout {});
}

#[tokio::test]
async fn test_solutions() {
    let storage = MemoryStorage::new();
    let solution = sign_with_random_keypair(solution_with_decision_variables(0));
    let solution2 = sign_with_random_keypair(solution_with_decision_variables(1));
    let solution3 = sign_with_random_keypair(solution_with_decision_variables(2));
    let solution4 = sign_with_random_keypair(solution_with_decision_variables(3));

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
        .move_solutions_to_solved(&[hash(&solution.data)])
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
        .move_solutions_to_solved(&[hash(&solution2.data), hash(&solution3.data)])
        .await
        .unwrap();

    let result = storage.list_solutions_pool().await.unwrap();
    assert_eq!(result.len(), 1);
    assert!(result.contains(&solution4));

    let solution4_hash = hash(&solution4.data);
    let solution4_fail_reason = SolutionFailReason::NotComposable;
    storage
        .move_solutions_to_failed(&[(solution4_hash, solution4_fail_reason.clone())])
        .await
        .unwrap();

    let result = storage.list_failed_solutions_pool().await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].solution, solution4);

    let result = storage.get_solution(solution4_hash).await.unwrap().unwrap();
    assert_eq!(result.outcome.unwrap(), solution4_fail_reason);

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
async fn test_partial_solutions() {
    let storage = MemoryStorage::new();
    let partial_solution1 = sign_with_random_keypair(partial_solution_with_decision_variables(0));
    let partial_solution2 = sign_with_random_keypair(partial_solution_with_decision_variables(1));
    let partial_solution3 = sign_with_random_keypair(partial_solution_with_decision_variables(2));

    // Idempotent insert
    storage
        .insert_partial_solution_into_pool(partial_solution1.clone())
        .await
        .unwrap();
    storage
        .insert_partial_solution_into_pool(partial_solution1.clone())
        .await
        .unwrap();

    let result = storage.list_partial_solutions_pool().await.unwrap();
    assert_eq!(result, vec![partial_solution1.clone()]);

    storage
        .insert_partial_solution_into_pool(partial_solution2.clone())
        .await
        .unwrap();
    let result = storage.list_partial_solutions_pool().await.unwrap();
    assert_eq!(result.len(), 2);
    assert!(result.contains(&partial_solution1));
    assert!(result.contains(&partial_solution2));

    let result = storage
        .get_partial_solution(&ContentAddress(hash(&partial_solution1.data)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result, partial_solution1);

    storage
        .move_partial_solutions_to_solved(&[hash(&partial_solution1.data)])
        .await
        .unwrap();

    let result = storage.list_partial_solutions_pool().await.unwrap();
    assert_eq!(result.len(), 1);
    assert!(result.contains(&partial_solution2));

    let result = storage
        .is_partial_solution_solved(&ContentAddress(hash(&partial_solution1.data)))
        .await
        .unwrap()
        .unwrap();
    assert!(result);
    let result = storage
        .is_partial_solution_solved(&ContentAddress(hash(&partial_solution2.data)))
        .await
        .unwrap()
        .unwrap();
    assert!(!result);

    storage
        .insert_partial_solution_into_pool(partial_solution3.clone())
        .await
        .unwrap();

    storage
        .move_partial_solutions_to_solved(&[
            hash(&partial_solution2.data),
            hash(&partial_solution3.data),
        ])
        .await
        .unwrap();

    let result = storage
        .is_partial_solution_solved(&ContentAddress(hash(&partial_solution3.data)))
        .await
        .unwrap()
        .unwrap();
    assert!(result);

    let result = storage.list_partial_solutions_pool().await.unwrap();
    assert!(result.is_empty());

    let result = storage
        .get_partial_solution(&ContentAddress(hash(&partial_solution2.data)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result, partial_solution2);

    let result = storage
        .get_partial_solution(&ContentAddress(hash(&partial_solution2.data)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result, partial_solution2);
}

#[tokio::test]
async fn test_update_and_query_state() {
    let storage = MemoryStorage::new();

    let intent_set = sign_with_random_keypair(vec![intent_with_decision_variables(0)]);
    let address = ContentAddress(hash(&intent_set.data));
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
