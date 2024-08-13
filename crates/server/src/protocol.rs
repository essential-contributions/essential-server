use essential_types::{
    contract::Contract,
    predicate::Predicate,
    solution::{Mutation, Solution, SolutionData},
    ContentAddress, PredicateAddress,
};

/// # Storage
/// l1_block_number
/// { 0 => int }
/// l1_block_timestamp
/// { 1 => int }
pub(crate) fn block_state_contract() -> Contract {
    let predicates = vec![Predicate {
        state_read: vec![],
        constraints: vec![],
        directive: essential_types::predicate::Directive::Satisfy,
    }];

    let salt = essential_hash::hash(&"block-state-contract");
    Contract { predicates, salt }
}

pub(crate) fn block_state_contract_address() -> ContentAddress {
    let contract = block_state_contract();
    essential_hash::content_addr(&contract)
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

#[test]
#[ignore]
fn print_time_address() {
    let block_state_address = block_state_contract_address();
    let words = essential_types::convert::word_4_from_u8_32(block_state_address.0);

    println!("{:?}", block_state_address);
    println!("{}", block_state_address);
    println!("{:?}", words);
}
