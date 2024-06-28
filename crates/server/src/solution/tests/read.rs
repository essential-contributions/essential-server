use crate::{
    solution::{read::read_contract_from_storage, validate_contract},
    test_utils::sanity_solution,
};
use essential_types::PredicateAddress;
use test_utils::empty::Empty;

#[tokio::test]
async fn test_retrieve_contract() {
    let (solution, storage) = sanity_solution().await;
    let contract = read_contract_from_storage(&solution, &storage)
        .await
        .unwrap();
    validate_contract(&solution, &contract).unwrap();
}

#[tokio::test]
#[should_panic(expected = "Failed to retrieve contract from storage")]
async fn test_fail_to_retrieve_contract() {
    let (mut solution, storage) = sanity_solution().await;
    // Corrupt the contract read from storage
    solution.data[0].predicate_to_solve = PredicateAddress::empty();
    let contract = read_contract_from_storage(&solution, &storage)
        .await
        .unwrap();
    validate_contract(&solution, &contract).unwrap();
}
