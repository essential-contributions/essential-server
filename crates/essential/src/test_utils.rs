use essential_types::{
    intent::{Directive, Intent},
    slots::Slots,
    solution::Solution,
};
use placeholder::Signed;

pub fn empty_intent() -> Intent {
    Intent {
        slots: Slots {
            decision_variables: 0,
            state: Default::default(),
            permits: Default::default(),
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
    }
}

pub fn sign<T>(data: T) -> Signed<T> {
    Signed {
        data,
        signature: todo!(),
        public_key: todo!(),
    }
}
