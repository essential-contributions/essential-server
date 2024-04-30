use crate::{solution::submit_solution, test_utils::sanity_solution};
use storage::Storage;
use test_utils::sign_with_random_keypair;

#[tokio::test]
async fn test_submit_empty_solution() {
    let (solution, storage) = sanity_solution().await;
    let solution = sign_with_random_keypair(solution);
    let _result = submit_solution(&storage, solution.clone()).await.unwrap();
    let result = storage.list_solutions_pool().await.unwrap();
    assert_eq!(result, vec![solution]);
}
