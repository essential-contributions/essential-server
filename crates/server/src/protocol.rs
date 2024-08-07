use std::{ops::Bound, slice::SliceIndex, sync::Arc};

use essential_check::solution::{check_predicates, CheckPredicateConfig};
use essential_constraint_vm::asm as c_asm;
use essential_state_read_vm::{asm as s_asm, StateRead};
use essential_storage::{StateStorage, Storage};
use essential_transaction_storage::{Transaction, TransactionStorage};
use essential_types::{
    contract::Contract,
    convert::word_4_from_u8_32,
    predicate::Predicate,
    solution::{Mutation, Solution, SolutionData},
    Block, PredicateAddress,
};

struct ValidationCheck {
    contract: Contract,
    pre_state: Option<usize>,
    post_state: Option<usize>,
}

/// # Storage
/// l1_block_number
/// { 0 => int }
/// l1_block_timestamp
/// { 1 => int }
///
/// # Constraints
/// constraint __mut_keys_contains(0);
/// constraint __mut_keys_contains(1);
/// constraint __mut_keys_len() == 2;
pub(crate) fn block_state_contract() -> Contract {
    let constraints = vec![
        c_asm::Stack::Push(0).into(),
        c_asm::Stack::Push(1).into(),
        c_asm::Access::MutKeysContains.into(),
        c_asm::Stack::Push(1).into(),
        c_asm::Stack::Push(1).into(),
        c_asm::Access::MutKeysContains.into(),
        c_asm::Pred::And.into(),
        c_asm::Access::MutKeysLen.into(),
        c_asm::Stack::Push(2).into(),
        c_asm::Pred::Eq.into(),
        c_asm::Pred::And.into(),
    ];
    let predicates = vec![Predicate {
        state_read: vec![],
        constraints: vec![c_asm::to_bytes(constraints).collect()],
        directive: essential_types::predicate::Directive::Satisfy,
    }];
    // Maybe random salt
    Contract {
        predicates,
        salt: Default::default(),
    }
}

/// # Types
/// type L1Block = {
///   number: int,
///   timestamp: int,
/// };
///
/// # State read
/// state l1_block_number = block_contract::l1_block_number;
/// state l1_block_timestamp = block_contract::l1_block_number;
///
/// # Dec vars
/// var l1_block: L1Block;
///
/// # Constraints
/// constraint l1_block_number' == l1_block.number;
/// constraint l1_block_number == l1_block_number';
/// constraint l1_block_timestamp' == l1_block.timestamp;
/// constraint l1_block_timestamp == l1_block_timestamp';
fn block_validation() -> ValidationCheck {
    let block_state_contract_address =
        essential_hash::contract_addr::from_contract(&block_state_contract());
    let block_state_contract_address: Vec<s_asm::Op> =
        word_4_from_u8_32(block_state_contract_address.0)
            .into_iter()
            .map(s_asm::Stack::Push)
            .map(Into::into)
            .collect();

    let mut read_l1_block_number = vec![
        s_asm::Stack::Push(2).into(),
        s_asm::StateSlots::AllocSlots.into(),
    ];
    read_l1_block_number.extend(&block_state_contract_address);
    read_l1_block_number.extend(&[
        s_asm::Stack::Push(0).into(),
        s_asm::Stack::Push(1).into(),
        s_asm::Stack::Push(1).into(),
        s_asm::Stack::Push(0).into(),
        s_asm::StateRead::KeyRangeExtern,
    ]);
    read_l1_block_number.extend(&block_state_contract_address);
    read_l1_block_number.extend(&[
        s_asm::Stack::Push(1).into(),
        s_asm::Stack::Push(1).into(),
        s_asm::Stack::Push(1).into(),
        s_asm::Stack::Push(1).into(),
        s_asm::StateRead::KeyRangeExtern,
        s_asm::TotalControlFlow::Halt.into(),
    ]);

    let state_read = vec![s_asm::to_bytes(read_l1_block_number).collect()];

    let constraints = vec![
        c_asm::Stack::Push(0).into(),
        c_asm::Stack::Push(1).into(),
        c_asm::Access::State.into(),
        c_asm::Stack::Push(0).into(),
        c_asm::Stack::Push(0).into(),
        c_asm::Access::DecisionVarAt.into(),
        c_asm::Pred::Eq.into(),
        c_asm::Stack::Push(0).into(),
        c_asm::Stack::Push(0).into(),
        c_asm::Access::State.into(),
        c_asm::Stack::Push(0).into(),
        c_asm::Stack::Push(1).into(),
        c_asm::Access::State.into(),
        c_asm::Pred::Eq.into(),
        c_asm::Pred::And.into(),
        c_asm::Stack::Push(1).into(),
        c_asm::Stack::Push(1).into(),
        c_asm::Access::State.into(),
        c_asm::Stack::Push(0).into(),
        c_asm::Stack::Push(1).into(),
        c_asm::Access::DecisionVarAt.into(),
        c_asm::Pred::Eq.into(),
        c_asm::Pred::And.into(),
        c_asm::Stack::Push(1).into(),
        c_asm::Stack::Push(0).into(),
        c_asm::Access::State.into(),
        c_asm::Stack::Push(1).into(),
        c_asm::Stack::Push(1).into(),
        c_asm::Access::State.into(),
        c_asm::Pred::Eq.into(),
        c_asm::Pred::And.into(),
    ];

    let constraints = vec![c_asm::to_bytes(constraints).collect()];
    let predicate = Predicate {
        state_read,
        constraints,
        directive: essential_types::predicate::Directive::Satisfy,
    };
    let contract = Contract {
        predicates: vec![predicate],
        salt: Default::default(),
    };
    ValidationCheck {
        contract,
        pre_state: Some(1),
        post_state: None,
    }
}

fn block_validation_solution(l1_block_number: u64, l1_block_timestamp: u64) -> Solution {
    let contract = block_validation().contract;
    let block_validation_address = essential_hash::contract_addr::from_contract(&contract);
    let predicate = essential_hash::content_addr(&contract.predicates[0]);
    let predicate_to_solve = PredicateAddress {
        contract: block_validation_address,
        predicate,
    };
    let decision_variables = vec![vec![
        l1_block_number.try_into().unwrap(),
        l1_block_timestamp.try_into().unwrap(),
    ]];
    Solution {
        data: vec![SolutionData {
            predicate_to_solve,
            decision_variables,
            transient_data: Default::default(),
            state_mutations: Default::default(),
        }],
    }
}

pub(crate) fn block_state_solution(l1_block_number: u64, l1_block_timestamp: u64) -> Solution {
    let contract = block_state_contract();
    let block_state_address = essential_hash::contract_addr::from_contract(&contract);
    let predicate = essential_hash::content_addr(&contract.predicates[0]);
    let predicate_to_solve = PredicateAddress {
        contract: block_state_address,
        predicate,
    };
    let l1_block_number = l1_block_number.try_into().unwrap();
    let l1_block_timestamp = l1_block_timestamp.try_into().unwrap();
    Solution {
        data: vec![SolutionData {
            predicate_to_solve,
            decision_variables: Default::default(),
            transient_data: Default::default(),
            state_mutations: vec![
                Mutation {
                    key: vec![0],
                    value: vec![l1_block_number],
                },
                Mutation {
                    key: vec![1],
                    value: vec![l1_block_timestamp],
                },
            ],
        }],
    }
}

pub(crate) async fn validate<S>(
    storage: &S,
    block: &Block,
    config: Arc<CheckPredicateConfig>,
) -> anyhow::Result<bool>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
{
    let l1_block_number = block.number;
    let l1_block_timestamp = block.timestamp.as_secs();
    let solution = block_validation_solution(l1_block_number, l1_block_timestamp);

    let ValidationCheck {
        contract,
        pre_state,
        post_state,
    } = block_validation();
    let pre_state_range = 0..pre_state.unwrap_or_default();
    let post_state_range = (
        Bound::Included(pre_state.unwrap_or_default()),
        post_state.map_or(Bound::Unbounded, Bound::Excluded),
    );
    let mut pre_state = storage.clone().transaction();
    create_state(&mut pre_state, block, pre_state_range);
    let mut post_state = pre_state.clone();
    create_state(&mut post_state, block, post_state_range);
    let predicate = Arc::new(contract.predicates.into_iter().next().unwrap());

    let (utility, _) = check_predicates(
        &pre_state.view(),
        &post_state.view(),
        Arc::new(solution),
        |_| predicate.clone(),
        config,
    )
    .await?;
    Ok(utility > 0.0)
}

fn create_state<S, R>(storage: &mut TransactionStorage<S>, block: &Block, range: R)
where
    S: StateStorage,
    R: SliceIndex<[Solution], Output = [Solution]>,
{
    for solution in block.solutions.get(range).iter().flat_map(|s| s.iter()) {
        crate::solution::apply_mutations(storage, solution);
    }
}

#[test]
#[ignore]
fn print_time_address() {
    let contract = block_state_contract();
    let block_state_address = essential_hash::contract_addr::from_contract(&contract);
    let words = word_4_from_u8_32(block_state_address.0);

    println!("{:?}", block_state_address);
    println!("{}", block_state_address);
    println!("{:?}", words);
    
}