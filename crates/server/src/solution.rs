use essential_check as check;
use essential_storage::{StateStorage, Storage};
use essential_transaction_storage::TransactionStorage;
use essential_types::{intent::Intent, solution::Solution, ContentAddress, IntentAddress, Signed};
use std::{collections::HashMap, sync::Arc};
use tracing::Level;

pub(crate) mod read;
#[cfg(test)]
mod tests;

/// Validates a signed solution and submits it to storage.
#[tracing::instrument(skip_all, err(level=Level::DEBUG), ret(Display))]
pub async fn submit_solution<S>(
    storage: &S,
    solution: Signed<Solution>,
) -> anyhow::Result<ContentAddress>
where
    S: Storage,
{
    check::solution::check_signed(&solution)?;

    // Validation of intents being read from storage.
    let intents: HashMap<IntentAddress, Arc<Intent>> =
        read::read_intents_from_storage(&solution.data, storage).await?;
    validate_intents(&solution.data, &intents)?;

    // Insert the solution into the pool.
    let solution_hash = essential_hash::content_addr(&solution.data);
    match storage.insert_solution_into_pool(solution).await {
        Ok(()) => Ok(solution_hash),
        Err(err) => anyhow::bail!("Failed to submit solution: {}", err),
    }
}

/// Apply mutations proposed by the given `solution` to the given `storage`.
pub(crate) fn apply_mutations<S>(
    storage: &mut TransactionStorage<S>,
    solution: &Solution,
) -> anyhow::Result<()>
where
    S: StateStorage,
{
    for state_mutation in &solution.state_mutations {
        let set = &solution
            .data
            .get(state_mutation.pathway as usize)
            .ok_or(anyhow::anyhow!("Intent in solution data not found"))?
            .intent_to_solve
            .set;
        for mutation in state_mutation.mutations.iter() {
            storage.apply_state(set, &mutation.key, mutation.value);
        }
    }
    Ok(())
}

/// Given the pre_state and a solution, produce the post_state with all proposed
/// solution mutations applied.
#[tracing::instrument(skip_all, err)]
pub fn create_post_state<S>(
    pre_state: &TransactionStorage<S>,
    solution: &Solution,
) -> anyhow::Result<TransactionStorage<S>>
where
    S: Clone + StateStorage,
{
    let mut post_state = pre_state.clone();
    match apply_mutations(&mut post_state, solution) {
        Ok(()) => Ok(post_state),
        Err(err) => Err(err),
    }
}

/// Validate what we can of the solution's associated intents without performing execution.
#[tracing::instrument(skip_all, err)]
pub fn validate_intents(
    solution: &Solution,
    intents: &HashMap<IntentAddress, Arc<Intent>>,
) -> anyhow::Result<()> {
    // The map must contain all intents referred to by solution data.
    match contains_all_intents(solution, intents) {
        Ok(()) => {
            // The decision variable lengths must match.
            check::solution::check_decision_variable_lengths(solution, |addr| intents[addr].clone())
                .map_err(|(ix, err)| anyhow::anyhow!("solution data at {ix} invalid: {err}"))
        }
        Err(err) => Err(err),
    }
}

/// Ensure that all intents referred to by the solution have been read from the storage.
pub fn contains_all_intents(
    solution: &Solution,
    intents: &HashMap<IntentAddress, Arc<Intent>>,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        solution
            .data
            .iter()
            .all(|data| intents.contains_key(&data.intent_to_solve)),
        "All intents must be in the set"
    );
    Ok(())
}
