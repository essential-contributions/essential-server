use crate::{
    run::run,
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

    storage.insert_solution_into_pool(solution).await.unwrap();

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

    storage.insert_solution_into_pool(solution2).await.unwrap();
    storage.insert_solution_into_pool(solution3).await.unwrap();

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
#[ignore]
async fn test_counter() {
    let intent = counter_intent(1);
    let (intent_address, storage) = deploy_intent(intent.clone()).await;

    let unsigned_solution = counter_solution(intent_address.clone(), 1, 1).await;
    let solution = sign_with_random_keypair(unsigned_solution.clone());
    let solution_signature = solution.signature.clone();
    let mutation_key = solution.data.state_mutations[0].mutations[0].key;

    let solution2 = counter_solution(intent_address.clone(), 1, 2).await;
    let solution2 = sign_with_random_keypair(solution2.clone());
    let solution2_signature = solution2.signature.clone();

    let solution3 = sign_with_random_keypair(unsigned_solution);
    let solution3_signature = solution3.signature.clone();

    storage.insert_solution_into_pool(solution).await.unwrap();
    storage
        .insert_solution_into_pool(solution2.clone())
        .await
        .unwrap();
    storage.insert_solution_into_pool(solution3).await.unwrap();

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
    assert_eq!(post_state.unwrap(), 1);

    let blocks = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].batch.solutions.len(), 1);
    assert!(
        (blocks[0].batch.solutions[0].signature == solution_signature)
            || (blocks[0].batch.solutions[0].signature == solution3_signature)
    );

    storage.insert_solution_into_pool(solution2).await.unwrap();

    let pre_state = storage
        .query_state(&intent_address.set, &mutation_key)
        .await
        .unwrap();
    assert!(pre_state.is_some());
    assert_eq!(post_state.unwrap(), 1);

    run(&storage).await.unwrap();

    let post_state = storage
        .query_state(&intent_address.set, &mutation_key)
        .await
        .unwrap();
    assert!(post_state.is_some());
    assert_eq!(post_state.unwrap(), 2);

    let blocks = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1].batch.solutions.len(), 1);
    assert_eq!(blocks[1].batch.solutions[0].signature, solution2_signature);
}
