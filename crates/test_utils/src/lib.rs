use essential_types::{
    intent::{Directive, Intent},
    slots::Slots,
    solution::Solution,
    Signed,
};
use secp256k1::{rand::rngs::OsRng, PublicKey, Secp256k1, SecretKey};

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

pub fn sign<T>(data: T) -> Signed<T> {
    Signed {
        data,
        signature: [0; 64],
    }
}

pub fn random_keypair() -> (SecretKey, PublicKey) {
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut OsRng)
}
