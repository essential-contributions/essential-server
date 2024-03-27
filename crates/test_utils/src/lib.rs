use essential_types::{
    intent::{Directive, Intent},
    slots::Slots,
    solution::{PartialSolution, Solution},
    Signed,
};
use secp256k1::{rand::rngs::OsRng, PublicKey, Secp256k1, SecretKey};
use serde::Serialize;
use utils::sign;

pub fn empty_intent() -> Intent {
    Intent {
        slots: Slots {
            decision_variables: 0,
            state: Default::default(),
        },
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    }
}

pub fn intent_with_vars(decision_variables: u32) -> Intent {
    Intent {
        slots: Slots {
            decision_variables,
            state: Default::default(),
        },
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    }
}

pub fn empty_solution() -> Solution {
    Solution {
        data: Default::default(),
        state_mutations: Default::default(),
        partial_solutions: Default::default(),
    }
}

pub fn empty_partial_solution() -> PartialSolution {
    PartialSolution {
        data: Default::default(),
        state_mutations: Default::default(),
    }
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
