use essential_types::{
    intent::{Directive, Intent},
    slots::StateSlot,
    solution::{DecisionVariable, PartialSolution, Solution, SolutionData, StateMutation},
    ContentAddress, IntentAddress, Word,
};

/// Utility trait to provide empty Instantiaters for essential types
pub trait Empty<T> {
    fn empty() -> T;
}

impl Empty<Intent> for Intent {
    fn empty() -> Intent {
        Intent {
            slots: Default::default(),
            state_read: Default::default(),
            constraints: Default::default(),
            directive: Directive::Satisfy,
        }
    }
}

impl Empty<StateSlot> for StateSlot {
    fn empty() -> StateSlot {
        StateSlot {
            amount: Default::default(),
            index: Default::default(),
            program_index: Default::default(),
        }
    }
}

impl Empty<ContentAddress> for ContentAddress {
    fn empty() -> ContentAddress {
        ContentAddress([0; 32])
    }
}

impl Empty<IntentAddress> for IntentAddress {
    fn empty() -> IntentAddress {
        IntentAddress {
            set: ContentAddress::empty(),
            intent: ContentAddress::empty(),
        }
    }
}

impl Empty<Solution> for Solution {
    fn empty() -> Solution {
        Solution {
            data: Default::default(),
            state_mutations: Default::default(),
            partial_solutions: Default::default(),
        }
    }
}

impl Empty<SolutionData> for SolutionData {
    fn empty() -> SolutionData {
        SolutionData {
            intent_to_solve: IntentAddress::empty(),
            decision_variables: Default::default(),
        }
    }
}

impl Empty<DecisionVariable> for DecisionVariable {
    fn empty() -> DecisionVariable {
        DecisionVariable::Inline(0 as Word)
    }
}

impl Empty<StateMutation> for StateMutation {
    fn empty() -> StateMutation {
        StateMutation {
            pathway: Default::default(),
            mutations: Default::default(),
        }
    }
}

impl Empty<PartialSolution> for PartialSolution {
    fn empty() -> PartialSolution {
        PartialSolution {
            data: Default::default(),
            state_mutations: Default::default(),
        }
    }
}
