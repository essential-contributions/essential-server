use crate::{
    solution::{
        read::{read_intents_from_storage, read_partial_solutions_from_storage},
        validate_intents,
    },
    test_utils::sanity_solution,
};
use essential_types::{ContentAddress, IntentAddress};
use test_utils::{empty::Empty, sign_with_random_keypair};

#[tokio::test]
async fn test_retrieve_intent_set() {
    let (solution, storage) = sanity_solution().await;
    let solution = sign_with_random_keypair(solution);
    let intents = read_intents_from_storage(&solution.data, &storage)
        .await
        .unwrap();
    validate_intents(&solution.data, &intents).unwrap();
}

#[tokio::test]
#[should_panic(expected = "Failed to retrieve intent set from storage")]
async fn test_fail_to_retrieve_intent_set() {
    let (mut solution, storage) = sanity_solution().await;
    // Corrupt the intent set read from storage
    solution.data[0].intent_to_solve = IntentAddress::empty();
    let solution = sign_with_random_keypair(solution);
    let intents = read_intents_from_storage(&solution.data, &storage)
        .await
        .unwrap();
    validate_intents(&solution.data, &intents).unwrap();
}

#[tokio::test]
#[should_panic(expected = "Failed to retrieve partial solution from storage")]
async fn test_fail_to_retrieve_partial_solution() {
    let (mut solution, storage) = sanity_solution().await;
    solution.partial_solutions = vec![sign_with_random_keypair(ContentAddress::empty())];
    let solution = sign_with_random_keypair(solution);
    let _partial_solutions = read_partial_solutions_from_storage(&solution.data, &storage)
        .await
        .unwrap();
}
