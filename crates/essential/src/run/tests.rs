use crate::{run::run, utils::solution_with_deps};
use storage::Storage;
use test_utils::sign_with_random_keypair;
use utils::hash;

#[tokio::test]
async fn test_run() {
    let (solution, storage) = solution_with_deps().await;
    let solution_hash = hash(&solution);
    let solution = sign_with_random_keypair(solution);
    storage.insert_solution_into_pool(solution).await.unwrap();

    run(&storage).await.unwrap();

    let result = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].batch.solutions.len(), 1);
    assert_eq!(hash(&result[0].batch.solutions[0].data), solution_hash);
}
