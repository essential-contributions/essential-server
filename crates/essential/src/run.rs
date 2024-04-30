use crate::solution::{check_solution_with_intents, read::read_intents_from_storage};
use essential_state_read_vm::StateRead;
use essential_types::{solution::Solution, Signed};
use std::{sync::Arc, time::Duration};
use storage::{failed_solution::SolutionFailReason, state_write::StateWrite, Storage};
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
    S: Storage + StateRead + StateWrite + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
    <S as StateWrite>::Future: Send,
    <S as StateWrite>::Error: Send,
{
    let _result = storage
        .prune_failed_solutions(Duration::from_secs(604800))
        .await;

    let (solutions, mut transaction) = build_block(storage).await?;

    let storage = transaction.storage();
    let failed_solutions: Vec<([u8; 32], SolutionFailReason)> = solutions
        .failed_solutions
        .iter()
        .map(|(solution, reason)| (hash(&solution.data), reason.clone()))
        .collect();
    storage.move_solutions_to_failed(&failed_solutions).await?;

    let mut solved_partial_solutions = vec![];
    let solved_solutions: Vec<[u8; 32]> = solutions
        .valid_solutions
        .iter()
        .map(|(solution, _utility)| {
            let solution_hash = hash(&solution.data);
            solution.data.partial_solutions.iter().for_each(|ps| {
                solved_partial_solutions.push(ps.data.0);
            });
            solution_hash
        })
        .collect();
    storage.move_solutions_to_solved(&solved_solutions).await?;
    storage
        .move_partial_solutions_to_solved(&solved_partial_solutions)
        .await?;

    transaction.commit().await?;

    Ok(())
}

async fn build_block<S>(storage: &S) -> anyhow::Result<(Solutions, TransactionStorage<S>)>
where
    S: Storage + StateRead + StateWrite + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
    <S as StateWrite>::Future: Send,
    <S as StateWrite>::Error: Send,
{
    let solutions = storage.list_solutions_pool().await?;
    let mut transaction = storage.clone().transaction();

    let mut valid_solutions: Vec<(Signed<Solution>, f64)> = vec![];
    let mut failed_solutions: Vec<(Signed<Solution>, SolutionFailReason)> = vec![];

    for solution in solutions.iter() {
        let intents = read_intents_from_storage(&solution.data, storage).await?;
        match check_solution_with_intents(
            &transaction.storage(),
            Arc::new(solution.clone().data),
            &intents,
        )
        .await
        {
            Ok(output) => {
                transaction = output.transaction;
                valid_solutions.push((solution.to_owned(), output.utility));
                // TODO: check composability
            }
            Err(_e) => {
                transaction.rollback();
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
