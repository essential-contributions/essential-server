use crate::{solution::read::read_intents_from_storage, PRUNE_FAILED_STORAGE_OLDER_THAN};
use anyhow::Context;
use essential_hash::hash;
use essential_state_read_vm::StateRead;
use essential_storage::{failed_solution::SolutionFailReason, CommitData, Storage};
use essential_transaction_storage::{Transaction, TransactionStorage};
use essential_types::{solution::Solution, Hash};
use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;

pub(crate) const RUN_LOOP_FREQUENCY: std::time::Duration = std::time::Duration::from_secs(10);
const MAX_BLOCK_SIZE: usize = 1_000;

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
pub async fn run<S>(
    storage: &S,
    mut shutdown: Shutdown,
    run_loop_interval: Duration,
) -> anyhow::Result<()>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    // Run the main loop on a fixed interval.
    // The interval is immediately ready the first time.
    let mut interval = tokio::time::interval(run_loop_interval);

    loop {
        // Either wait for the interval to tick or the shutdown signal.
        tokio::select! {
            _ = interval.tick() => {},
            _ = &mut shutdown.0 => return Ok(()),
        }

        // Errors are emitted via `tracing`.
        let _ = run_loop(storage).await;
    }
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err))]
async fn run_loop<S>(storage: &S) -> anyhow::Result<()>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    // Build a block.
    let (solutions, transaction) = build_block(storage).await.context("error building block")?;

    // FIXME: These 3 database commits should be atomic. If one fails they should all fail.
    // We don't have transactions for storage yet so that will be required to implement this.

    // Move failed solutions.
    let failed_solutions: Vec<(Hash, SolutionFailReason)> = solutions
        .failed_solutions
        .iter()
        .map(|(solution, reason)| (hash(solution.as_ref()), reason.clone()))
        .collect();

    // Move valid solutions.
    let solved_solutions: Vec<Hash> = solutions
        .valid_solutions
        .iter()
        .map(|s| hash(s.0.as_ref()))
        .collect();

    let data = CommitData {
        failed: &failed_solutions,
        solved: &solved_solutions,
        state_updates: Box::new(transaction.into_updates()),
    };

    // Atomically commit the block.
    storage
        .commit_block(data)
        .await
        .context("error committing block")?;

    storage
        .prune_failed_solutions(PRUNE_FAILED_STORAGE_OLDER_THAN)
        .await
        .context("error pruning failed solutions")?;

    Ok(())
}

/// Build a block from the solutions pool.
///
/// The current implementation is very simple and just builds the
/// block in FIFO order. If a solution becomes invalid, it is moved to failed.
async fn build_block<S>(storage: &S) -> anyhow::Result<(Solutions, TransactionStorage<S>)>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
{
    // Get max solutions from the pool.
    // This returns the solutions in FIFO order.

    // This is the page size we use in both our dbs. Unfortunately, we can't
    // export the const as we are using a generic here.
    let page_size = 100;
    let mut last = 0;
    let mut solutions = Vec::new();
    let mut page = 0;

    // Pull in the first round of solutions.
    let new_solutions = storage.list_solutions_pool(Some(page)).await?;
    solutions.extend(new_solutions);
    page += 1;

    // While we are pulling full pages and are under the max block size, keep pulling.
    while solutions.len() - last == page_size && solutions.len() <= MAX_BLOCK_SIZE {
        let new_solutions = storage.list_solutions_pool(Some(page)).await?;
        last = solutions.len();
        solutions.extend(new_solutions);
        page += 1;
    }

    // Create a state db transaction.
    let mut transaction = storage.clone().transaction();

    let mut valid_solutions: Vec<_> = vec![];
    let mut failed_solutions: Vec<_> = vec![];

    for solution in solutions {
        #[cfg(feature = "tracing")]
        let solution_hash = essential_hash::content_addr(&solution);
        // Put the solution into an Arc so it's cheap to clone.
        let solution = Arc::new(solution);

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
                #[cfg(feature = "tracing")]
                tracing::debug!(valid_solution = %solution_hash, utility = util);
            }
            Err(err) => {
                // Collect the failed solution with the reason.
                failed_solutions.push((
                    solution,
                    SolutionFailReason::ConstraintsFailed(err.to_string()),
                ));
                #[cfg(feature = "tracing")]
                tracing::debug!(failed_solution = %solution_hash, %err);
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
