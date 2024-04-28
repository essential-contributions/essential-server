use crate::deploy::deploy;
use essential_types::{
    intent::Intent,
    slots::{Slots, StateSlot},
    solution::{
        DecisionVariable, Mutation, PartialSolution, PartialSolutionData, Solution, SolutionData,
        StateMutation,
    },
    ContentAddress, IntentAddress,
};
use memory_storage::MemoryStorage;
use storage::Storage;
use test_utils::{empty::Empty, sign_with_random_keypair, solution_with_intent};

// Sign and deploy given intent to newly created memory storage.
pub async fn deploy_intent(intent: Intent) -> (IntentAddress, MemoryStorage) {
    let storage = MemoryStorage::default();
    (deploy_intent_to_storage(&storage, intent).await, storage)
}

// Sign and deploy empty intent to newly created memory storage.
pub async fn deploy_empty_intent() -> (IntentAddress, MemoryStorage) {
    deploy_intent(Intent::empty()).await
}

// Sign an empty intent and deploy it to newly created memory storage,
// create a solution with the signed intent address.
pub async fn deploy_empty_intent_and_get_solution() -> (Solution, IntentAddress, MemoryStorage) {
    let (intent_address, storage) = deploy_empty_intent().await;
    let mut solution = Solution::empty();
    let mut solution_data = SolutionData::empty();
    solution_data.intent_to_solve = intent_address.clone();
    solution.data.push(solution_data);
    (solution, intent_address, storage)
}

// Create a partial solution with given data,
// sign it and deploy it to given storage,
// add signed partial solution address to given solution.
pub async fn deploy_partial_solution_with_data_to_storage<S: Storage>(
    storage: &S,
    solution: &mut Solution,
    partial_solution_data: PartialSolutionData,
) -> (ContentAddress, Solution) {
    let partial_solution = PartialSolution {
        data: vec![partial_solution_data],
        state_mutations: Default::default(),
    };
    let partial_solution_address =
        deploy_partial_solution_to_storage(storage, partial_solution).await;
    solution
        .partial_solutions
        .push(sign_with_random_keypair(partial_solution_address.clone()));
    (partial_solution_address, solution.to_owned())
}

// Sign given intent and deploy it to given storage.
pub async fn deploy_intent_to_storage<S: Storage>(storage: &S, intent: Intent) -> IntentAddress {
    let intent_hash = ContentAddress(utils::hash(&intent));
    let intent = sign_with_random_keypair(vec![intent]);
    let result = deploy(storage, intent).await.unwrap();
    IntentAddress {
        set: result,
        intent: intent_hash,
    }
}

// Sign given partial solution and deploy it to given storage.
pub async fn deploy_partial_solution_to_storage<S: Storage>(
    storage: &S,
    partial_solution: PartialSolution,
) -> ContentAddress {
    let partial_solution = sign_with_random_keypair(partial_solution);
    storage
        .insert_partial_solution_into_pool(partial_solution.clone())
        .await
        .unwrap();
    ContentAddress(utils::hash(&partial_solution.data))
}

// Empty solution with empty intent.
pub async fn sanity_solution() -> (Solution, MemoryStorage) {
    let (intent_address, storage) = deploy_intent(Intent::empty()).await;
    let solution = solution_with_intent(intent_address);
    (solution, storage)
}

// Solution that satisfies an intent with state read and constraint programs.
pub async fn solution_with_deps() -> (Solution, MemoryStorage) {
    // Intent that expects the value of previously unset state slot with index 0 to be 42.
    let mut intent = Intent::empty();
    intent.slots = Slots {
        decision_variables: 1, // Expects one decision variable.
        state: vec![StateSlot {
            index: 0,
            amount: 1,
            program_index: 0,
        }],
    };
    // Program to read state slot 0.
    intent.state_read = vec![essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::Memory::Alloc.into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateRead::WordRange,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect()];
    // Program to check pre-mutation value is None and
    // post-mutation value is 42 at slot 0.
    intent.constraints = vec![essential_constraint_vm::asm::to_bytes(vec![
        essential_constraint_vm::asm::Stack::Push(0).into(), // slot
        essential_constraint_vm::asm::Stack::Push(0).into(), // pre
        essential_constraint_vm::asm::Access::StateIsSome.into(),
        essential_constraint_vm::asm::Pred::Not.into(),
        essential_constraint_vm::asm::Stack::Push(0).into(), // slot
        essential_constraint_vm::asm::Stack::Push(1).into(), // post
        essential_constraint_vm::asm::Access::State.into(),
        essential_state_read_vm::asm::Stack::Push(42).into(),
        essential_constraint_vm::asm::Pred::Eq.into(),
        essential_constraint_vm::asm::Pred::And.into(),
    ])
    .collect()];
    let (intent_address, storage) = deploy_intent(intent).await;
    let mut solution = Solution::empty();
    solution.data = vec![SolutionData {
        intent_to_solve: intent_address.clone(),
        decision_variables: vec![DecisionVariable::Inline(42)],
    }];
    // State mutation to satisfy the intent.
    solution.state_mutations = vec![StateMutation {
        pathway: 0,
        mutations: vec![Mutation {
            key: [0, 0, 0, 0],
            value: Some(42),
        }],
    }];
    (solution, storage)
}
