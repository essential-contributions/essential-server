use anyhow::ensure;
use essential_types::{solution::Solution, Signed};
use serde::Serialize;
use storage::Storage;
use utils::verify;

#[cfg(test)]
mod tests;

/// Trait for validating essential types.
/// Validation of these types is necessary to ensure invalid data does not reach the storage.
pub trait Validate {
    fn validate(&self) -> anyhow::Result<()>;
}

/// Validation for a signed data.
/// Verifies the signature and then validates the signed data.
impl<T: Clone + Validate + Serialize> Validate for Signed<T> {
    fn validate(&self) -> anyhow::Result<()> {
        ensure!(verify(self.clone()), "Failed to verify signature");
        self.data.validate()
    }
}

/// Trait for solution validation that requires reading from storage.
pub trait ValidateWithStorage<S: Storage> {
    async fn validate_with_storage(&self, storage: &S, solution: Solution) -> anyhow::Result<()>;
}

pub mod slots {
    use super::*;
    use essential_types::{
        slots::{state_len, Slots},
        solution::SolutionData,
    };

    /// Maximum number of decision variables of the slots of an intent or solution.
    pub const MAX_DECISION_VARIABLES: u32 = 100;
    /// Maximum number of state slots of an intent.
    pub const MAX_NUM_STATE_SLOTS: usize = 1000;
    /// Maximum length of state slots of an intent.
    pub const MAX_STATE_LEN: u32 = 101;

    /// Validation for slots.
    impl Validate for Slots {
        fn validate(&self) -> anyhow::Result<()> {
            ensure!(
                self.decision_variables <= MAX_DECISION_VARIABLES,
                "Too many decision variables"
            );
            ensure!(
                self.state.len() <= MAX_NUM_STATE_SLOTS,
                "Too many state slots"
            );
            ensure!(
                state_len(&self.state).is_some(),
                "Invalid slots state length"
            );
            ensure!(
                state_len(&self.state).unwrap() <= MAX_STATE_LEN,
                "Slots state length too large"
            );
            Ok(())
        }
    }

    /// Validation for slots against solution data.
    impl Validate for (SolutionData, Slots) {
        fn validate(&self) -> anyhow::Result<()> {
            let (data, slots) = self;
            ensure!(
                data.decision_variables.len() as u32 == slots.decision_variables,
                "Decision variables mismatch"
            );
            Ok(())
        }
    }
}

pub mod intent {
    use super::*;
    use anyhow::ensure;
    use essential_types::intent::{Directive, Intent};

    /// Maximum number of intents that of an intent set.
    pub const MAX_INTENTS: usize = 99;
    /// Maximum number of state read programs of an intent.
    pub const MAX_STATE_READS: usize = 102;
    /// Maximum size of state read programs of an intent.
    pub const MAX_STATE_READ_SIZE: usize = 1001;
    /// Maximum number of constraint check programs of an intent.
    pub const MAX_CONSTRAINTS: usize = 98;
    /// Maximum size of constraint check programs of an intent.
    pub const MAX_CONSTRAINT_SIZE: usize = 999;
    /// Maximum size of directive of an intent.
    pub const MAX_DIRECTIVE_SIZE: usize = 1002;

    /// Validation for a set of intents.
    /// Checks the size of the set and then validates each intent.
    impl Validate for Vec<Intent> {
        fn validate(&self) -> anyhow::Result<()> {
            ensure!(self.len() <= MAX_INTENTS, "Too many intents");
            for i in self {
                i.validate()?;
            }
            Ok(())
        }
    }

    /// Validation for a single intent.
    /// Validates the slots, directive, state reads, and constraints.
    impl Validate for Intent {
        fn validate(&self) -> anyhow::Result<()> {
            self.slots.validate()?;
            self.directive.validate()?;
            // TODO: impl Validate for Vec<StateReadBytecode> and Vec<ConstraintBytecode> are
            // not possible because of conflicting implementations of Vec<Vec<u8>>
            // consider changing them to 1-tuple structs
            ensure!(
                self.state_read.len() <= MAX_STATE_READS,
                "Too many state reads"
            );
            ensure!(
                self.state_read
                    .iter()
                    .all(|sr| sr.len() <= MAX_STATE_READ_SIZE),
                "State read too large"
            );
            ensure!(
                self.constraints.len() <= MAX_CONSTRAINTS,
                "Too many constraints"
            );
            ensure!(
                self.constraints
                    .iter()
                    .all(|c| c.len() <= MAX_CONSTRAINT_SIZE),
                "Constraint too large"
            );
            Ok(())
        }
    }

    /// Validaton for intent directive.
    impl Validate for Directive {
        fn validate(&self) -> anyhow::Result<()> {
            if let Directive::Maximize(program) | Directive::Minimize(program) = &self {
                ensure!(program.len() <= MAX_DIRECTIVE_SIZE, "Directive too large");
            }
            Ok(())
        }
    }
}

pub mod solution {
    use super::{Validate as ValidateSignature, *};
    use essential_types::{
        intent::Intent,
        solution::{
            DecisionVariable, DecisionVariableIndex, PartialSolution, Solution, SolutionData,
            StateMutation,
        },
        ContentAddress, IntentAddress,
    };
    use solution::slots::MAX_DECISION_VARIABLES;
    use std::collections::{HashMap, HashSet};

    /// Maximum number of solution data of a solution.
    pub const MAX_SOLUTION_DATA: usize = 103;
    /// Maximum number of state mutations of a solution.
    pub const MAX_STATE_MUTATIONS: usize = 998;
    /// Maximum number of partial solutions of a solution.
    pub const MAX_PARTIAL_SOLUTIONS: usize = 97;

    /// Validation for solution.
    /// Validates the data, state mutations, and partial solutions.
    impl Validate for Solution {
        fn validate(&self) -> anyhow::Result<()> {
            self.data.validate()?;
            self.state_mutations.validate()?;
            self.partial_solutions.validate()?;

            Ok(())
        }
    }

    /// Validation for solution.data.
    impl Validate for Vec<SolutionData> {
        fn validate(&self) -> anyhow::Result<()> {
            ensure!(self.len() <= MAX_SOLUTION_DATA, "Too many solution data");
            ensure!(
                self.iter().all(|d| d
                    .decision_variables
                    .len()
                    .try_into()
                    .map_or(false, |num: u32| num <= MAX_DECISION_VARIABLES)),
                "Too many decision variables"
            );
            Ok(())
        }
    }

    /// Validation for solution.state_mutations.
    impl Validate for Vec<StateMutation> {
        fn validate(&self) -> anyhow::Result<()> {
            ensure!(
                self.len() <= MAX_STATE_MUTATIONS,
                "Too many state mutations"
            );
            Ok(())
        }
    }

    /// Validation for solution.partial_solutions.
    impl Validate for Vec<Signed<ContentAddress>> {
        fn validate(&self) -> anyhow::Result<()> {
            ensure!(
                self.len() <= MAX_PARTIAL_SOLUTIONS,
                "Too many partial solutions"
            );
            for ps in self {
                ValidateSignature::validate(ps)?;
            }
            Ok(())
        }
    }

    /// Validation for solution data.
    impl Validate for ContentAddress {
        fn validate(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    /// Validation with read from storage for solution.data.
    /// Called externally after non-storage validations.
    impl<S: Storage> ValidateWithStorage<S> for Vec<SolutionData> {
        async fn validate_with_storage(
            &self,
            storage: &S,
            solution: Solution,
        ) -> anyhow::Result<()> {
            let mut intents: HashMap<IntentAddress, Intent> = HashMap::new();
            for data in self {
                let address = data.intent_to_solve.clone();
                if let Ok(Some(intent)) = storage.get_intent(&address).await {
                    intents.insert(address, intent);
                } else {
                    anyhow::bail!("Failed to retrieve intent set from storage");
                }
            }
            (solution, intents).validate()?;
            Ok(())
        }
    }

    /// Validation with read from storage for solution.partial_solutions.
    /// Called externally after non-storage validations.
    impl<S: Storage> ValidateWithStorage<S> for Vec<Signed<ContentAddress>> {
        async fn validate_with_storage(
            &self,
            storage: &S,
            solution: Solution,
        ) -> anyhow::Result<()> {
            let mut partial_solutions: HashMap<ContentAddress, PartialSolution> = HashMap::new();
            for ps in self {
                let address = ps.data.clone();
                if let Ok(Some(ps)) = storage.get_partial_solution(&address).await {
                    partial_solutions.insert(address, ps.data);
                } else {
                    anyhow::bail!("Failed to retrieve partial solution from storage");
                }
            }
            (solution, partial_solutions).validate()?;
            Ok(())
        }
    }

    /// Validation for intents retrieved from storage against solution.
    impl Validate for (Solution, HashMap<IntentAddress, Intent>) {
        fn validate(&self) -> anyhow::Result<()> {
            let (
                Solution {
                    data,
                    state_mutations,
                    partial_solutions: _,
                },
                intents,
            ) = self;

            // This will never fail if called from ValidateWithStorage
            // TODO: No fail test exists for this reason
            // Maybe remove this check since it will be called from ValidateWithStorage.
            ensure!(
                data.iter()
                    .map(|d| &d.intent_to_solve)
                    .all(|address| intents.contains_key(address)),
                "All intents must be in the set"
            );

            ensure!(
                state_mutations
                    .iter()
                    .map(|mutation| &mutation.pathway)
                    .map(|pathway| data.get(*pathway as usize).map(|d| { &d.intent_to_solve }))
                    .all(|address| address.map_or(false, |address| intents.contains_key(address))),
                "All state mutations must have an intent in the set"
            );

            ensure!(
                data.iter().all(|d| (
                    d.clone(),
                    intents.get(&d.intent_to_solve).unwrap().slots.clone()
                )
                    .validate()
                    .is_ok()),
                "Ensure all solution data is valid"
            );

            ensure!(
                data.iter().all(|d| {
                    d.decision_variables.iter().all(|dv| match dv {
                        DecisionVariable::Inline(_) => true,
                        DecisionVariable::Transient(DecisionVariableIndex {
                            solution_data_index,
                            variable_index,
                        }) => data.get(*solution_data_index as usize).map_or(false, |d| {
                            d.decision_variables.len() > *variable_index as usize
                        }),
                    })
                }),
                "Invalid transient decision variable"
            );
            Ok(())
        }
    }

    /// Validation for partial solutions retrieved from storage against solution.
    impl Validate for (Solution, HashMap<ContentAddress, PartialSolution>) {
        fn validate(&self) -> anyhow::Result<()> {
            let (
                Solution {
                    data,
                    state_mutations,
                    partial_solutions,
                },
                solutions,
            ) = self;

            let data: HashMap<_, _> = data.iter().map(|d| (&d.intent_to_solve, d)).collect();
            let state_mutations: HashSet<_> = state_mutations.iter().collect();

            partial_solutions
                .iter()
                .map(|address| &address.data)
                .all(|address| {
                    solutions.get(address).map_or(false, |solution| {
                        let PartialSolution {
                            data: partial_data,
                            state_mutations: partial_state_mutations,
                        } = solution;

                        partial_data.iter().all(|d| {
                            data.get(&d.intent_to_solve).map_or(false, |data| {
                                let dec_vars: HashMap<usize, DecisionVariable> = data
                                    .decision_variables
                                    .iter()
                                    .enumerate()
                                    .map(|(i, dv)| (i, dv.clone()))
                                    .collect();
                                d.decision_variables
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(i, dv)| dv.as_ref().map(|dv| (i, dv)))
                                    .all(|(i, dv)| {
                                        dec_vars.get(&i).map_or(false, |data_dv| data_dv == dv)
                                    })
                            })
                        }) && partial_state_mutations
                            .iter()
                            .all(|mutation| state_mutations.contains(mutation))
                    })
                });

            Ok(())
        }
    }
}
