use essential_types::{
    intent::{Directive, Intent},
    slots::StateSlot,
    solution::{DecisionVariable, PartialSolution, Solution, SolutionData, StateMutation},
    ContentAddress, IntentAddress, Word,
};

/// Utility trait to provide empty instantiaters for essential types
pub trait Empty {
    /// Create an empty instance of the type
    fn empty() -> Self;
}

impl Empty for Intent {
    fn empty() -> Self {
        Self {
            slots: Default::default(),
            state_read: Default::default(),
            constraints: Default::default(),
            directive: Directive::Satisfy,
        }
    }
}

impl Empty for StateSlot {
    fn empty() -> Self {
        Self {
            amount: Default::default(),
            index: Default::default(),
            program_index: Default::default(),
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
            state_mutations: Default::default(),
            partial_solutions: Default::default(),
        }
    }
}

impl Empty for SolutionData {
    fn empty() -> Self {
        Self {
            intent_to_solve: IntentAddress::empty(),
            decision_variables: Default::default(),
        }
    }
}

impl Empty for DecisionVariable {
    fn empty() -> Self {
        Self::Inline(0 as Word)
    }
}

impl Empty for StateMutation {
    fn empty() -> Self {
        Self {
            pathway: Default::default(),
            mutations: Default::default(),
        }
    }
}

impl Empty for PartialSolution {
    fn empty() -> Self {
        Self {
            data: Default::default(),
            state_mutations: Default::default(),
        }
    }
}
