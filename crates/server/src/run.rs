use crate::{
    solution::read::read_contract_from_storage, TimeConfig, PRUNE_FAILED_STORAGE_OLDER_THAN,
};
use anyhow::Context;
use essential_hash::hash;
use essential_state_read_vm::StateRead;
use essential_storage::{failed_solution::SolutionFailReason, CommitData, Storage};
use essential_transaction_storage::{Transaction, TransactionStorage};
use essential_types::{contract::SignedContract, solution::Solution, Hash, Signature};
use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;

pub(crate) const RUN_LOOP_FREQUENCY: std::time::Duration = std::time::Duration::from_secs(10);

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
    time_config: &TimeConfig,
) -> anyhow::Result<()>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    if time_config.enable_time {
        // Deploy the block state contract.
        deploy_protocol_contracts(storage).await?;
    }

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
        let _ = run_loop(storage, time_config).await;
    }
}

/// Deploy the protocol contracts.
async fn deploy_protocol_contracts<S>(storage: &S) -> Result<(), anyhow::Error>
where
    S: Storage,
{
    let block_state_contract = crate::protocol::block_state_contract();

    // Signing with fake signature because there's no
    // private key to sign with and this is never checked
    // at this level anyway.
    let block_state_contract = SignedContract {
        contract: block_state_contract,
        signature: Signature([0; 64], 0),
    };
    storage.insert_contract(block_state_contract).await?;
    Ok(())
}

#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, err))]
async fn run_loop<S>(storage: &S, time_config: &TimeConfig) -> anyhow::Result<()>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    // Build a block.
    let (block_number, block_timestamp, solutions, transaction) = build_block(storage, time_config)
        .await
        .context("error building block")?;

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
        block_number,
        block_timestamp,
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
async fn build_block<S>(
    storage: &S,
    time_config: &TimeConfig,
) -> anyhow::Result<(u64, Duration, Solutions, TransactionStorage<S>)>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
{
    // Get all solutions from the pool.
    // This returns the solutions in FIFO order.
    let solutions = storage.list_solutions_pool(Some(0)).await?;

    // Create a state db transaction.
    let mut transaction = storage.clone().transaction();

    let mut valid_solutions: Vec<_> = vec![];
    let mut failed_solutions: Vec<_> = vec![];

    let latest_block = storage.get_latest_block().await?;
    let number = latest_block
        .as_ref()
        .map(|b| b.number.saturating_add(1))
        .unwrap_or(0);
    let timestamp = match &latest_block {
        Some(block) => {
            let monotonic_time = block.timestamp.saturating_add(Duration::from_secs(1));
            let system_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(monotonic_time);
            monotonic_time.max(system_time)
        }
        None => std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default(),
    };

    let solutions = if time_config.enable_time {
        // Add the block state solution at the begging of the block.
        let block_state_solution =
            crate::protocol::block_state_solution(number, timestamp.as_secs());
        let mut s = vec![block_state_solution];
        s.extend(solutions);
        s
    } else {
        solutions
    };

    for solution in solutions {
        #[cfg(feature = "tracing")]
        let solution_hash = essential_hash::content_addr(&solution);
        // Put the solution into an Arc so it's cheap to clone.
        let solution = Arc::new(solution);

        // Get the contract for this solution.
        let contract = read_contract_from_storage(&solution, storage).await?;

        // Apply the proposed mutations, check the contract and return the result.
        let config = Default::default();

        match crate::checked_state_transition(&transaction, solution.clone(), &contract, config)
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

    // If there is only one valid solution then
    // it's only the block state solution.
    if valid_solutions.len() == 1 && time_config.enable_time {
        valid_solutions.clear();
    }

    Ok((
        number,
        timestamp,
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

    pub fn contract_jh(&mut self, jh: tokio::task::JoinHandle<anyhow::Result<()>>) {
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
