use crate::{
    solution::validate::validate_solution_with_deps,
    test_utils::{deploy_partial_solution_with_data_to_storage, sanity_solution},
};
use essential_types::{solution::PartialSolutionData, ContentAddress, IntentAddress};
use test_utils::{empty::Empty, sign_with_random_keypair};

#[tokio::test]
async fn test_retrieve_intent_set() {
    let (solution, storage) = sanity_solution().await;
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_retrieve_partial_solution() {
    let (mut solution, storage) = sanity_solution().await;
    let partial_solution_data = PartialSolutionData {
        intent_to_solve: solution.data[0].intent_to_solve.clone(),
        decision_variables: Default::default(),
    };
    let (_, solution) = deploy_partial_solution_with_data_to_storage(
        &storage,
        &mut solution,
        partial_solution_data,
    )
    .await;
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "Failed to retrieve intent set from storage")]
async fn test_fail_to_retrieve_intent_set() {
    let (mut solution, storage) = sanity_solution().await;
    // Corrupt the intent set read from storage
    solution.data[0].intent_to_solve = IntentAddress::empty();
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "Failed to retrieve partial solution from storage")]
async fn test_fail_to_retrieve_partial_solution() {
    let (mut solution, storage) = sanity_solution().await;
    solution.partial_solutions = vec![sign_with_random_keypair(ContentAddress::empty())];
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}
