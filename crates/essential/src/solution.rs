use self::validate::validate_solution_with_deps;
use essential_constraint_vm::{check_intent, exec_bytecode_iter};
use essential_state_read_vm::{
    asm::Op, Access, GasLimit, SolutionAccess, StateRead, StateSlots, Vm,
};
use essential_types::{
    intent::{Directive, Intent},
    slots::state_len,
    solution::Solution,
    Hash, IntentAddress, Signed, Word,
};
use std::{collections::HashMap, sync::Arc};
use storage::{state_write::StateWrite, StateStorage, Storage};
use tokio::task::JoinSet;
use transaction_storage::{Transaction, TransactionStorage};

mod read;
#[cfg(test)]
mod tests;
mod validate;

pub struct Output<S: StateStorage> {
    transaction: TransactionStorage<S>,
    utility: u64,
}

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
pub async fn check_solution<S>(storage: &S, solution: Arc<Solution>) -> anyhow::Result<Output<S>>
where
    S: Storage + StateStorage + StateRead + StateWrite + Clone + Send + Sync + 'static,
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
) -> anyhow::Result<Output<S>>
where
    S: Storage + StateStorage + StateRead + StateWrite + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
    <S as StateWrite>::Future: Send,
    <S as StateWrite>::Error: Send,
{
    let mut transaction = storage.clone().transaction();

    // Read pre-state
    let mut set: JoinSet<anyhow::Result<Vec<Option<Word>>>> = JoinSet::new();
    let solution = solution.clone();
    for (intent_index, data) in solution.data.iter().enumerate() {
        let Some(intent) = intents.get(&data.intent_to_solve).cloned() else {
            anyhow::bail!("Intent in solution data not found in intents set");
        };
        for state_read in intent.state_read {
            let solution = solution.clone();
            let storage = storage.clone();
            let intent_state_len = state_len(&intent.slots.state).unwrap() as usize;
            set.spawn(async move {
                let solution_access =
                    SolutionAccess::new(&solution, intent_index.try_into().unwrap());
                let pre_slots: Vec<Option<Word>> = vec![None; intent_state_len];
                let post_slots: Vec<Option<Word>> = vec![];
                let mut vm = Vm::default();
                let access = Access {
                    solution: solution_access,
                    state_slots: StateSlots {
                        pre: &pre_slots,
                        post: &post_slots,
                    },
                };
                match read_state(&mut vm, &state_read, access, &storage).await {
                    Ok(_gas) => {}
                    Err(e) => anyhow::bail!("State read VM execution failed: {}", e),
                }
                Ok(vm.into_state_slots())
            });
        }
    }
    let mut pre_slots = vec![];
    while let Some(res) = set.join_next().await {
        pre_slots.extend(res??);
    }

    // Use transactional storage to simulate state changes
    let mut set: JoinSet<anyhow::Result<()>> = JoinSet::new();
    let solution = solution.clone();
    for state_mutation in solution.state_mutations.iter() {
        let intent = &solution
            .data
            .get(state_mutation.pathway as usize)
            .unwrap()
            .intent_to_solve
            .set;
        for mutation in state_mutation.mutations.iter() {
            // TODO: spawn
            transaction
                .update_state(intent, &mutation.key, mutation.value)
                .await?;
        }
    }
    while let Some(res) = set.join_next().await {
        res??;
    }

    // Read post-state
    let mut set: JoinSet<anyhow::Result<Vec<Option<Word>>>> = JoinSet::new();
    let pre_slots = pre_slots.clone();
    for (intent_index, data) in solution.data.iter().enumerate() {
        let Some(intent) = intents.get(&data.intent_to_solve).cloned() else {
            anyhow::bail!("Intent in solution data not found in intents set");
        };
        for state_read in intent.state_read {
            let solution = solution.clone();
            let pre_slots = pre_slots.clone();
            let intent_state_len = state_len(&intent.slots.state).unwrap() as usize;
            let transaction = transaction.view();
            set.spawn(async move {
                let solution_access =
                    SolutionAccess::new(&solution, intent_index.try_into().unwrap());
                let mut vm = Vm::default();
                let access = Access {
                    solution: solution_access,
                    state_slots: StateSlots {
                        pre: &pre_slots,
                        post: &vec![None; intent_state_len],
                    },
                };
                read_state(&mut vm, &state_read, access, &transaction).await?;
                Ok(vm.into_state_slots())
            });
        }
    }
    let mut post_slots = vec![];
    while let Some(res) = set.join_next().await {
        post_slots.extend(res??);
    }

    // Check constraints.
    let mut set: JoinSet<anyhow::Result<u64>> = JoinSet::new();
    for (intent_index, data) in solution.data.iter().enumerate() {
        let Some(intent) = intents.get(&data.intent_to_solve).cloned() else {
            anyhow::bail!("Intent in solution data not found in intents set");
        };
        let solution = solution.clone();
        let pre_slots = pre_slots.clone();
        let post_slots = post_slots.clone();
        set.spawn_blocking(move || {
            let solution_access = SolutionAccess::new(&solution, intent_index.try_into().unwrap());
            let access = Access {
                solution: solution_access,
                state_slots: StateSlots {
                    pre: &pre_slots,
                    post: &post_slots,
                },
            };
            check_constraints(&intent, access)
        });
    }
    let mut utility = 0;
    while let Some(res) = set.join_next().await {
        utility += res??;
    }

    // Rollback changes
    transaction.rollback();

    Ok(Output {
        transaction,
        utility,
    })
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

/// Checks intent constraints against its state slots.
///
/// Returns the utility of solution for intent.
fn check_constraints(intent: &Intent, access: Access) -> anyhow::Result<u64> {
    match check_intent(&intent.constraints, access) {
        Ok(()) => Ok(calculate_utility(&intent.directive, access)?),
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
