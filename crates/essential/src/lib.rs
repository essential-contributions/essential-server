use essential_state_read_vm::StateRead;
use essential_types::{
    intent::Intent, solution::Solution, Block, ContentAddress, Hash, IntentAddress, Key, Signed,
    StorageLayout, Word,
};
use solution::Output;
use std::{ops::Range, sync::Arc, time::Duration};
use storage::{failed_solution::CheckOutcome, Storage};
use transaction_storage::Transaction;

mod deploy;
mod run;
mod solution;
#[cfg(test)]
mod test_utils;

#[derive(Clone)]
pub struct Essential<S>
where
    S: Storage + Clone,
{
    storage: S,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct CheckSolutionOutput {
    pub utility: f64,
    pub gas: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum SolutionOutcome {
    Success(u64),
    Fail(String),
}

const PRUNE_FAILED_STORAGE_OLDER_THAN: Duration = Duration::from_secs(604800); // one week

impl<S> Essential<S>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    pub fn new(storage: S) -> Self {
        Self { storage }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        run::run(&self.storage).await
    }

    pub async fn deploy_intent_set(
        &self,
        intents: Signed<Vec<Intent>>,
    ) -> anyhow::Result<ContentAddress> {
        deploy::deploy(&self.storage, intents).await
    }

    pub async fn check_solution(
        &self,
        solution: Signed<Solution>,
    ) -> anyhow::Result<CheckSolutionOutput> {
        let intents = solution::validate_solution_with_deps(&solution, &self.storage).await?;
        let transaction = self.storage.clone().transaction();
        let Output {
            transaction: _,
            utility,
            gas_used,
        } = solution::check_solution_with_intents(transaction, Arc::new(solution.data), &intents)
            .await?;
        Ok(CheckSolutionOutput {
            utility,
            gas: gas_used,
        })
    }

    pub async fn submit_solution(&self, solution: Signed<Solution>) -> anyhow::Result<Hash> {
        solution::submit_solution(&self.storage, solution).await
    }

    pub async fn solution_outcome(
        &self,
        solution_hash: &Hash,
    ) -> anyhow::Result<Option<SolutionOutcome>> {
        Ok(self
            .storage
            .get_solution(*solution_hash)
            .await?
            .map(|outcome| match outcome.outcome {
                CheckOutcome::Success(block_number) => SolutionOutcome::Success(block_number),
                CheckOutcome::Fail(fail) => SolutionOutcome::Fail(fail.to_string()),
            }))
    }

    pub async fn get_intent(&self, address: &IntentAddress) -> anyhow::Result<Option<Intent>> {
        self.storage.get_intent(address).await
    }

    pub async fn get_intent_set(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<Signed<Vec<Intent>>>> {
        self.storage.get_intent_set(address).await
    }

    pub async fn list_intent_sets(
        &self,
        time_range: Option<Range<Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Vec<Intent>>> {
        self.storage.list_intent_sets(time_range, page).await
    }

    pub async fn list_solutions_pool(&self) -> anyhow::Result<Vec<Signed<Solution>>> {
        self.storage.list_solutions_pool().await
    }

    pub async fn list_winning_blocks(
        &self,
        time_range: Option<Range<Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Block>> {
        self.storage.list_winning_blocks(time_range, page).await
    }

    pub async fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> anyhow::Result<Option<Word>> {
        self.storage.query_state(address, key).await
    }

    pub async fn get_storage_layout(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<StorageLayout>> {
        self.storage.get_storage_layout(address).await
    }
}
