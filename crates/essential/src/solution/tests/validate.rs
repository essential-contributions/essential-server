use crate::{
    solution::{
        read::{read_intents_from_storage, read_partial_solutions_from_storage},
        validate::{
            validate_intents_against_solution, validate_partial_solutions_against_solution,
            validate_solution, validate_solution_with_deps, MAX_DECISION_VARIABLES,
            MAX_SOLUTION_DATA, MAX_STATE_MUTATIONS,
        },
    },
    utils::{
        deploy_empty_intent, deploy_empty_intent_and_get_solution, deploy_intent,
        deploy_partial_solution_with_data_to_storage,
    },
};
use essential_types::{
    intent::Intent,
    solution::{
        DecisionVariable, DecisionVariableIndex, PartialSolutionData, Solution, SolutionData,
        StateMutation,
    },
    ContentAddress, IntentAddress,
};
use memory_storage::MemoryStorage;
use test_utils::{empty::Empty, sign_corrupted, sign_with_random_keypair};

#[test]
fn test_empty_solution() {
    let solution = sign_with_random_keypair(Solution::empty());
    validate_solution(&solution).unwrap();
}

#[test]
#[should_panic(expected = "Failed to verify solution signature")]
fn test_fail_invalid_signature() {
    let solution = sign_corrupted(Solution::empty());
    validate_solution(&solution).unwrap();
}

#[test]
#[should_panic(expected = "Too many solution data")]
fn test_fail_too_many_solution_data() {
    let mut solution = Solution::empty();
    solution.data = (0..MAX_SOLUTION_DATA + 1)
        .map(|_| SolutionData::empty())
        .collect();
    let solution = sign_with_random_keypair(solution);
    validate_solution(&solution).unwrap();
}

#[test]
#[should_panic(expected = "Too many decision variables")]
fn test_fail_too_many_decision_variables() {
    let mut solution = Solution::empty();
    let solution_data = SolutionData {
        intent_to_solve: IntentAddress::empty(),
        decision_variables: vec![DecisionVariable::empty(); (MAX_DECISION_VARIABLES + 1) as usize],
    };
    solution.data.push(solution_data);
    let solution = sign_with_random_keypair(solution);
    validate_solution(&solution).unwrap();
}

#[test]
#[should_panic(expected = "Too many state mutations")]
fn test_fail_too_many_state_mutations() {
    let mut solution = Solution::empty();
    solution.state_mutations = (0..MAX_STATE_MUTATIONS + 1)
        .map(|_| StateMutation::empty())
        .collect();
    let solution = sign_with_random_keypair(solution);
    validate_solution(&solution).unwrap();
}

#[test]
#[should_panic(expected = "Too many partial solutions")]
fn test_fail_too_many_partial_solutions() {
    let mut solution = Solution::empty();
    solution.partial_solutions = (0..MAX_STATE_MUTATIONS + 1)
        .map(|_| sign_with_random_keypair(ContentAddress::empty()))
        .collect();
    let solution = sign_with_random_keypair(solution);
    validate_solution(&solution).unwrap();
}

#[tokio::test]
async fn test_retrieve_intent_set() {
    let (solution, _, storage) = deploy_empty_intent_and_get_solution().await;
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "Failed to retrieve intent set from storage")]
async fn test_fail_to_retrieve_intent_set() {
    let (mut solution, _, storage) = deploy_empty_intent_and_get_solution().await;
    // Corrupt the intent set read from storage
    solution.data[0].intent_to_solve = IntentAddress::empty();
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_retrieve_partial_solution() {
    let (mut solution, intent_address, storage) = deploy_empty_intent_and_get_solution().await;
    let partial_solution_data = PartialSolutionData {
        intent_to_solve: intent_address,
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
#[should_panic(expected = "Failed to retrieve partial solution from storage")]
async fn test_fail_to_retrieve_partial_solution() {
    let storage = MemoryStorage::new();
    let mut solution = Solution::empty();
    solution.partial_solutions = vec![sign_with_random_keypair(ContentAddress::empty())];
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_all_intents_must_be_in_the_set() {
    let (solution, _, storage) = deploy_empty_intent_and_get_solution().await;
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_all_state_mutations_must_have_an_intent_in_the_set() {
    let (mut solution, _, storage) = deploy_empty_intent_and_get_solution().await;
    solution.state_mutations = vec![StateMutation {
        pathway: 0,
        mutations: Default::default(),
    }];
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "All state mutations must have an intent in the set")]
async fn test_fail_all_state_mutations_must_have_an_intent_in_the_set() {
    let storage = MemoryStorage::new();
    let mut solution = Solution::empty();
    solution.state_mutations = vec![StateMutation {
        pathway: 0,
        mutations: Default::default(),
    }];
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "Decision variables mismatch")]
async fn test_fail_decision_variables_mismatch() {
    let (intent_address, storage) = deploy_empty_intent().await;
    let mut solution = Solution::empty();
    solution.data = vec![SolutionData {
        intent_to_solve: intent_address,
        decision_variables: vec![DecisionVariable::Inline(0)],
    }];
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "Invalid transient decision variable")]
async fn test_fail_invalid_transient_decision_variable() {
    let mut intent = Intent::empty();
    intent.slots.decision_variables = 1;
    let (intent_address, storage) = deploy_intent(intent).await;
    let mut solution = Solution::empty();
    solution.data = vec![SolutionData {
        intent_to_solve: intent_address,
        decision_variables: vec![DecisionVariable::Transient(DecisionVariableIndex {
            solution_data_index: 1, // TODO: does not fail when this is 0. Confirm this should not be the case
            variable_index: Default::default(),
        })],
    }];
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "All intents must be in the set")]
async fn test_fail_not_all_intents_in_set() {
    let (solution, intent_address, storage) = deploy_empty_intent_and_get_solution().await;
    let mut intents = read_intents_from_storage(&solution, &storage)
        .await
        .unwrap();
    intents.remove(&intent_address);
    validate_intents_against_solution(&solution, &intents).unwrap();
}

#[tokio::test]
#[should_panic(expected = "All partial solutions must be in the set")]
async fn test_fail_not_all_partial_solutions_in_set() {
    let storage = MemoryStorage::new();
    let (partial_solution_address, solution) = deploy_partial_solution_with_data_to_storage(
        &storage,
        &mut Solution::empty(),
        PartialSolutionData::empty(),
    )
    .await;
    let mut partial_solutions = read_partial_solutions_from_storage(&solution, &storage)
        .await
        .unwrap();
    // Corrupt the partial solutions read from storage
    partial_solutions.remove(&partial_solution_address);
    validate_partial_solutions_against_solution(&solution, &partial_solutions).unwrap();
}
