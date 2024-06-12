pub mod empty;

use empty::Empty;
use essential_sign::sign;
use essential_types::{
    intent::{self, Directive, Intent},
    solution::{Mutation, Solution, SolutionData},
    IntentAddress, Signature, Signed, Word,
};
use secp256k1::{rand::rngs::OsRng, PublicKey, Secp256k1, SecretKey};
use serde::Serialize;

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

pub fn sign_intent_set_with_random_keypair(set: Vec<Intent>) -> intent::SignedSet {
    essential_sign::intent_set::sign(set, &random_keypair().0)
}

pub fn sign_with_random_keypair<T: Serialize>(data: T) -> Signed<T> {
    sign(data, &random_keypair().0)
}

pub fn sign_corrupted<T: Serialize>(data: T) -> Signed<T> {
    let mut signed = sign(data, &random_keypair().0);
    signed.signature = Signature([0u8; 64], 0);
    signed
}

pub fn solution_with_intent(intent_to_solve: IntentAddress) -> Solution {
    Solution {
        data: vec![SolutionData {
            intent_to_solve,
            decision_variables: Default::default(),
            state_mutations: Default::default(),
            transient_data: Default::default(),
        }],
    }
}

pub fn intent_with_salt(salt: Word) -> Intent {
    Intent {
        state_read: Default::default(),
        constraints: vec![essential_constraint_vm::asm::to_bytes(vec![
            essential_constraint_vm::asm::Stack::Push(salt).into(),
            essential_constraint_vm::asm::Stack::Pop.into(),
        ])
        .collect()],
        directive: Directive::Satisfy,
    }
}

pub fn intent_with_salt_and_state(salt: Word, key: Word) -> Intent {
    Intent {
        state_read: vec![essential_state_read_vm::asm::to_bytes(vec![
            essential_state_read_vm::asm::Stack::Push(1).into(),
            essential_state_read_vm::asm::StateSlots::AllocSlots.into(),
            essential_state_read_vm::asm::Stack::Push(key).into(),
            essential_state_read_vm::asm::Stack::Push(1).into(),
            essential_state_read_vm::asm::Stack::Push(1).into(),
            essential_state_read_vm::asm::Stack::Push(0).into(),
            essential_state_read_vm::asm::StateRead::KeyRange,
            essential_state_read_vm::asm::ControlFlow::Halt.into(),
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
            intent_to_solve: IntentAddress::empty(),
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
            intent_to_solve: IntentAddress::empty(),
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
            intent_to_solve: IntentAddress::empty(),
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
