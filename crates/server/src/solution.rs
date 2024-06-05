use essential_check as check;
use essential_server_types::SolutionOutcome;
use essential_storage::{failed_solution::CheckOutcome, StateStorage, Storage};
use essential_transaction_storage::TransactionStorage;
use essential_types::{intent::Intent, solution::Solution, ContentAddress, Hash, IntentAddress};
use std::{collections::HashMap, sync::Arc};

pub(crate) mod read;
#[cfg(test)]
mod tests;

/// Validates a signed solution and submits it to storage.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err(level=tracing::Level::DEBUG), ret(Display)))]
pub async fn submit_solution<S>(storage: &S, solution: Solution) -> anyhow::Result<ContentAddress>
where
    S: Storage,
{
    check::solution::check(&solution)?;

    // Validation of intents being read from storage.
    let intents: HashMap<IntentAddress, Arc<Intent>> =
        read::read_intents_from_storage(&solution, storage).await?;
    validate_intents(&solution, &intents)?;

    // Insert the solution into the pool.
    let solution_hash = essential_hash::content_addr(&solution);
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
    for data in &solution.data {
        for mutation in data.state_mutations.iter() {
            storage.apply_state(
                &data.intent_to_solve.set,
                mutation.key.clone(),
                mutation.value.clone(),
            );
        }
    }
    Ok(())
}

/// Given the pre_state and a solution, produce the post_state with all proposed
/// solution mutations applied.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err))]
pub fn create_post_state<S>(
    pre_state: &TransactionStorage<S>,
    solution: &Solution,
) -> anyhow::Result<TransactionStorage<S>>
where
    S: Clone + StateStorage,
{
    let mut post_state = pre_state.clone();
    apply_mutations(&mut post_state, solution)?;
    Ok(post_state)
}

/// Validate what we can of the solution's associated intents without performing execution.
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err))]
pub fn validate_intents(
    solution: &Solution,
    intents: &HashMap<IntentAddress, Arc<Intent>>,
) -> anyhow::Result<()> {
    // The map must contain all intents referred to by solution data.
    contains_all_intents(solution, intents)?;
    Ok(())
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

/// Get all FIFO ordered outcomes of solution.
/// Outcome is the number of included block for successful solutions
/// and failure reason for failed solutions.
pub async fn solution_outcome<S>(
    storage: &S,
    solution_hash: &Hash,
) -> anyhow::Result<Vec<SolutionOutcome>>
where
    S: Storage,
{
    Ok(storage
        .get_solution(*solution_hash)
        .await?
        .map(|outcome| {
            outcome
                .outcome
                .into_iter()
                .map(|outcome| match outcome {
                    CheckOutcome::Success(block_number) => SolutionOutcome::Success(block_number),
                    CheckOutcome::Fail(fail) => SolutionOutcome::Fail(fail.to_string()),
                })
                .collect()
        })
        .unwrap_or_default())
}
