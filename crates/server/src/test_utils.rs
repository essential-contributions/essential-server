use crate::deploy::deploy;
use essential_memory_storage::MemoryStorage;
use essential_state_read_vm::StateRead;
use essential_storage::Storage;
use essential_types::{
    intent::Intent,
    solution::{Mutation, Solution, SolutionData},
    ContentAddress, IntentAddress, Word,
};
use std::time::Duration;
use test_utils::{empty::Empty, sign_intent_set_with_random_keypair, solution_with_intent};

pub async fn run<S>(storage: &S) -> anyhow::Result<()>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    let shutdown = super::Shutdown(rx);
    let s = storage.clone();
    let jh =
        tokio::spawn(
            async move { crate::run::run(&s, shutdown, crate::run::RUN_LOOP_FREQUENCY).await },
        );
    tokio::time::sleep(Duration::from_millis(100)).await;
    tx.send(()).unwrap();
    jh.await?
}

// Empty valid solution.
// Sign an empty valid intent and deploy it to newly created memory storage,
// create a solution with the signed intent address.
pub async fn sanity_solution() -> (Solution, MemoryStorage) {
    let (intent_address, storage) = deploy_intent(Intent::empty()).await;
    let solution = solution_with_intent(intent_address);
    (solution, storage)
}

// Sign and deploy given intent to newly created memory storage.
pub async fn deploy_intent(intent: Intent) -> (IntentAddress, MemoryStorage) {
    deploy_intent_to_storage(MemoryStorage::default(), intent).await
}

// Sign and deploy given intent to newly created memory storage.
pub async fn deploy_intent_to_storage(
    storage: MemoryStorage,
    intent: Intent,
) -> (IntentAddress, MemoryStorage) {
    let intent_hash = ContentAddress(essential_hash::hash(&intent));
    let intent = sign_intent_set_with_random_keypair(vec![intent]);
    let result = deploy(&storage, intent).await.unwrap();
    (
        IntentAddress {
            set: result,
            intent: intent_hash,
        },
        storage.clone(),
    )
}

pub fn test_intent(salt: Word) -> Intent {
    // Intent that expects the value of previously unset state slot with index 0 to be 42.
    let mut intent = Intent::empty();
    // Program to read state slot 0.
    intent.state_read = vec![essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(4).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // delta
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRange,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect()];
    // Program to check pre-mutation value is None and
    // post-mutation value is 42 at slot 0.
    intent.constraints = vec![essential_constraint_vm::asm::to_bytes(vec![
        essential_constraint_vm::asm::Stack::Push(salt).into(), // Salt
        essential_constraint_vm::asm::Stack::Pop.into(),
        essential_constraint_vm::asm::Stack::Push(0).into(), // slot
        essential_constraint_vm::asm::Stack::Push(0).into(), // pre
        essential_constraint_vm::asm::Access::StateLen.into(),
        essential_constraint_vm::asm::Stack::Push(0).into(),
        essential_constraint_vm::asm::Pred::Eq.into(),
        essential_constraint_vm::asm::Stack::Push(0).into(), // slot
        essential_constraint_vm::asm::Stack::Push(1).into(), // post
        essential_constraint_vm::asm::Access::State.into(),
        essential_constraint_vm::asm::Stack::Push(42).into(),
        essential_constraint_vm::asm::Pred::Eq.into(),
        essential_constraint_vm::asm::Pred::And.into(),
    ])
    .collect()];
    intent
}

// Solution that satisfies an intent with state read and constraint programs.
pub async fn test_solution(
    storage: Option<MemoryStorage>,
    salt: Word,
) -> (Solution, MemoryStorage) {
    let (intent_address, storage) =
        deploy_intent_to_storage(storage.unwrap_or_default(), test_intent(salt)).await;
    let mut solution = Solution::empty();
    let solution_decision_variables = vec![vec![42]];
    solution.data = vec![SolutionData {
        intent_to_solve: intent_address.clone(),
        decision_variables: solution_decision_variables,
        state_mutations: vec![Mutation {
            key: vec![0, 0, 0, 0],
            value: vec![42],
        }],
        transient_data: Default::default(),
    }];
    (solution, storage)
}

pub fn counter_intent(salt: Word) -> Intent {
    let mut intent = Intent::empty();
    intent.state_read = vec![essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(4).into(), // key length
        essential_state_read_vm::asm::Stack::Push(1).into(), // delta
        essential_state_read_vm::asm::Stack::Push(0).into(), // slot index
        essential_state_read_vm::asm::StateRead::KeyRange,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect()];
    intent.constraints = vec![essential_constraint_vm::asm::to_bytes(vec![
        // Salt
        essential_constraint_vm::asm::Stack::Push(salt).into(),
        essential_constraint_vm::asm::Stack::Pop.into(),
        // Jump distance
        essential_constraint_vm::asm::Stack::Push(2).into(),
        // Check if the state is not empty
        essential_constraint_vm::asm::Stack::Push(0).into(),
        essential_constraint_vm::asm::Stack::Push(0).into(),
        essential_constraint_vm::asm::Access::StateLen.into(),
        essential_constraint_vm::asm::Stack::Push(0).into(),
        essential_constraint_vm::asm::Pred::Eq.into(),
        essential_constraint_vm::asm::Pred::Not.into(),
        // If not empty skip pushing 0
        essential_constraint_vm::asm::TotalControlFlow::JumpForwardIf.into(),
        essential_constraint_vm::asm::Stack::Push(0).into(),
        // Add 1 to the state or zero.
        // If state is empty then it won't push anything on the stack.
        essential_constraint_vm::asm::Stack::Push(0).into(),
        essential_constraint_vm::asm::Stack::Push(0).into(),
        essential_constraint_vm::asm::Access::State.into(),
        essential_constraint_vm::asm::Stack::Push(1).into(),
        essential_constraint_vm::asm::Alu::Add.into(),
        essential_constraint_vm::asm::Stack::Push(0).into(),
        essential_constraint_vm::asm::Stack::Push(1).into(),
        essential_constraint_vm::asm::Access::State.into(),
        essential_constraint_vm::asm::Pred::Eq.into(),
        // Check the final value matches the dec var
        essential_constraint_vm::asm::Stack::Push(0).into(),
        essential_constraint_vm::asm::Access::DecisionVar.into(),
        essential_constraint_vm::asm::Stack::Push(0).into(),
        essential_constraint_vm::asm::Stack::Push(1).into(),
        essential_constraint_vm::asm::Access::State.into(),
        essential_constraint_vm::asm::Pred::Eq.into(),
        essential_constraint_vm::asm::Pred::And.into(),
    ])
    .collect()];
    intent
}

pub async fn counter_solution(intent_address: IntentAddress, final_value: Word) -> Solution {
    let mut solution = Solution::empty();
    let solution_decision_variables = vec![vec![final_value]];
    solution.data = vec![SolutionData {
        intent_to_solve: intent_address.clone(),
        decision_variables: solution_decision_variables,
        state_mutations: vec![Mutation {
            key: vec![0, 0, 0, 0],
            value: vec![final_value],
        }],
        transient_data: Default::default(),
    }];
    solution
}
