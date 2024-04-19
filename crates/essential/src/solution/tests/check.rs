use crate::solution::check_solution;
use essential_types::solution::Solution;
use memory_storage::MemoryStorage;
use test_utils::empty::Empty;

#[tokio::test]
async fn test_check_empty_solution() {
    let storage = MemoryStorage::default();
    let solution = Solution::empty();
    check_solution(&storage, solution).await.unwrap();
}
