use crate::{
    solution::{check_solution_with_intents, read::read_intents_from_storage, Output},
    test_utils::{deploy_intent, sanity_solution, solution_with_deps},
};
use essential_state_read_vm::StateRead;
use essential_types::{intent::Intent, solution::Solution};
use std::sync::Arc;
use storage::{StateStorage, Storage};
use test_utils::{empty::Empty, solution_with_intent};

async fn check_solution<S>(storage: &S, solution: Arc<Solution>) -> anyhow::Result<Output<S>>
where
    S: Storage + StateStorage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    // Read intents from storage.
    let intents = read_intents_from_storage(&solution, storage).await?;
    check_solution_with_intents(storage, solution, &intents).await
}
#[tokio::test]
async fn test_check_empty_solution() {
    let (solution, storage) = sanity_solution().await;
    let _result = check_solution(&storage, Arc::new(solution)).await.unwrap();
    // TODO: result is 0 here because there are no state reads or constraints.
    // Should such solutions be unsatisfied or satisfied by default?
}

#[tokio::test]
async fn test_check_solution_with_deps() {
    let (solution, storage) = solution_with_deps().await;
    let result = check_solution(&storage, Arc::new(solution)).await.unwrap();
    assert_eq!(result.utility, 1.0);
}

#[tokio::test]
#[should_panic(expected = "State read VM execution failed: ")]
async fn test_check_solution_fail_state_read() {
    let mut intent = Intent::empty();
    // Program does not end with `asm::ControlFlow::Halt`
    intent.state_read = vec![essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(0).into(),
    ])
    .collect()];
    let (intent_address, storage) = deploy_intent(intent).await;
    let solution = solution_with_intent(intent_address);
    check_solution(&storage, Arc::new(solution)).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "Constraint VM execution failed: ")]
async fn test_check_solution_fail_constraint() {
    let mut intent = Intent::empty();
    intent.slots.state = vec![essential_types::slots::StateSlot {
        index: 0,
        amount: 1,
        program_index: 0,
    }];
    intent.state_read = vec![essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::Memory::Alloc.into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Memory::Push.into(),
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect()];
    // State slot out of bounds
    intent.constraints = vec![essential_constraint_vm::asm::to_bytes(vec![
        essential_constraint_vm::asm::Stack::Push(1).into(),
        essential_constraint_vm::asm::Stack::Push(0).into(),
        essential_constraint_vm::asm::Access::State.into(),
    ])
    .collect()];
    let (intent_address, storage) = deploy_intent(intent).await;
    let solution = solution_with_intent(intent_address);
    check_solution(&storage, Arc::new(solution)).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_directive_maximize() {
    todo!();
}
