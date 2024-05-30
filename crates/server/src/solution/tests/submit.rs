use crate::{solution::submit_solution, test_utils::sanity_solution};
use essential_storage::Storage;

#[tokio::test]
async fn test_submit_empty_solution() {
    let (solution, storage) = sanity_solution().await;
    let _result = submit_solution(&storage, solution.clone()).await.unwrap();
    let result = storage.list_solutions_pool(None).await.unwrap();
    assert_eq!(result, vec![solution]);
}
