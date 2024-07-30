pub mod empty;

use empty::Empty;
use essential_types::{
    contract::{Contract, SignedContract},
    predicate::{Directive, Predicate},
    solution::{Mutation, Solution, SolutionData},
    PredicateAddress, Word,
};
use secp256k1::{rand::rngs::OsRng, PublicKey, Secp256k1, SecretKey};

pub fn duration_secs(secs: u64) -> std::time::Duration {
    std::time::Duration::from_secs(secs)
}

pub fn random_keypair() -> (SecretKey, PublicKey) {
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut OsRng)
}

pub fn keypair(key: [u8; 32]) -> (SecretKey, PublicKey) {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&key).unwrap();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    (secret_key, public_key)
}

pub fn sign_contract_with_random_keypair(contract: impl Into<Contract>) -> SignedContract {
    essential_sign::contract::sign(contract.into(), &random_keypair().0)
}

pub fn solution_with_predicate(predicate_to_solve: PredicateAddress) -> Solution {
    Solution {
        data: vec![SolutionData {
            predicate_to_solve,
            decision_variables: Default::default(),
            state_mutations: Default::default(),
            transient_data: Default::default(),
        }],
    }
}

pub fn predicate_with_salt(salt: Word) -> Predicate {
    Predicate {
        state_read: Default::default(),
        constraints: vec![essential_constraint_vm::asm::to_bytes(vec![
            essential_constraint_vm::asm::Stack::Push(salt).into(),
            essential_constraint_vm::asm::Stack::Pop.into(),
        ])
        .collect()],
        directive: Directive::Satisfy,
    }
}

pub fn predicate_with_salt_and_state(salt: Word, key: Word) -> Predicate {
    Predicate {
        state_read: vec![essential_state_read_vm::asm::to_bytes(vec![
            essential_state_read_vm::asm::Stack::Push(1).into(),
            essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
            essential_state_read_vm::asm::Stack::Push(key).into(),
            essential_state_read_vm::asm::Stack::Push(1).into(),
            essential_state_read_vm::asm::Stack::Push(1).into(),
            essential_state_read_vm::asm::Stack::Push(0).into(),
            essential_state_read_vm::asm::StateRead::KeyRange,
            essential_state_read_vm::asm::TotalControlFlow::Halt.into(),
        ])
        .collect()],
        constraints: vec![essential_constraint_vm::asm::to_bytes(vec![
            essential_constraint_vm::asm::Stack::Push(salt).into(),
            essential_constraint_vm::asm::Stack::Pop.into(),
        ])
        .collect()],
        directive: Directive::Satisfy,
    }
}

pub fn solution_with_decision_variables(decision_variables: usize) -> Solution {
    Solution {
        data: vec![SolutionData {
            predicate_to_solve: PredicateAddress::empty(),
            decision_variables: vec![vec![decision_variables as Word; decision_variables]],
            state_mutations: Default::default(),
            transient_data: Default::default(),
        }],
    }
}

pub fn solution_with_all_inputs(i: usize) -> Solution {
    let input = vec![i as Word; i];
    Solution {
        data: vec![SolutionData {
            predicate_to_solve: PredicateAddress::empty(),
            decision_variables: vec![input.clone()],
            state_mutations: vec![
                Mutation {
                    key: input.clone(),
                    value: input.clone()
                };
                i
            ],
            transient_data: vec![
                Mutation {
                    key: input.clone(),
                    value: input.clone()
                };
                i
            ],
        }],
    }
}

pub fn solution_with_all_inputs_fixed_size(i: usize, size: usize) -> Solution {
    let input = vec![i as Word; size];
    Solution {
        data: vec![SolutionData {
            predicate_to_solve: PredicateAddress::empty(),
            decision_variables: vec![input.clone()],
            state_mutations: vec![
                Mutation {
                    key: input.clone(),
                    value: input.clone()
                };
                size
            ],
            transient_data: vec![
                Mutation {
                    key: input.clone(),
                    value: input.clone()
                };
                size
            ],
        }],
    }
}
