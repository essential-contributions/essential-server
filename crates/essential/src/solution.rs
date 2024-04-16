// TODO: Remove this
#![allow(dead_code)]
#![allow(unused_variables)]

use self::validate::validate_solution_fully;
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
    validate_solution_fully(&solution, storage).await?;

    match storage.insert_solution_into_pool(solution.clone()).await {
        Ok(()) => Ok(utils::hash(&solution.data)),
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
