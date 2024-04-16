use crate::solution::submit_solution;
use essential_types::solution::Solution;
use memory_storage::MemoryStorage;
use storage::Storage;
use test_utils::{empty::Empty, sign_with_random_keypair};

#[tokio::test]
async fn test_submit_empty_solution() {
    let storage = MemoryStorage::default();
    let solution = Solution::empty();
    let solution = sign_with_random_keypair(solution);
    let result = submit_solution(&storage, solution.clone()).await.unwrap();
    let result = storage.list_solutions_pool().await.unwrap();
    assert_eq!(result, vec![solution]);
}
