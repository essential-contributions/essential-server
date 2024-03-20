use essential_types::solution::SolutionData;
use memory_storage::MemoryStorage;

use crate::deploy::deploy;
use test_utils::{empty_intent, empty_solution, sign};

use super::*;

#[tokio::test]
async fn test_submit_solution() {
    let storage = MemoryStorage::default();
    let solution = empty_solution();
    let solution = sign(solution);
    let result = submit_solution(&storage, solution.clone()).await.unwrap();
    let result = storage.list_solutions_pool().await.unwrap();
    // assert_eq!(result, vec![solution]);
}

#[tokio::test]
async fn test_solve() {
    let storage = MemoryStorage::default();
    let intent = empty_intent();
    let intent = sign(vec![intent]);
    let result = deploy(&storage, intent.clone()).await.unwrap();
    let mut solution = empty_solution();
    solution.data.push(SolutionData {
        intent_to_solve: todo!(),
        decision_variables: todo!(),
    });
    let solution = sign(solution);
    submit_solution(&storage, solution.clone()).await.unwrap();
    solve(&storage).await.unwrap();
    let result = storage.list_solutions_pool().await.unwrap();
    assert!(result.is_empty());
    let result = storage.list_winning_batches(None, None).await.unwrap();
    // Assert that the solution is in the only winning batch
}
