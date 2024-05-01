use crate::solution::{check_solution_with_intents, read::read_intents_from_storage};
use essential_state_read_vm::StateRead;
use essential_types::{solution::Solution, Signed};
use std::sync::Arc;
use storage::{failed_solution::SolutionFailReason, Storage};
use transaction_storage::{Transaction, TransactionStorage};
use utils::hash;

#[cfg(test)]
pub mod tests;

struct Solutions {
    valid_solutions: Vec<(Signed<Solution>, f64)>,
    failed_solutions: Vec<(Signed<Solution>, SolutionFailReason)>,
}

pub async fn run<S>(storage: &S) -> anyhow::Result<()>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let (solutions, mut transaction) = build_block(storage).await?;

    let failed_solutions: Vec<([u8; 32], SolutionFailReason)> = solutions
        .failed_solutions
        .iter()
        .map(|(solution, reason)| (hash(&solution.data), reason.clone()))
        .collect();
    storage.move_solutions_to_failed(&failed_solutions).await?;

    let solved_solutions: Vec<[u8; 32]> = solutions
        .valid_solutions
        .iter()
        .map(|s| hash(&s.0.data))
        .collect();
    storage.move_solutions_to_solved(&solved_solutions).await?;

    transaction.commit().await?;

    Ok(())
}

async fn build_block<S>(storage: &S) -> anyhow::Result<(Solutions, TransactionStorage<S>)>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    let solutions = storage.list_solutions_pool().await?;
    let mut transaction = storage.clone().transaction();

    let mut valid_solutions: Vec<(Signed<Solution>, f64)> = vec![];
    let mut failed_solutions: Vec<(Signed<Solution>, SolutionFailReason)> = vec![];

    for solution in solutions.iter() {
        let intents = read_intents_from_storage(&solution.data, storage).await?;
        let snapshot = transaction.snapshot();
        match check_solution_with_intents(transaction, Arc::new(solution.data.clone()), &intents)
            .await
        {
            Ok(output) => {
                match output.transaction.updates().iter().any(|(address, keys)| {
                    snapshot
                        .updates()
                        .get(address)
                        .map(|set| !set.to_owned().is_disjoint(keys))
                        .unwrap_or_default()
                }) {
                    true => {
                        transaction = snapshot;
                        failed_solutions
                            .push((solution.to_owned(), SolutionFailReason::NotComposable));
                    }
                    false => {
                        transaction = output.transaction;
                        valid_solutions.push((solution.to_owned(), output.utility));
                    }
                }
            }
            Err(_e) => {
                transaction = snapshot;
                failed_solutions.push((solution.to_owned(), SolutionFailReason::ConstraintsFailed));
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
