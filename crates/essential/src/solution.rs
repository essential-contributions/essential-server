// TODO: Remove this
#![allow(dead_code)]
#![allow(unused_variables)]

use essential_types::{intent::Intent, solution::Solution, Hash};
use placeholder::Signed;
use storage::Storage;

#[cfg(test)]
mod tests;

pub async fn submit_solution<S>(storage: &S, solution: Signed<Solution>) -> anyhow::Result<Hash>
where
    S: Storage,
{
    todo!()
}

pub async fn solve<S>(storage: &S) -> anyhow::Result<()>
where
    S: Storage,
{
    todo!()
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
