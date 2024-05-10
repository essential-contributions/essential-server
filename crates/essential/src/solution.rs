use essential_types::{solution::Solution, Hash, Signed};
use storage::Storage;

pub use validate::{validate_solution_with_data, validate_solution_with_deps};

pub(crate) mod read;
#[cfg(test)]
mod tests;
pub(crate) mod validate;

/// Validates a solution and submits it to storage.
pub async fn submit_solution<S>(storage: &S, solution: Signed<Solution>) -> anyhow::Result<Hash>
where
    S: Storage,
{
    validate_solution_with_deps(&solution, storage).await?;
    let solution_hash = essential_hash::hash(&solution.data);

    match storage.insert_solution_into_pool(solution).await {
        Ok(()) => Ok(solution_hash),
        Err(e) => anyhow::bail!("Failed to submit solution: {}", e),
    }
}
