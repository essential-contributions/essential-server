use essential_state_read_vm::StateRead;
use essential_types::{solution::Solution, Signed};
use std::sync::Arc;
use storage::{failed_solution::SolutionFailReason, state_write::StateWrite, Storage};
use utils::hash;

use crate::solution::{check_solution_with_intents, validate::validate_solution_with_deps};

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
    let solutions = get_filtered_solutions(storage).await?;

    // TODO: search for best batch of solutions
    // update solutions.valid_solutions that will make up the block
    // and update solutions.failed_solutions that will be moved to failed solutions pool

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

    Ok(())
}

async fn get_filtered_solutions<S>(storage: &S) -> anyhow::Result<Solutions>
where
    S: Storage + StateRead + StateWrite + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
    <S as StateWrite>::Future: Send,
    <S as StateWrite>::Error: Send,
{
    let solutions = storage.list_solutions_pool().await?;

    let mut valid_solutions: Vec<(Signed<Solution>, f64)> = vec![];
    let mut failed_solutions: Vec<(Signed<Solution>, SolutionFailReason)> = vec![];

    for solution in solutions.iter() {
        // TODO: use tokio tasks
        match validate_solution_with_deps(solution, storage).await {
            Ok(intents) => {
                match check_solution_with_intents(
                    storage,
                    Arc::new(solution.data.clone()),
                    &intents,
                )
                .await
                {
                    Ok(output) => valid_solutions.push((solution.to_owned(), output.utility)),
                    Err(_e) => {
                        failed_solutions
                            .push((solution.to_owned(), SolutionFailReason::ConstraintsFailed));
                    }
                }
            }
            Err(_e) => {
                failed_solutions.push((solution.to_owned(), SolutionFailReason::ConstraintsFailed));
            }
        }
    }

    Ok(Solutions {
        valid_solutions,
        failed_solutions,
    })
}
