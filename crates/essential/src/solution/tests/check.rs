use crate::{solution::check_solution, utils::solution_with_deps};

#[tokio::test]
async fn test_check_solution_with_deps() {
    let (solution, storage) = solution_with_deps().await;
    check_solution(&storage, solution).await.unwrap();
}
