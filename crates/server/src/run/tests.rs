use crate::{
    deploy::deploy,
    solution::submit_solution,
    test_utils::{counter_intent, counter_solution, deploy_intent, test_solution},
};
use essential_memory_storage::MemoryStorage;
use essential_state_read_vm::StateRead;
use essential_storage::{QueryState, Storage};
use essential_types::{intent::Intent, ContentAddress, IntentAddress, Word};
use std::time::Duration;
use test_utils::{empty::Empty, sign_intent_set_with_random_keypair};

use super::RUN_LOOP_FREQUENCY;

async fn run<S>(storage: &S) -> anyhow::Result<()>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    let shutdown = super::Shutdown(rx);
    let s = storage.clone();
    let jh = tokio::spawn(async move { super::run(&s, shutdown, RUN_LOOP_FREQUENCY).await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    tx.send(()).unwrap();
    jh.await?
}

#[tokio::test]
async fn test_run() {
    let (solution, storage) = test_solution(None, 1).await;

    let first_state_mutation = &solution.data[0].state_mutations[0];
    let mutation_key = first_state_mutation.key.clone();
    let mutation_address = solution.data[0].intent_to_solve.set.clone();

    submit_solution(&storage, solution.clone()).await.unwrap();

    let pre_state = storage
        .query_state(&mutation_address, &mutation_key)
        .await
        .unwrap();
    assert!(pre_state.is_empty());

    run(&storage).await.unwrap();

    let post_state = storage
        .query_state(&mutation_address, &mutation_key)
        .await
        .unwrap();
    assert_eq!(post_state, vec![42]);

    let blocks = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].batch.solutions.len(), 1);
    assert_eq!(blocks[0].batch.solutions[0], solution);

    let solution2 = solution; // same as solution
    let (solution3, _) = test_solution(Some(storage.clone()), 2).await;

    submit_solution(&storage, solution2).await.unwrap();
    submit_solution(&storage, solution3.clone()).await.unwrap();

    run(&storage).await.unwrap();

    let blocks = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1].batch.solutions.len(), 1);
    assert!(blocks[1].batch.solutions.iter().any(|s| s == &solution3));
}

#[tokio::test]
async fn test_counter() {
    let intent = counter_intent(1);
    let (intent_address, storage) = deploy_intent(intent.clone()).await;

    let solution = counter_solution(intent_address.clone(), 1).await;
    let solution2 = counter_solution(intent_address.clone(), 2).await;
    let solution3 = counter_solution(intent_address.clone(), 3).await;
    let solution4 = counter_solution(intent_address.clone(), 4).await;

    let mutation_key = solution.data[0].state_mutations[0].key.clone();

    submit_solution(&storage, solution.clone()).await.unwrap();
    submit_solution(&storage, solution.clone()).await.unwrap();
    submit_solution(&storage, solution2.clone()).await.unwrap();
    submit_solution(&storage, solution4.clone()).await.unwrap();

    let pre_state = storage
        .query_state(&intent_address.set, &mutation_key)
        .await
        .unwrap();
    assert!(pre_state.is_empty());

    run(&storage).await.unwrap();

    let post_state = storage
        .query_state(&intent_address.set, &mutation_key)
        .await
        .unwrap();
    assert_eq!(post_state, vec![2]);

    let blocks = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].batch.solutions.len(), 2);
    let solutions = &blocks[0].batch.solutions;
    assert!(solutions.contains(&solution));
    assert!(solutions.contains(&solution2));

    submit_solution(&storage, solution3.clone()).await.unwrap();
    submit_solution(&storage, solution4.clone()).await.unwrap();

    run(&storage).await.unwrap();

    let post_state = storage
        .query_state(&intent_address.set, &mutation_key)
        .await
        .unwrap();
    assert_eq!(post_state, vec![4]);

    let blocks = storage.list_winning_blocks(None, None).await.unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1].batch.solutions.len(), 2);
    let solutions = &blocks[1].batch.solutions;
    assert!(solutions.contains(&solution3));
    assert!(solutions.contains(&solution4));
}

fn state_read_error_intent(salt: Word) -> Intent {
    let mut intent = Intent::empty();
    intent.state_read = vec![essential_state_read_vm::asm::to_bytes(vec![
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::Stack::Push(4).into(),
        essential_state_read_vm::asm::Stack::Push(1).into(),
        essential_state_read_vm::asm::Stack::Push(0).into(),
        essential_state_read_vm::asm::StateRead::KeyRange,
        essential_state_read_vm::asm::ControlFlow::Halt.into(),
    ])
    .collect()];
    intent.constraints = vec![essential_constraint_vm::asm::to_bytes(vec![
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
    ])
    .collect()];
    intent
}

#[tokio::test]
async fn test_tracing() {
    std::env::set_var("RUST_LOG", "trace");
    #[cfg(feature = "tracing")]
    let _ = tracing_subscriber::fmt::try_init();
    let intent: Intent = state_read_error_intent(1);

    let storage = MemoryStorage::default();
    let intent_hash = ContentAddress(essential_hash::hash(&intent));
    let set = sign_intent_set_with_random_keypair(vec![intent]);
    let result = deploy(&storage, set).await.unwrap();
    let intent_address = IntentAddress {
        set: result,
        intent: intent_hash,
    };
    let solution = counter_solution(intent_address.clone(), 1).await;
    submit_solution(&storage, solution.clone()).await.unwrap();
    run(&storage).await.unwrap();
}
