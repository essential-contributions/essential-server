use crate::instantiate::Instantiate;
use essential_types::{
    solution::{Solution, SolutionData},
    IntentAddress,
};

/// Utility trait to provide various other Instantiaters for Solution type
pub trait InstantiateSolution<Solution>: Instantiate<Solution> {
    fn with_intent(intent_to_solve: IntentAddress) -> Solution;
}

impl InstantiateSolution<Solution> for Solution {
    fn with_intent(intent_to_solve: IntentAddress) -> Solution {
        Solution {
            data: vec![SolutionData {
                intent_to_solve,
                decision_variables: Default::default(),
            }],
            state_mutations: Default::default(),
            partial_solutions: Default::default(),
        }
    }
}
