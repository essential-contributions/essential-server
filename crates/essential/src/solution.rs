// TODO: Remove this
#![allow(dead_code)]
#![allow(unused_variables)]

use self::validate::validate_solution_with_deps;
use essential_types::{intent::Intent, solution::Solution, Hash, Signed};
use storage::Storage;

mod read;
#[cfg(test)]
mod tests;
mod validate;

/// Validates a solution and submits it to storage.
pub async fn submit_solution<S>(storage: &S, solution: Signed<Solution>) -> anyhow::Result<Hash>
where
    S: Storage,
{
    validate_solution_with_deps(&solution, storage).await?;
    let solution_hash = utils::hash(&solution.data);

    match storage.insert_solution_into_pool(solution).await {
        Ok(()) => Ok(solution_hash),
        Err(e) => anyhow::bail!("Failed to submit solution: {}", e),
    }
}

pub async fn check_solution<S>(storage: &S, solution: Solution) -> anyhow::Result<f64>
where
    S: Storage,
{
    todo!()
}

pub async fn check_individual(intent: Intent, solution: Solution) -> anyhow::Result<f64> {
    todo!()
}
