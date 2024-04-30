use crate::{run::run, test_utils::test_solution};
use storage::{StateStorage, Storage};
use test_utils::sign_with_random_keypair;

#[tokio::test]
async fn test_run() {
    let (solution, storage) = test_solution(None, 1).await;
    let solution = sign_with_random_keypair(solution);
    let first_state_mutation = &solution.data.state_mutations[0];
    let mutation_key = first_state_mutation.mutations[0].key;
    let mutation_address = solution.data.data[first_state_mutation.pathway as usize]
        .intent_to_solve
        .set
        .clone();
    let solution_signature = solution.signature.clone();

    storage.insert_solution_into_pool(solution).await.unwrap();

    let pre_state = storage
        .query_state(&mutation_address, &mutation_key)
        .await
        .unwrap();
    assert!(pre_state.is_none());

    run(&storage).await.unwrap();

    let result = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].batch.solutions.len(), 1);
    assert_eq!(result[0].batch.solutions[0].signature, solution_signature);

    let post_state = storage
        .query_state(&mutation_address, &mutation_key)
        .await
        .unwrap();
    assert!(post_state.is_some());
    assert_eq!(post_state.unwrap(), 42);
}

#[tokio::test]
#[ignore]
async fn test_run_weird_behaviour() {
    let (solution, storage) = test_solution(None, 1).await;
    let (solution2, _) = test_solution(Some(storage.clone()), 2).await;
    let (solution3, _) = test_solution(Some(storage.clone()), 3).await;
    let solution = sign_with_random_keypair(solution);
    let solution2 = sign_with_random_keypair(solution2);
    let solution3 = sign_with_random_keypair(solution3);
    let solution_signature = solution.signature.clone();
    let solution2_signature = solution2.signature.clone();
    let solution3_signature = solution3.signature.clone();

    storage.insert_solution_into_pool(solution).await.unwrap();
    storage.insert_solution_into_pool(solution2).await.unwrap();

    run(&storage).await.unwrap();

    storage.insert_solution_into_pool(solution3).await.unwrap();

    run(&storage).await.unwrap();

    let result = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].batch.solutions.len(), 2);
    assert_eq!(result[0].batch.solutions[0].signature, solution_signature);
    assert_eq!(result[0].batch.solutions[1].signature, solution2_signature);
    assert_eq!(result[1].batch.solutions.len(), 1);
    assert_eq!(result[1].batch.solutions[0].signature, solution3_signature);
}
