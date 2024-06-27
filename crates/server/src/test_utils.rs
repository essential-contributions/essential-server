use crate::deploy::deploy;
use essential_memory_storage::MemoryStorage;
use essential_types::{
    predicate::Predicate,
    solution::{Mutation, Solution, SolutionData},
    ContentAddress, PredicateAddress, Word,
};
use test_utils::{empty::Empty, sign_contract_with_random_keypair, solution_with_predicate};

// Empty valid solution.
// Sign an empty valid predicate and deploy it to newly created memory storage,
// create a solution with the signed predicate address.
pub async fn sanity_solution() -> (Solution, MemoryStorage) {
    let (predicate_address, storage) = deploy_predicate(Predicate::empty()).await;
    let solution = solution_with_predicate(predicate_address);
    (solution, storage)
}

// Sign and deploy given predicate to newly created memory storage.
pub async fn deploy_predicate(predicate: Predicate) -> (PredicateAddress, MemoryStorage) {
    deploy_predicate_to_storage(MemoryStorage::default(), predicate).await
}

pub async fn deploy_contracts(
    contracts: Vec<Contract>,
) -> (Vec<Vec<PredicateAddress>>, MemoryStorage) {
    let mut s = MemoryStorage::default();
    let mut addresses = Vec::new();
    for contract in contracts {
        let (addr, s2) = deploy_contract_to_storage(s, contract).await;
        s = s2;
        addresses.push(addr);
    }
    (addresses, s)
}

// Sign and deploy given predicate to newly created memory storage.
pub async fn deploy_predicate_to_storage(
    storage: MemoryStorage,
    predicate: Predicate,
) -> (PredicateAddress, MemoryStorage) {
    let predicate_hash = ContentAddress(essential_hash::hash(&predicate));
    let predicate = sign_contract_with_random_keypair(vec![predicate]);
    let result = deploy(&storage, predicate).await.unwrap();
    (
        PredicateAddress {
            contract: result,
            predicate: predicate_hash,
        },
        storage,
    )
}

pub async fn deploy_contract_to_storage(
    storage: MemoryStorage,
    contract: Contract,
) -> (Vec<PredicateAddress>, MemoryStorage) {
    let contract_hash = essential_hash::contract_addr::from_contract(&contract);
    let addresses = contract
        .iter()
        .map(|predicate| PredicateAddress {
            contract: contract_hash.clone(),
            predicate: essential_hash::content_addr(predicate),
        })
        .collect();
    let contract = sign_contract_with_random_keypair(contract);
    deploy(&storage, contract).await.unwrap();
    (addresses, storage)
}

pub fn test_predicate(salt: Word) -> Predicate {
    // Predicate that expects the value of previously uncontract state slot with index 0 to be 42.
    let mut predicate = Predicate::empty();
    // Program to read state slot 0.
    predicate.state_read = vec![essential_state_read_vm::asm::to_bytes(vec![
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
    predicate.constraints = vec![essential_constraint_vm::asm::to_bytes(vec![
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
    predicate
}

// Solution that satisfies an predicate with state read and constraint programs.
pub async fn test_solution(
    storage: Option<MemoryStorage>,
    salt: Word,
) -> (Solution, MemoryStorage) {
    let (predicate_address, storage) =
        deploy_predicate_to_storage(storage.unwrap_or_default(), test_predicate(salt)).await;
    let mut solution = Solution::empty();
    let solution_decision_variables = vec![vec![42]];
    solution.data = vec![SolutionData {
        predicate_to_solve: predicate_address.clone(),
        decision_variables: solution_decision_variables,
        state_mutations: vec![Mutation {
            key: vec![0, 0, 0, 0],
            value: vec![42],
        }],
        transient_data: Default::default(),
    }];
    (solution, storage)
}

pub fn counter_predicate(salt: Word) -> Predicate {
    let mut predicate = Predicate::empty();
    predicate.state_read = vec![essential_state_read_vm::asm::to_bytes(vec![
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
    predicate.constraints = vec![essential_constraint_vm::asm::to_bytes(vec![
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
    predicate
}

pub async fn counter_solution(predicate_address: PredicateAddress, final_value: Word) -> Solution {
    let mut solution = Solution::empty();
    let solution_decision_variables = vec![vec![final_value]];
    solution.data = vec![SolutionData {
        predicate_to_solve: predicate_address.clone(),
        decision_variables: solution_decision_variables,
        state_mutations: vec![Mutation {
            key: vec![0, 0, 0, 0],
            value: vec![final_value],
        }],
        transient_data: Default::default(),
    }];
    solution
}
