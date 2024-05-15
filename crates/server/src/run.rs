use crate::{solution::read::read_intents_from_storage, PRUNE_FAILED_STORAGE_OLDER_THAN};
use essential_hash::hash;
use essential_state_read_vm::StateRead;
use essential_storage::{failed_solution::SolutionFailReason, Storage};
use essential_transaction_storage::{Transaction, TransactionStorage};
use essential_types::{solution::Solution, Hash};
use std::sync::Arc;
use tokio::sync::oneshot;

const RUN_LOOP_FREQUENCY: std::time::Duration = std::time::Duration::from_secs(10);

#[cfg(test)]
pub mod tests;

pub struct Handle {
    tx: oneshot::Sender<()>,
    jh: Option<tokio::task::JoinHandle<anyhow::Result<()>>>,
}

pub struct Shutdown(oneshot::Receiver<()>);

struct Solutions {
    valid_solutions: Vec<(Arc<Solution>, f64)>,
    failed_solutions: Vec<(Arc<Solution>, SolutionFailReason)>,
}

/// The main loop that builds blocks.
pub async fn run<S>(storage: &S, mut shutdown: Shutdown) -> anyhow::Result<()>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    // Run the main loop on a fixed interval.
    // The interval is immediately ready the first time.
    let mut interval = tokio::time::interval(RUN_LOOP_FREQUENCY);

    loop {
        // Either wait for the interval to tick or the shutdown signal.
        tokio::select! {
            _ = interval.tick() => {},
            _ = &mut shutdown.0 => return Ok(()),
        }

        // Build a block.
        match build_block(storage).await {
            Ok((solutions, mut transaction)) => {
                // FIXME: These 3 database commits should be atomic. If one fails they should all fail.
                // We don't have transactions for storage yet so that will be required to implement this.

                // Move failed solutions.
                let failed_solutions: Vec<(Hash, SolutionFailReason)> = solutions
                    .failed_solutions
                    .iter()
                    .map(|(solution, reason)| (hash(solution.as_ref()), reason.clone()))
                    .collect();
                match storage.move_solutions_to_failed(&failed_solutions).await {
                    Ok(()) => {
                        // Move valid solutions.
                        let solved_solutions: Vec<Hash> = solutions
                            .valid_solutions
                            .iter()
                            .map(|s| hash(s.0.as_ref()))
                            .collect();
                        match storage.move_solutions_to_solved(&solved_solutions).await {
                            Ok(()) => {
                                // Commit the state updates transaction.
                                match transaction.commit().await {
                                    Ok(()) => {
                                        if let Some(err) = storage
                                            .prune_failed_solutions(PRUNE_FAILED_STORAGE_OLDER_THAN)
                                            .await
                                            .err()
                                        {
                                            tracing::warn!(
                                                "error pruning failed solutions: {}",
                                                err
                                            )
                                        }
                                    }
                                    Err(err) => {
                                        tracing::error!(
                                            "error committing state changes to storage: {}",
                                            err
                                        )
                                    }
                                }
                            }
                            Err(err) => {
                                tracing::warn!("error marking solutions as solved: {}", err)
                            }
                        }
                    }
                    Err(err) => tracing::warn!("error marking solutions as failed: {}", err),
                }
            }
            Err(err) => tracing::error!("error building block: {}", err),
        }
    }
}

/// Build a block from the solutions pool.
///
/// The current implementation is very simple and just builds the
/// block in FIFO order. If a solution becomes invalid, it is moved to failed.
async fn build_block<S>(storage: &S) -> anyhow::Result<(Solutions, TransactionStorage<S>)>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
{
    // Get all solutions from the pool.
    // This returns the solutions in FIFO order.
    //
    // TODO: Page this and limit the amount of solutions pulled into memory.
    let solutions = storage.list_solutions_pool().await?;

    // Create a state db transaction.
    let mut transaction = storage.clone().transaction();

    let mut valid_solutions: Vec<_> = vec![];
    let mut failed_solutions: Vec<_> = vec![];

    for solution in solutions {
        let solution_hex = hex::encode(essential_hash::hash(&solution.data));
        // Put the solution into an Arc so it's cheap to clone.
        let solution = Arc::new(solution.data);

        // Get the intents for this solution.
        let intents = read_intents_from_storage(&solution, storage).await?;

        // Apply the proposed mutations, check the intents and return the result.
        let config = Default::default();

        match crate::checked_state_transition(&transaction, solution.clone(), &intents, config)
            .await
        {
            Ok((post_state, util, _gas)) => {
                // Update the transaction to the post state.
                transaction = post_state;
                // Collect the valid solution.
                valid_solutions.push((solution, util));

                tracing::debug!(
                    "valid solution with hash 0x{} and utility {}",
                    solution_hex,
                    util
                );
            }
            Err(err) => {
                // Collect the failed solution with the reason.
                tracing::debug!(
                    "failed solution with hash 0x{}: {}",
                    solution_hex,
                    err.to_string()
                );
                failed_solutions.push((
                    solution,
                    SolutionFailReason::ConstraintsFailed(err.to_string()),
                ));
            }
        }
    }

    Ok((
        Solutions {
            valid_solutions,
            failed_solutions,
        },
        transaction,
    ))
}

impl Handle {
    pub fn new() -> (Self, Shutdown) {
        let (tx, rx) = oneshot::channel();
        (Self { tx, jh: None }, Shutdown(rx))
    }

    pub fn set_jh(&mut self, jh: tokio::task::JoinHandle<anyhow::Result<()>>) {
        self.jh = Some(jh);
    }

    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.tx
            .send(())
            .map_err(|_| anyhow::anyhow!("Failed to send shutdown signal"))?;
        if let Some(jh) = self.jh {
            jh.await??;
        }
        Ok(())
    }
}
