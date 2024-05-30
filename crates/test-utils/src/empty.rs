use essential_types::{
    intent::{Directive, Intent},
    solution::{Solution, SolutionData},
    ContentAddress, IntentAddress,
};

/// Utility trait to provide empty instantiaters for essential types
pub trait Empty {
    /// Create an empty instance of the type
    fn empty() -> Self;
}

impl Empty for Intent {
    fn empty() -> Self {
        Self {
            state_read: Default::default(),
            constraints: Default::default(),
            directive: Directive::Satisfy,
        }
    }
}

impl Empty for ContentAddress {
    fn empty() -> Self {
        Self([0; 32])
    }
}

impl Empty for IntentAddress {
    fn empty() -> Self {
        Self {
            set: ContentAddress::empty(),
            intent: ContentAddress::empty(),
        }
    }
}

impl Empty for Solution {
    fn empty() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl Empty for SolutionData {
    fn empty() -> Self {
        Self {
            intent_to_solve: IntentAddress::empty(),
            decision_variables: Default::default(),
            transient_data: Default::default(),
            state_mutations: Default::default(),
        }
    }
}
