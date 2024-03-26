use memory_storage::MemoryStorage;

use crate::deploy::deploy;
use test_utils::{empty_intent, empty_solution, random_keypair};
use utils::sign;

use super::*;

#[tokio::test]
#[ignore]
async fn test_submit_solution() {
    let storage = MemoryStorage::default();
    let solution = empty_solution();
    let solution = sign(solution, random_keypair().0);
    let _result = submit_solution(&storage, solution.clone()).await.unwrap();
    let _result = storage.list_solutions_pool().await.unwrap();
    // assert_eq!(result, vec![solution]);
}

#[tokio::test]
#[ignore]
async fn test_solve() {
    let storage = MemoryStorage::default();
    let intent = empty_intent();
    let intent = sign(vec![intent], random_keypair().0);
    let _result = deploy(&storage, intent.clone()).await.unwrap();
    let solution = empty_solution();
    // solution.data.push(SolutionData {
    //     intent_to_solve: todo!(),
    //     decision_variables: todo!(),
    // });
    let solution = sign(solution, random_keypair().0);
    submit_solution(&storage, solution.clone()).await.unwrap();
    solve(&storage).await.unwrap();
    let result = storage.list_solutions_pool().await.unwrap();
    assert!(result.is_empty());
    let _result = storage.list_winning_blocks(None, None).await.unwrap();
    // Assert that the solution is in the only winning batch
}
