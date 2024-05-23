use crate::{
    solution::{read::read_intents_from_storage, validate_intents},
    test_utils::sanity_solution,
};
use essential_types::IntentAddress;
use test_utils::empty::Empty;

#[tokio::test]
async fn test_retrieve_intent_set() {
    let (solution, storage) = sanity_solution().await;
    let intents = read_intents_from_storage(&solution, &storage)
        .await
        .unwrap();
    validate_intents(&solution, &intents).unwrap();
}

#[tokio::test]
#[should_panic(expected = "Failed to retrieve intent set from storage")]
async fn test_fail_to_retrieve_intent_set() {
    let (mut solution, storage) = sanity_solution().await;
    // Corrupt the intent set read from storage
    solution.data[0].intent_to_solve = IntentAddress::empty();
    let intents = read_intents_from_storage(&solution, &storage)
        .await
        .unwrap();
    validate_intents(&solution, &intents).unwrap();
}
