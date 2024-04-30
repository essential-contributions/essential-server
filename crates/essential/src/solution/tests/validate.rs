use crate::{
    solution::{
        read::{read_intents_from_storage, read_partial_solutions_from_storage},
        validate::{
            validate_intents_against_solution, validate_partial_solutions_against_solution,
            validate_solution, validate_solution_with_deps, MAX_DECISION_VARIABLES,
            MAX_SOLUTION_DATA, MAX_STATE_MUTATIONS,
        },
    },
    test_utils::{
        deploy_empty_intent, deploy_empty_intent_and_get_solution, deploy_intent,
        deploy_partial_solution_to_storage, deploy_partial_solution_with_data_to_storage,
        solution_with_deps,
    },
};
use essential_types::{
    intent::Intent,
    solution::{
        DecisionVariable, DecisionVariableIndex, PartialSolution, PartialSolutionData, Solution,
        SolutionData, StateMutation,
    },
    ContentAddress, IntentAddress,
};
use memory_storage::MemoryStorage;
use test_utils::{empty::Empty, sign_corrupted, sign_with_random_keypair};

#[test]
fn test_validate_solution() {
    let mut solution = Solution::empty();
    solution.data = vec![SolutionData {
        intent_to_solve: IntentAddress::empty(),
        decision_variables: vec![DecisionVariable::Inline(0)],
    }];
    solution.state_mutations = vec![StateMutation {
        pathway: 0,
        mutations: Default::default(),
    }];
    solution.partial_solutions = vec![sign_with_random_keypair(ContentAddress::empty())];
    let solution = sign_with_random_keypair(solution);
    validate_solution(&solution).unwrap();
}

#[tokio::test]
async fn test_validate_solution_with_deps() {
    let (solution, storage) = solution_with_deps().await;
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[test]
fn test_all_state_mutations_must_have_an_intent_in_the_set() {
    let mut solution = Solution::empty();
    solution.state_mutations = vec![StateMutation {
        pathway: 0,
        mutations: Default::default(),
    }];
    solution.data = vec![SolutionData::empty()];
    let solution = sign_with_random_keypair(solution);
    validate_solution(&solution).unwrap();
}

#[tokio::test]
async fn test_transient_decision_variable() {
    let mut intent = Intent::empty();
    intent.slots.decision_variables = 1;
    let (intent_address, storage) = deploy_intent(intent).await;
    let mut solution = Solution::empty();
    solution.data = vec![
        SolutionData {
            intent_to_solve: intent_address.clone(),
            decision_variables: vec![DecisionVariable::Transient(DecisionVariableIndex {
                solution_data_index: 1,
                variable_index: 0,
            })],
        },
        SolutionData {
            intent_to_solve: intent_address,
            decision_variables: vec![DecisionVariable::Inline(Default::default())],
        },
    ];
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[test]
#[should_panic(expected = "Invalid solution signature")]
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
#[should_panic(expected = "All state mutations must have an intent in the set")]
fn test_fail_all_state_mutations_must_have_an_intent_in_the_set() {
    let mut solution = Solution::empty();
    solution.state_mutations = vec![StateMutation {
        pathway: 0,
        mutations: Default::default(),
    }];
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

#[test]
#[should_panic(expected = "Invalid partial solution signature")]
fn test_fail_partial_solution_signature() {
    let mut solution = Solution::empty();
    solution.partial_solutions = vec![sign_corrupted(ContentAddress::empty())];
    let solution = sign_with_random_keypair(solution);
    validate_solution(&solution).unwrap();
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
            solution_data_index: 1,
            variable_index: Default::default(),
        })],
    }];
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
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

#[tokio::test]
#[should_panic(expected = "Partial solution intent to solve mismatch with solution data")]
async fn test_fail_partial_solution_data_must_be_in_the_set() {
    let storage = MemoryStorage::new();
    let (_, solution) = deploy_partial_solution_with_data_to_storage(
        &storage,
        &mut Solution::empty(),
        PartialSolutionData {
            intent_to_solve: IntentAddress::empty(),
            decision_variables: vec![],
        },
    )
    .await;
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "Partial solution decision variables mismatch with solution data")]
async fn test_fail_decision_variables_must_be_in_solution_data() {
    let mut intent = Intent::empty();
    intent.slots.decision_variables = 1;
    let (intent_address, storage) = deploy_intent(intent).await;
    let (_, mut solution) = deploy_partial_solution_with_data_to_storage(
        &storage,
        &mut Solution::empty(),
        PartialSolutionData {
            intent_to_solve: intent_address.clone(),
            decision_variables: vec![Some(DecisionVariable::Inline(i64::from(1)))],
        },
    )
    .await;
    solution.data = vec![SolutionData {
        intent_to_solve: intent_address,
        decision_variables: vec![DecisionVariable::Inline(i64::from(2))],
    }];
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "Partial solution state mutations mismatch with solution data")]
async fn test_fail_state_mutations_must_be_in_solution() {
    let (mut solution, intent_address, storage) = deploy_empty_intent_and_get_solution().await;
    let partial_solution = PartialSolution {
        data: vec![PartialSolutionData {
            intent_to_solve: intent_address.clone(),
            decision_variables: Default::default(),
        }],
        state_mutations: vec![StateMutation::empty()],
    };
    let partial_solution_address =
        deploy_partial_solution_to_storage(&storage, partial_solution).await;
    solution
        .partial_solutions
        .push(sign_with_random_keypair(partial_solution_address.clone()));
    let solution = sign_with_random_keypair(solution);
    validate_solution_with_deps(&solution, &storage)
        .await
        .unwrap();
}
