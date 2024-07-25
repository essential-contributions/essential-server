//! A library providing the Essential server's core logic around handling
//! storage read/write access, validation and state transitions.
//!
//! For an executable implementation of the Essential server, see the
//! `essential-rest-server` crate.

use essential_check::{
    self as check,
    solution::{CheckPredicateConfig, Utility},
};
pub use essential_server_types::{CheckSolutionOutput, SolutionOutcome};
pub use essential_state_read_vm::{Gas, StateRead};
use essential_storage::failed_solution::CheckOutcome;
pub use essential_storage::Storage;
use essential_transaction_storage::{Transaction, TransactionStorage};
use essential_types::{
    contract::{Contract, SignedContract},
    predicate::Predicate,
    solution::Solution,
    Block, ContentAddress, Hash, Key, PredicateAddress, Word,
};
use run::{Handle, Shutdown};
use solution::read::read_contract_from_storage;
use std::{collections::HashMap, ops::Range, sync::Arc, time::Duration};

mod deploy;
mod query_state_reads;
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
    config: Arc<CheckPredicateConfig>,
}

#[derive(Debug, Clone)]
/// Server configuration.
pub struct Config {
    /// Interval at which to run the main loop.
    pub run_loop_interval: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            run_loop_interval: run::RUN_LOOP_FREQUENCY,
        }
    }
}

const PRUNE_FAILED_STORAGE_OLDER_THAN: Duration = Duration::from_secs(604800); // one week

impl<S> Essential<S>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    pub fn new(storage: S, config: Arc<CheckPredicateConfig>) -> Self {
        Self { storage, config }
    }

    pub fn spawn(self, config: Config) -> anyhow::Result<Handle>
    where
        S: 'static + Send + Sync,
    {
        let (mut handle, shutdown) = Handle::new();
        let jh = tokio::spawn(async move { self.run(shutdown, config.run_loop_interval).await });
        handle.contract_jh(jh);
        Ok(handle)
    }

    pub async fn run(&self, shutdown: Shutdown, run_loop_interval: Duration) -> anyhow::Result<()> {
        run::run(&self.storage, shutdown, run_loop_interval).await
    }

    pub async fn deploy_contract(
        &self,
        contract: SignedContract,
    ) -> anyhow::Result<ContentAddress> {
        deploy::deploy(&self.storage, contract).await
    }

    pub async fn check_solution(&self, solution: Solution) -> anyhow::Result<CheckSolutionOutput> {
        check::solution::check(&solution)?;
        let contract = read_contract_from_storage(&solution, &self.storage).await?;
        let transaction = self.storage.clone().transaction();
        let solution = Arc::new(solution);
        let config = self.config.clone();
        let (_post_state, utility, gas) =
            checked_state_transition(&transaction, solution, &contract, config).await?;
        Ok(CheckSolutionOutput { utility, gas })
    }

    pub async fn check_solution_with_contracts(
        &self,
        solution: Solution,
        contracts: Vec<Contract>,
    ) -> anyhow::Result<CheckSolutionOutput> {
        let predicates: HashMap<_, _> = contracts
            .into_iter()
            .flat_map(|contract| {
                let contract_addr = essential_hash::contract_addr::from_contract(&contract);
                contract.predicates.into_iter().map({
                    let contract_addr = contract_addr.clone();
                    move |predicate| {
                        (
                            PredicateAddress {
                                contract: contract_addr.clone(),
                                predicate: essential_hash::content_addr(&predicate),
                            },
                            Arc::new(predicate),
                        )
                    }
                })
            })
            .collect();

        check::solution::check(&solution)?;

        let transaction = self.storage.clone().transaction();
        let config = self.config.clone();
        let solution = Arc::new(solution);
        let (_post_state, utility, gas) =
            checked_state_transition(&transaction, solution, &predicates, config).await?;
        Ok(CheckSolutionOutput { utility, gas })
    }

    pub async fn submit_solution(&self, solution: Solution) -> anyhow::Result<ContentAddress> {
        solution::submit_solution(&self.storage, solution).await
    }

    pub async fn solution_outcome(
        &self,
        solution_hash: &Hash,
    ) -> anyhow::Result<Vec<SolutionOutcome>> {
        Ok(self
            .storage
            .get_solution(*solution_hash)
            .await?
            .map(|outcome| {
                outcome
                    .outcome
                    .into_iter()
                    .map(|outcome| match outcome {
                        CheckOutcome::Success(block_number) => {
                            SolutionOutcome::Success(block_number)
                        }
                        CheckOutcome::Fail(fail) => SolutionOutcome::Fail(fail.to_string()),
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn get_predicate(
        &self,
        address: &PredicateAddress,
    ) -> anyhow::Result<Option<Predicate>> {
        self.storage.get_predicate(address).await
    }

    pub async fn get_contract(
        &self,
        address: &ContentAddress,
    ) -> anyhow::Result<Option<SignedContract>> {
        self.storage.get_contract(address).await
    }

    pub async fn list_contracts(
        &self,
        time_range: Option<Range<Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Contract>> {
        self.storage.list_contracts(time_range, page).await
    }

    pub fn subscribe_contracts(
        &self,
        start_time: Option<Duration>,
        start_page: Option<usize>,
    ) -> impl futures::stream::Stream<Item = anyhow::Result<Contract>> + Send + 'static {
        self.storage
            .clone()
            .subscribe_contracts(start_time, start_page)
    }

    pub async fn list_solutions_pool(&self, page: Option<usize>) -> anyhow::Result<Vec<Solution>> {
        self.storage.list_solutions_pool(page).await
    }

    pub async fn list_blocks(
        &self,
        time_range: Option<Range<Duration>>,
        page: Option<usize>,
    ) -> anyhow::Result<Vec<Block>> {
        self.storage.list_blocks(time_range, page).await
    }

    pub fn subscribe_blocks(
        &self,
        start_time: Option<Duration>,
        start_number: Option<u64>,
        start_page: Option<usize>,
    ) -> impl futures::stream::Stream<Item = anyhow::Result<Block>> + Send + 'static {
        self.storage
            .clone()
            .subscribe_blocks(start_time, start_number, start_page)
    }

    pub async fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> anyhow::Result<Vec<Word>> {
        self.storage.query_state(address, key).await
    }

    pub async fn query_state_reads(
        &self,
        query: essential_server_types::QueryStateReads,
    ) -> anyhow::Result<essential_server_types::QueryStateReadsOutput> {
        let storage = self.storage.clone().transaction();
        query_state_reads::query_state_reads(storage, query).await
    }
}

/// Performs the three main steps of producing a state transition.
///
/// 1. Validates the given `contract` against the given `solution` prior to execution.
/// 2. Clones the `pre_state` storage transaction and creates the proposed `post_state`.
/// 3. Checks that the solution's data satisfies all constraints.
///
/// In the success case, returns the post state, utility and total gas used.
pub(crate) async fn checked_state_transition<S>(
    pre_state: &TransactionStorage<S>,
    solution: Arc<Solution>,
    contract: &HashMap<PredicateAddress, Arc<Predicate>>,
    config: Arc<check::solution::CheckPredicateConfig>,
) -> anyhow::Result<(TransactionStorage<S>, Utility, Gas)>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
{
    // Pre-execution validation.
    solution::validate_contract(&solution, contract)?;
    let get_predicate = |addr: &PredicateAddress| contract[addr].clone();

    // Create the post state for constraint checking.
    let post_state = solution::create_post_state(pre_state, &solution)?;

    // We only need read-only access to pre and post state during validation.
    let pre = pre_state.view();
    let post = post_state.view();
    let (util, gas) =
        check::solution::check_predicates(&pre, &post, solution.clone(), get_predicate, config)
            .await?;

    Ok((post_state, util, gas))
}
