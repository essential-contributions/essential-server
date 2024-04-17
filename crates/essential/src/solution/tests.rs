use super::*;
use crate::deploy::deploy;
use memory_storage::MemoryStorage;
use test_utils::{empty::Empty, sign_with_random_keypair};

#[tokio::test]
#[ignore]
async fn test_submit_solution() {
    let storage = MemoryStorage::default();
    let solution = Solution::empty();
    let solution = sign_with_random_keypair(solution);
    let _result = submit_solution(&storage, solution.clone()).await.unwrap();
    let _result = storage.list_solutions_pool().await.unwrap();
    // assert_eq!(result, vec![solution]);
}

#[tokio::test]
#[ignore]
async fn test_solve() {
    let storage = MemoryStorage::default();
    let intent = Intent::empty();
    let intent = sign_with_random_keypair(vec![intent]);
    let _result = deploy(&storage, intent.clone()).await.unwrap();
    let solution = Solution::empty();
    // solution.data.push(SolutionData {
    //     intent_to_solve: todo!(),
    //     decision_variables: todo!(),
    // });
    let solution = sign_with_random_keypair(solution);
    submit_solution(&storage, solution.clone()).await.unwrap();
    solve(&storage).await.unwrap();
    let result = storage.list_solutions_pool().await.unwrap();
    assert!(result.is_empty());
    let _result = storage.list_winning_blocks(None, None).await.unwrap();
    // Assert that the solution is in the only winning batch
}
