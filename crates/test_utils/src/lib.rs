use essential_types::{
    intent::{Directive, Intent},
    slots::Slots,
    solution::Solution,
    Signed,
};

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
