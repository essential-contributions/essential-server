use super::read::{read_intents_from_storage, read_partial_solutions_from_storage};
use anyhow::ensure;
use essential_check as check;
use essential_types::{
    intent::Intent,
    solution::{DecisionVariable, PartialSolution, Solution},
    ContentAddress, IntentAddress, Signed,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use storage::Storage;

/// Validate partial solutions retrieved from storage against solution.
pub fn validate_partial_solutions_against_solution(
    solution: &Solution,
    partial_solutions: &HashMap<ContentAddress, Arc<PartialSolution>>,
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
    for PartialSolution {
        data: partial_data,
        state_mutations: partial_state_mutations,
    } in partial_solutions.iter().map(|(_, ps)| ps.as_ref())
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
    check::solution::check_signed(solution)?;

    // Validation of intents being read from storage.
    let intents = read_intents_from_storage(&solution.data, storage).await?;
    check::solution::check_decision_variable_lengths(&solution.data, |addr| intents[addr].clone())
        .map_err(|(ix, err)| anyhow::anyhow!("solution data at {ix} invalid: {err}"))?;

    // Validation of partial solutions being read from storage.
    let partial_solutions = read_partial_solutions_from_storage(&solution.data, storage).await?;
    // TODO: Do this in `essential-check`.
    validate_partial_solutions_against_solution(&solution.data, &partial_solutions)?;

    Ok(intents)
}

/// Validate solution fully, with intents and partial solutions from storage.
pub fn validate_solution_with_data(
    solution: &Signed<Solution>,
    partial_solutions: &HashMap<ContentAddress, Arc<PartialSolution>>,
    intents: &HashMap<IntentAddress, Arc<Intent>>,
) -> anyhow::Result<()> {
    // Pre-storage read validations.
    check::solution::check_signed(solution)?;

    // Validation of intents being read from storage.
    check::solution::check_decision_variable_lengths(&solution.data, |addr| intents[addr].clone())
        .map_err(|(ix, err)| anyhow::anyhow!("solution data at {ix} invalid: {err}"))?;

    // Validation of partial solutions being read from storage.
    // TODO: Do this in `essential-check`.
    validate_partial_solutions_against_solution(&solution.data, partial_solutions)?;

    Ok(())
}
