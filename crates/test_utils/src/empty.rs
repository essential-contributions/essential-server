use essential_types::{
    intent::{Directive, Intent},
    solution::{PartialSolution, Solution},
    ContentAddress, IntentAddress,
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

impl Empty<PartialSolution> for PartialSolution {
    fn empty() -> PartialSolution {
        PartialSolution {
            data: Default::default(),
            state_mutations: Default::default(),
        }
    }
}
