//! A library providing the Essential server's core logic around handling
//! storage read/write access, validation and state transitions.
//!
//! For an executable implementation of the Essential server, see the
//! `essential-rest-server` crate.

use essential_check::{
    self as check,
    solution::{CheckIntentConfig, Utility},
};
pub use essential_state_read_vm::{Gas, StateRead};
use essential_storage::failed_solution::CheckOutcome;
pub use essential_storage::Storage;
use essential_transaction_storage::{Transaction, TransactionStorage};
use essential_types::{
    intent::Intent, solution::Solution, Block, ContentAddress, Hash, IntentAddress, Key, Signed,
    StorageLayout, Word,
};
use run::{Handle, Shutdown};
use solution::read::read_intents_from_storage;
use std::{collections::HashMap, ops::Range, sync::Arc, time::Duration};

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
    // Currently only check-related config, though we may want to add a
    // top-level `Config` type for other kinds of configuration (e.g. gas costs).
    config: Arc<CheckIntentConfig>,
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
    pub fn new(storage: S, config: Arc<CheckIntentConfig>) -> Self {
        Self { storage, config }
    }

    pub fn spawn(self) -> anyhow::Result<Handle>
    where
        S: 'static + Send + Sync,
    {
        let (mut handle, shutdown) = Handle::new();
        let jh = tokio::spawn(async move { self.run(shutdown).await });
        handle.set_jh(jh);
        Ok(handle)
    }

    pub async fn run(&self, shutdown: Shutdown) -> anyhow::Result<()> {
        run::run(&self.storage, shutdown).await
    }

    pub async fn deploy_intent_set(
        &self,
        intents: Signed<Vec<Intent>>,
    ) -> anyhow::Result<ContentAddress> {
        deploy::deploy(&self.storage, intents).await
    }

    pub async fn check_solution(&self, solution: Solution) -> anyhow::Result<CheckSolutionOutput> {
        check::solution::check(&solution)?;
        let intents = read_intents_from_storage(&solution, &self.storage).await?;
        let transaction = self.storage.clone().transaction();
        let solution = Arc::new(solution);
        let config = self.config.clone();
        let (_post_state, utility, gas) =
            checked_state_transition(&transaction, solution, &intents, config).await?;
        Ok(CheckSolutionOutput { utility, gas })
    }

    pub async fn check_solution_with_data(
        &self,
        solution: Solution,
        intents: Vec<Intent>,
    ) -> anyhow::Result<CheckSolutionOutput> {
        let set = ContentAddress(essential_hash::hash(&intents));
        let intents: HashMap<_, _> = intents
            .into_iter()
            .map(|intent| {
                (
                    IntentAddress {
                        set: set.clone(),
                        intent: ContentAddress(essential_hash::hash(&intent)),
                    },
                    Arc::new(intent),
                )
            })
            .collect();

        check::solution::check(&solution)?;

        let transaction = self.storage.clone().transaction();
        let config = self.config.clone();
        let solution = Arc::new(solution);
        let (_post_state, utility, gas) =
            checked_state_transition(&transaction, solution, &intents, config).await?;
        Ok(CheckSolutionOutput { utility, gas })
    }

    pub async fn submit_solution(&self, solution: Solution) -> anyhow::Result<ContentAddress> {
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

    pub async fn list_solutions_pool(&self) -> anyhow::Result<Vec<Solution>> {
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

/// Performs the three main steps of producing a state transition.
///
/// 1. Validates the given `intents` against the given `solution` prior to execution.
/// 2. Clones the `pre_state` storage transaction and creates the proposed `post_state`.
/// 3. Checks that the solution's data satisfies all constraints.
///
/// In the success case, returns the post state, utility and total gas used.
pub(crate) async fn checked_state_transition<S>(
    pre_state: &TransactionStorage<S>,
    solution: Arc<Solution>,
    intents: &HashMap<IntentAddress, Arc<Intent>>,
    config: Arc<check::solution::CheckIntentConfig>,
) -> anyhow::Result<(TransactionStorage<S>, Utility, Gas)>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
{
    // Pre-execution validation.
    solution::validate_intents(&solution, intents)?;
    let get_intent = |addr: &IntentAddress| intents[addr].clone();

    // Create the post state for constraint checking.
    let post_state = solution::create_post_state(pre_state, &solution)?;

    // We only need read-only access to pre and post state during validation.
    let pre = pre_state.view();
    let post = post_state.view();
    let (util, gas) =
        check::solution::check_intents(&pre, &post, solution.clone(), get_intent, config).await?;

    Ok((post_state, util, gas))
}
