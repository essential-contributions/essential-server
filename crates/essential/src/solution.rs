// TODO: Remove this
#![allow(dead_code)]
#![allow(unused_variables)]

use crate::validate::{Validate, ValidateWithStorage};
use essential_types::{intent::Intent, solution::Solution, Hash, Signed};
use storage::Storage;

#[cfg(test)]
mod tests;

pub async fn submit_solution<S>(storage: &S, solution: Signed<Solution>) -> anyhow::Result<Hash>
where
    S: Storage,
{
    solution.validate()?;

    solution
        .clone()
        .data
        .data
        .validate_with_storage(storage, solution.data.clone())
        .await?;

    solution
        .clone()
        .data
        .partial_solutions
        .validate_with_storage(storage, solution.data.clone())
        .await?;

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
