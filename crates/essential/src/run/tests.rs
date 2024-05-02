use crate::{
    run::run,
    solution::submit_solution,
    test_utils::{counter_intent, counter_solution, deploy_intent, test_solution},
};
use storage::{QueryState, Storage};
use test_utils::sign_with_random_keypair;

#[tokio::test]
async fn test_run() {
    let (unsigned_solution, storage) = test_solution(None, 1).await;
    let solution = sign_with_random_keypair(unsigned_solution.clone());
    let solution_signature = solution.signature.clone();

    let first_state_mutation = &solution.data.state_mutations[0];
    let mutation_key = first_state_mutation.mutations[0].key;
    let mutation_address = solution.data.data[first_state_mutation.pathway as usize]
        .intent_to_solve
        .set
        .clone();

    submit_solution(&storage, solution).await.unwrap();

    let pre_state = storage
        .query_state(&mutation_address, &mutation_key)
        .await
        .unwrap();
    assert!(pre_state.is_none());

    run(&storage).await.unwrap();

    let post_state = storage
        .query_state(&mutation_address, &mutation_key)
        .await
        .unwrap();
    assert!(post_state.is_some());
    assert_eq!(post_state.unwrap(), 42);

    let blocks = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].batch.solutions.len(), 1);
    assert_eq!(blocks[0].batch.solutions[0].signature, solution_signature);

    let solution2 = unsigned_solution; // same as solution
    let (solution3, _) = test_solution(Some(storage.clone()), 2).await;
    let solution2 = sign_with_random_keypair(solution2);
    let solution3 = sign_with_random_keypair(solution3);
    let solution3_signature = solution3.signature.clone();

    submit_solution(&storage, solution2).await.unwrap();
    submit_solution(&storage, solution3).await.unwrap();

    run(&storage).await.unwrap();

    let blocks = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1].batch.solutions.len(), 1);
    assert!(blocks[1]
        .batch
        .solutions
        .iter()
        .any(|s| s.signature == solution3_signature));
}

#[tokio::test]
async fn test_counter() {
    let intent = counter_intent(1);
    let (intent_address, storage) = deploy_intent(intent.clone()).await;

    let unsigned_solution = counter_solution(intent_address.clone(), 1, 1).await;
    let solution = sign_with_random_keypair(unsigned_solution.clone());
    let solution_signature = &solution.signature;
    let mutation_key = solution.data.state_mutations[0].mutations[0].key;

    let solution_clone = solution.clone();

    let solution2 = counter_solution(intent_address.clone(), 1, 2).await;
    let solution2 = sign_with_random_keypair(solution2.clone());
    let solution2_signature = &solution2.signature;

    let solution3 = counter_solution(intent_address.clone(), 1, 3).await;
    let solution3 = sign_with_random_keypair(solution3.clone());
    let solution3_signature = &solution3.signature;

    let solution4 = counter_solution(intent_address.clone(), 1, 4).await;
    let solution4 = sign_with_random_keypair(solution4.clone());
    let solution4_signature = &solution4.signature;

    submit_solution(&storage, solution.clone()).await.unwrap();
    submit_solution(&storage, solution_clone.clone())
        .await
        .unwrap();
    submit_solution(&storage, solution2.clone()).await.unwrap();
    submit_solution(&storage, solution4.clone()).await.unwrap();

    let pre_state = storage
        .query_state(&intent_address.set, &mutation_key)
        .await
        .unwrap();
    assert!(pre_state.is_none());

    run(&storage).await.unwrap();

    let post_state = storage
        .query_state(&intent_address.set, &mutation_key)
        .await
        .unwrap();
    assert!(post_state.is_some());
    assert_eq!(post_state.unwrap(), 2);

    let blocks = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].batch.solutions.len(), 2);
    let signatures: Vec<&essential_types::Signature> = blocks[0]
        .batch
        .solutions
        .iter()
        .map(|s| &s.signature)
        .collect();
    assert!(signatures.contains(&solution_signature));
    assert!(signatures.contains(&solution2_signature));

    submit_solution(&storage, solution3.clone()).await.unwrap();
    submit_solution(&storage, solution4.clone()).await.unwrap();

    run(&storage).await.unwrap();

    let post_state = storage
        .query_state(&intent_address.set, &mutation_key)
        .await
        .unwrap();
    assert!(post_state.is_some());
    assert_eq!(post_state.unwrap(), 4);

    let blocks = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1].batch.solutions.len(), 2);
    let signatures: Vec<&essential_types::Signature> = blocks[1]
        .batch
        .solutions
        .iter()
        .map(|s| &s.signature)
        .collect();
    assert!(signatures.contains(&solution3_signature));
    assert!(signatures.contains(&solution4_signature));
}
