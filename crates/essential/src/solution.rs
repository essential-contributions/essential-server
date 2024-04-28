use self::validate::validate_solution_with_deps;
use anyhow::Error;
use essential_constraint_vm::{check_intent, exec_bytecode_iter};
use essential_state_read_vm::{
    asm::Op, Access, GasLimit, SolutionAccess, StateRead, StateSlots, Vm,
};
use essential_types::{
    intent::{Directive, Intent},
    solution::{Solution, StateMutation},
    ContentAddress, Hash, IntentAddress, Signed, Word,
};
use std::{collections::HashMap, sync::Arc};
use storage::{state_write::StateWrite, Storage};
use tokio::task::JoinSet;
use utils::Lock;

mod read;
#[cfg(test)]
mod tests;
pub(crate) mod validate;

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

/// Checks a solution against state read VM and constraint VM after reading intents from storage.
///
/// Returns utility score of solution.
pub async fn check_solution<S>(storage: &S, solution: Arc<Solution>) -> anyhow::Result<u64>
where
    S: Storage + StateRead + StateWrite + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
    <S as StateWrite>::Future: Send,
    <S as StateWrite>::Error: Send,
{
    // Read intents from storage.
    let intents = read::read_intents_from_storage(&solution, storage).await?;
    check_solution_with_intents(storage, solution, &intents).await
}

/// Checks a solution against state read VM and constraint VM.
///
/// For each state read program of intents in each solution data, applies state mutations
/// and then check constraints on pre-mutation and post-mutation state.
///
/// Unlike `check_solution`, this function takes a set of intents to check against.
/// This is useful when intents have already been read from storage, e.g. during solution validation.
///
/// Returns utility score of solution.
pub async fn check_solution_with_intents<S>(
    storage: &S,
    solution: Arc<Solution>,
    intents: &HashMap<IntentAddress, Intent>,
) -> anyhow::Result<u64>
where
    S: Storage + StateRead + StateWrite + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
    <S as StateWrite>::Future: Send,
    <S as StateWrite>::Error: Send,
{
    let utility = Arc::new(Lock::new(0));
    let mut set = JoinSet::new();
    // TODO: avoid clone here?
    let state_mutations = Arc::new(solution.state_mutations.clone());

    for (intent_index, data) in solution.data.iter().cloned().enumerate() {
        let Some(intent) = intents.get(&data.intent_to_solve).cloned() else {
            anyhow::bail!("Intent in solution data not found in intents set");
        };
        let solution = solution.clone();
        let utility = utility.clone();
        let storage = storage.clone();
        let state_mutations = state_mutations.clone();

        set.spawn(async move {
            for state_read in &intent.state_read {
                // Pre-mutation state read.
                let mut vm = Vm::default();
                let solution_access =
                    SolutionAccess::new(&solution, intent_index.try_into().unwrap());
                let mut access = Access {
                    solution: solution_access,
                    state_slots: StateSlots::EMPTY,
                };
                match read_state(&mut vm, state_read, access, &storage).await {
                    Ok(_gas) => {
                        // Apply state mutations.
                        let relevant_state_mutations: Vec<StateMutation> = state_mutations
                            .iter()
                            .filter(|mutation| mutation.pathway == intent_index.try_into().unwrap())
                            .cloned()
                            .collect();

                        // Set pre-mutation state slots.
                        let pre_slots = apply_mutations(
                            relevant_state_mutations,
                            data.intent_to_solve.set.clone(),
                            &storage,
                        )
                        .await
                        .map_err(|e| anyhow::Error::msg(e.to_string()))?;
                        access.state_slots.pre = &pre_slots;

                        // Post-mutation state read.
                        let mut vm = Vm::default();
                        match read_state(&mut vm, state_read, access, &storage).await {
                            Ok(_gas) => {
                                // Set post-mutation state slots.
                                let post_slots = vm.into_state_slots();
                                access.state_slots.post = &post_slots;
                                // Run constraint checks.
                                check_constraints(&intent, access, Some(utility.clone()))?;
                            }
                            Err(e) => anyhow::bail!("State read VM execution failed: {}", e),
                        }
                    }
                    Err(e) => anyhow::bail!("State read VM execution failed: {}", e),
                }
            }
            Ok(())
        });
    }

    while let Some(res) = set.join_next().await {
        res??;
    }

    Ok(utility.inner())
}

/// Reads state slots from storage using state read program.
///
/// The result is written to VM's memory and can be accessed
/// either using `Vm::into_state_slots` or by reading memory directly.
///
/// Returns gas used by VM.
async fn read_state<S>(
    vm: &mut Vm,
    state_read: &[u8],
    access: Access<'_>,
    storage: &S,
) -> anyhow::Result<u64>
where
    S: StateRead,
{
    vm.exec_bytecode_iter(
        state_read.iter().cloned(),
        access,
        storage,
        &|_: &Op| 1,
        GasLimit::UNLIMITED,
    )
    .await
    .map_err(|e| anyhow::Error::msg(e.to_string()))
}

/// Applies state mutations to storage for intent in solution.
///
/// Expects state mutations relevant to the intent as parameter.
///
/// Returns pre-mutation state slots for each intent in solution data.
async fn apply_mutations<S>(
    state_mutations: Vec<StateMutation>,
    intent: ContentAddress,
    storage: &S,
) -> Result<Vec<Option<Word>>, <S as StateWrite>::Error>
where
    S: StateWrite,
{
    let mut updates = vec![];
    for state_mutation in state_mutations.iter() {
        for mutation in state_mutation.mutations.iter() {
            updates.push((intent.clone(), mutation.key, mutation.value));
        }
    }
    storage.update_state_batch(updates).await
}

/// Checks intent constraints against its state slots.
///
/// If `utility` is `Some`, adds the utility of solution for intent to `utility`.
fn check_constraints(
    intent: &Intent,
    access: Access,
    utility: Option<Arc<Lock<u64>>>,
) -> anyhow::Result<()> {
    match check_intent(&intent.constraints, access) {
        Ok(()) => {
            if let Some(utility) = utility {
                utility.apply(|i| {
                    *i += calculate_utility(&intent.directive, access)?;
                    Ok::<(), Error>(())
                })?;
            }
            Ok::<(), Error>(())
        }
        Err(e) => {
            anyhow::bail!("Constraint VM execution failed: {}", e)
        }
    }
}

/// Calculates utility of solution for intent.
///
/// Returns utility.
fn calculate_utility(directive: &Directive, access: Access) -> anyhow::Result<u64> {
    match directive {
        Directive::Satisfy => Ok(100), // TODO: verify utility range
        Directive::Maximize(code) | Directive::Minimize(code) => {
            let Ok(mut stack) = exec_bytecode_iter(code.clone(), access) else {
                anyhow::bail!("Constraint VM execution failed processing directive");
            };
            Ok(stack.pop().unwrap() as u64) // TODO: verify utility type
        }
    }
}
