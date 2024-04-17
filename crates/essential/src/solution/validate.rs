use super::read::{read_intents_from_storage, read_partial_solutions_from_storage};
use anyhow::ensure;
use essential_types::{
    intent::Intent,
    solution::{DecisionVariable, DecisionVariableIndex, PartialSolution, Solution},
    ContentAddress, IntentAddress, Signed,
};
use std::collections::{HashMap, HashSet};
use storage::Storage;
use utils::verify;

/// Maximum number of decision variables of a solution.
pub const MAX_DECISION_VARIABLES: u32 = 100;
/// Maximum number of solution data of a solution.
pub const MAX_SOLUTION_DATA: usize = 100;
/// Maximum number of state mutations of a solution.
pub const MAX_STATE_MUTATIONS: usize = 1000;
/// Maximum number of partial solutions of a solution.
pub const MAX_PARTIAL_SOLUTIONS: usize = 20;

/// Validation for solution.
/// Validates the data, state mutations, and partial solutions.
pub fn validate_solution(solution: &Signed<Solution>) -> anyhow::Result<()> {
    ensure!(verify(solution), "Failed to verify solution signature");

    let Signed {
        data: solution,
        signature: _,
    } = solution;

    // Validate solution data.
    ensure!(
        solution.data.len() <= MAX_SOLUTION_DATA,
        "Too many solution data"
    );
    ensure!(
        solution.data.iter().all(|d| d
            .decision_variables
            .len()
            .try_into()
            .map_or(false, |num: u32| num <= MAX_DECISION_VARIABLES)),
        "Too many decision variables"
    );

    // Validate state mutations
    ensure!(
        solution.state_mutations.len() <= MAX_STATE_MUTATIONS,
        "Too many state mutations"
    );

    // Validate partial solutions
    ensure!(
        solution.partial_solutions.len() <= MAX_PARTIAL_SOLUTIONS,
        "Too many partial solutions"
    );
    for ps in &solution.partial_solutions {
        ensure!(verify(ps), "Failed to verify partial solution signature");
    }
    Ok(())
}

/// Validation for intents retrieved from storage against solution.
pub fn validate_intents_against_solution(
    solution: &Solution,
    intents: &HashMap<IntentAddress, Intent>,
) -> anyhow::Result<()> {
    let Solution {
        data,
        state_mutations,
        partial_solutions: _,
    } = solution;

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
        data.iter().all(|d| {
            d.decision_variables.len() as u32
                == intents
                    .get(&d.intent_to_solve)
                    .unwrap()
                    .slots
                    .decision_variables
        }),
        "Decision variables mismatch"
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

/// Validation for partial solutions retrieved from storage against solution.
pub fn validate_partial_solutions_against_solution(
    solution: &Solution,
    partial_solutions: &HashMap<ContentAddress, PartialSolution>,
) -> anyhow::Result<()> {
    ensure!(
        solution
            .partial_solutions
            .iter()
            .map(|ps| &ps.data)
            .all(|address| partial_solutions.contains_key(address)),
        "All partial solutions must be in the set"
    );

    let data: HashMap<_, _> = solution
        .data
        .iter()
        .map(|d| (&d.intent_to_solve, d))
        .collect();
    let state_mutations: HashSet<_> = solution.state_mutations.iter().collect();

    partial_solutions
        .iter()
        .map(|address| address.0)
        .all(|address| {
            partial_solutions.get(address).map_or(false, |solution| {
                // TODO: maybe remove
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
                            .all(|(i, dv)| dec_vars.get(&i).map_or(false, |data_dv| data_dv == dv))
                    })
                }) && partial_state_mutations
                    .iter()
                    .all(|mutation| state_mutations.contains(mutation))
            })
        });

    Ok(())
}

pub async fn validate_solution_fully<S>(
    solution: &Signed<Solution>,
    storage: &S,
) -> anyhow::Result<()>
where
    S: Storage,
{
    validate_solution(solution)?;

    let Signed {
        data: solution,
        signature: _,
    } = solution;

    validate_intents_against_solution(
        solution,
        &read_intents_from_storage(solution, storage).await?,
    )?;
    validate_partial_solutions_against_solution(
        solution,
        &read_partial_solutions_from_storage(solution, storage).await?,
    )?;
    Ok(())
}
