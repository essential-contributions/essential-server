use crate::solution::{check_solution_with_intents, read::read_intents_from_storage};
use essential_state_read_vm::StateRead;
use essential_types::{solution::Solution, Hash};
use std::sync::Arc;
use storage::{failed_solution::SolutionFailReason, Storage};
use tokio::sync::oneshot;
use transaction_storage::{Transaction, TransactionStorage};
use utils::hash;

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
pub async fn run<S>(storage: &S, shutdown: Shutdown) -> anyhow::Result<()>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    // Build a block.
    let (solutions, mut transaction) = build_block(storage).await?;

    // FIXME: These 3 database commits should be atomic. If one fails they should all fail.
    // We don't have transactions for storage yet so that will be required to implement this.

    // Move failed solutions.
    let failed_solutions: Vec<(Hash, SolutionFailReason)> = solutions
        .failed_solutions
        .iter()
        .map(|(solution, reason)| (hash(solution.as_ref()), reason.clone()))
        .collect();
    storage.move_solutions_to_failed(&failed_solutions).await?;

    // Move valid solutions.
    let solved_solutions: Vec<Hash> = solutions
        .valid_solutions
        .iter()
        .map(|s| hash(s.0.as_ref()))
        .collect();
    storage.move_solutions_to_solved(&solved_solutions).await?;

    // Commit the state updates transaction.
    transaction.commit().await?;

    shutdown.0.await?;

    Ok(())
}

/// Build a block from the solutions pool.
///
/// The current implementation is very simple and just builds the
/// block in FIFO order. If a solution becomes invalid, it is moved to failed.
async fn build_block<S>(storage: &S) -> anyhow::Result<(Solutions, TransactionStorage<S>)>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
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
        // Put the solution into an Arc so it's cheap to clone.
        let solution = Arc::new(solution.data);

        // Get the intents for this solution.
        let intents = read_intents_from_storage(&solution, storage).await?;

        // TODO: This snapshot means all state mutations will cause clones.
        // We should add a functionality to record which snapshots mutations are part of
        // then we can just record which index this snapshot is.
        // Then we can `rollback_to(snapshot)`.
        // This would also require returning the transaction on error.
        //
        // Check the solution.
        match check_solution_with_intents(transaction.snapshot(), solution.clone(), &intents).await
        {
            Ok(output) => {
                // Set update the transaction.
                transaction = output.transaction;

                // Collect the valid solution.
                valid_solutions.push((solution, output.utility));
            }
            Err(e) => {
                // Collect the failed solution with the reason.
                failed_solutions.push((
                    solution,
                    SolutionFailReason::ConstraintsFailed(e.to_string()),
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
