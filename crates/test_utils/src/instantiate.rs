use crate::empty::Empty;
use essential_types::{
    intent::{Directive, Intent},
    slots::Slots,
    solution::{DecisionVariable, PartialSolution, PartialSolutionData, Solution, SolutionData},
    IntentAddress, Word,
};

/// Utility trait to provide common Instantiaters for essential types
pub trait Instantiate<T>: Empty<T> {
    fn with_decision_variables(decision_variables: usize) -> T;
}

impl Instantiate<Intent> for Intent {
    fn with_decision_variables(decision_variables: usize) -> Intent {
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
}

impl Instantiate<PartialSolution> for PartialSolution {
    fn with_decision_variables(decision_variables: usize) -> PartialSolution {
        PartialSolution {
            data: vec![PartialSolutionData {
                intent_to_solve: IntentAddress::empty(),
                decision_variables: vec![
                    Some(DecisionVariable::Inline(decision_variables as Word));
                    decision_variables
                ],
            }],
            state_mutations: Default::default(),
        }
    }
}

impl Instantiate<Solution> for Solution {
    fn with_decision_variables(decision_variables: usize) -> Solution {
        Solution {
            data: vec![SolutionData {
                intent_to_solve: IntentAddress::empty(),
                decision_variables: vec![
                    DecisionVariable::Inline(decision_variables as Word);
                    decision_variables
                ],
            }],
            state_mutations: Default::default(),
            partial_solutions: Default::default(),
        }
    }
}
