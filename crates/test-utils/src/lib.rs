pub mod empty;

use empty::Empty;
use essential_sign::sign;
use essential_types::{
    intent::{Directive, Intent},
    slots::Slots,
    solution::{DecisionVariable, Solution, SolutionData},
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

pub fn sign_with_random_keypair<T: Serialize>(data: T) -> Signed<T> {
    sign(data, random_keypair().0)
}

pub fn sign_corrupted<T: Serialize>(data: T) -> Signed<T> {
    let mut signed = sign(data, random_keypair().0);
    signed.signature = Signature([0u8; 64], 0);
    signed
}

pub fn solution_with_intent(intent_to_solve: IntentAddress) -> Solution {
    Solution {
        data: vec![SolutionData {
            intent_to_solve,
            decision_variables: Default::default(),
        }],
        state_mutations: Default::default(),
    }
}

pub fn intent_with_decision_variables(decision_variables: usize) -> Intent {
    Intent {
        slots: Slots {
            decision_variables: decision_variables as u32,
            state: Default::default(),
        },
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    }
}

pub fn solution_with_decision_variables(decision_variables: usize) -> Solution {
    Solution {
        data: vec![SolutionData {
            intent_to_solve: IntentAddress::empty(),
            decision_variables: vec![
                DecisionVariable::Inline(decision_variables as Word);
                decision_variables
            ],
        }],
        state_mutations: Default::default(),
    }
}
