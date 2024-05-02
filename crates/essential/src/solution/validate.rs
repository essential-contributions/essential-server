use super::read::{read_intents_from_storage, read_partial_solutions_from_storage};
use anyhow::ensure;
use essential_types::{
    intent::Intent,
    solution::{DecisionVariable, DecisionVariableIndex, PartialSolution, Solution},
    ContentAddress, IntentAddress, Key, Signed,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
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

/// Validate solution.
///
/// Performs validations for data, state mutations and and partial solutions
/// without reading intents or partial solutions from storage.
pub fn validate_solution(solution: &Signed<Solution>) -> anyhow::Result<()> {
    // Verify solution signature.
    ensure!(verify(solution), "Invalid solution signature");

    let solution = &solution.data;

    // Validate solution data.
    // Ensure that solution data length is below limit length.
    ensure!(
        solution.data.len() <= MAX_SOLUTION_DATA,
        "Too many solution data"
    );
    // Ensure that decision variables of each solution data are below limit length.
    ensure!(
        solution.data.iter().all(|d| d
            .decision_variables
            .len()
            .try_into()
            .map_or(false, |num: u32| num <= MAX_DECISION_VARIABLES)),
        "Too many decision variables"
    );

    // Validate state mutations.
    // Ensure that solution state mutations length is below limit length.
    ensure!(
        solution.state_mutations.len() <= MAX_STATE_MUTATIONS,
        "Too many state mutations"
    );
    // Ensure that all state mutations with a pathway points to some solution data.
    ensure!(
        solution
            .state_mutations
            .iter()
            .map(|mutation| &mutation.pathway)
            .all(|pathway| solution.data.len() > *pathway as usize),
        "All state mutations must have an intent in the set"
    );
    // Ensure that all state mutations are for unique slots.
    ensure!(
        solution
            .state_mutations
            .iter()
            .enumerate()
            .all(|(index, mutations)| {
                let keys_set = &mutations
                    .mutations
                    .iter()
                    .map(|m| m.key)
                    .collect::<HashSet<Key>>();
                solution
                    .state_mutations
                    .iter()
                    .enumerate()
                    .filter(|(index2, mutations2)| {
                        index != *index2 && mutations.pathway == mutations2.pathway
                    })
                    .all(|(_, mutations3)| {
                        !mutations3
                            .mutations
                            .iter()
                            .any(|m| keys_set.contains(&m.key))
                    })
            }),
        "More than one state mutation for the same slot"
    );

    // Validate partial solutions.
    // Ensure that solution partial solutions length is below limit length.
    ensure!(
        solution.partial_solutions.len() <= MAX_PARTIAL_SOLUTIONS,
        "Too many partial solutions"
    );
    // Verify signatures of all partial solutions.
    ensure!(
        solution.partial_solutions.iter().all(verify),
        "Invalid partial solution signature"
    );

    Ok(())
}

/// Validate intents retrieved from storage against solution.
///
/// Checks that:
/// - All intents in the solution have been read from the storage.
/// - All decision variables in the solution are valid.
pub fn validate_intents_against_solution(
    solution: &Solution,
    intents: &HashMap<IntentAddress, Arc<Intent>>,
) -> anyhow::Result<()> {
    let data = &solution.data;

    // Ensure that all intents that solution data solves are retrieved from the storage.
    ensure!(
        data.iter()
            .map(|d| &d.intent_to_solve)
            .all(|address| intents.contains_key(address)),
        "All intents must be in the set"
    );

    // Validate decision variables.
    // Checking that there are no cycles is performed by the constraint VM.
    for data in data.iter() {
        // Ensure that the number of decision variables in each solution data is
        // equal to the number of decision variables in the intent it solves.
        ensure!(
            data.decision_variables.len() as u32
                == intents
                    .get(&data.intent_to_solve)
                    .unwrap()
                    .slots
                    .decision_variables,
            "Decision variables mismatch"
        );

        // Ensure that all transient decision variables point to valid decision variables.
        ensure!(
            data.decision_variables.iter().all(|dv| {
                match dv {
                    DecisionVariable::Inline(_) => true,
                    DecisionVariable::Transient(DecisionVariableIndex {
                        solution_data_index,
                        variable_index,
                    }) => solution
                        .data
                        .get(*solution_data_index as usize)
                        .map_or(false, |d| {
                            d.decision_variables.len() > *variable_index as usize
                        }),
                }
            }),
            "Invalid transient decision variable"
        );
    }

    Ok(())
}

/// Validate partial solutions retrieved from storage against solution.
pub fn validate_partial_solutions_against_solution(
    solution: &Solution,
    partial_solutions: &HashMap<ContentAddress, PartialSolution>,
) -> anyhow::Result<()> {
    // Ensure that all partial solutions in the solution have been read from the storage.
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

    // Validate partial solution data.
    for (
        _,
        PartialSolution {
            data: partial_data,
            state_mutations: partial_state_mutations,
        },
    ) in partial_solutions.iter()
    {
        for pd in partial_data.iter() {
            // Ensure that intent solved by partial solution matches the one is in solution data.
            let partial_solution_data = data.get(&pd.intent_to_solve);
            ensure!(
                partial_solution_data.is_some(),
                "Partial solution intent to solve mismatch with solution data"
            );
            let partial_solution_data = partial_solution_data.unwrap();
            let dec_vars: HashMap<usize, DecisionVariable> = partial_solution_data
                .decision_variables
                .iter()
                .enumerate()
                .map(|(i, dv)| (i, dv.clone()))
                .collect();
            // Ensure that decision variables in partial solution data match the ones in solution data.
            ensure!(
                pd.decision_variables
                    .iter()
                    .enumerate()
                    .filter_map(|(i, dv)| dv.as_ref().map(|dv| (i, dv)))
                    .all(|(i, dv)| dec_vars.get(&i).map_or(false, |data_dv| data_dv == dv)),
                "Partial solution decision variables mismatch with solution data"
            );

            // Ensure that state mutations in partial solution data match the ones in solution data.
            ensure!(
                partial_state_mutations
                    .iter()
                    .all(|mutation| state_mutations.contains(mutation)),
                "Partial solution state mutations mismatch with solution data"
            );
        }
    }

    Ok(())
}

/// Validate solution fully, with intents and partial solutions from storage.
pub async fn validate_solution_with_deps<S>(
    solution: &Signed<Solution>,
    storage: &S,
) -> anyhow::Result<HashMap<IntentAddress, Arc<Intent>>>
where
    S: Storage,
{
    // Pre-storage read validations.
    validate_solution(solution)?;
    let solution = &solution.data;
    // Validation of intents being read from storage.
    let intents = read_intents_from_storage(solution, storage).await?;
    validate_intents_against_solution(solution, &intents)?;
    // Validation of partial solutions being read from storage.
    validate_partial_solutions_against_solution(
        solution,
        &read_partial_solutions_from_storage(solution, storage).await?,
    )?;
    Ok(intents)
}
