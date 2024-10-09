use essential_types::{
    predicate::Predicate,
    solution::{Solution, SolutionData},
    ContentAddress, PredicateAddress,
};

/// Utility trait to provide empty instantiaters for essential types
pub trait Empty {
    /// Create an empty instance of the type
    fn empty() -> Self;
}

impl Empty for Predicate {
    fn empty() -> Self {
        Self {
            state_read: Default::default(),
            constraints: Default::default(),
        }
    }
}

impl Empty for ContentAddress {
    fn empty() -> Self {
        Self([0; 32])
    }
}

impl Empty for PredicateAddress {
    fn empty() -> Self {
        Self {
            contract: ContentAddress::empty(),
            predicate: ContentAddress::empty(),
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
            predicate_to_solve: PredicateAddress::empty(),
            decision_variables: Default::default(),
            transient_data: Default::default(),
            state_mutations: Default::default(),
        }
    }
}
