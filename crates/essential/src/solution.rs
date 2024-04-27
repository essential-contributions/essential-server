use self::validate::validate_solution_with_deps;
use essential_constraint_vm::{check_intent, exec_bytecode_iter};
use essential_state_read_vm::{
    asm::Op, Access, GasLimit, SolutionAccess, StateRead, StateSlots, Vm,
};
use essential_types::{
    intent::{Directive, Intent},
    solution::Solution,
    Hash, IntentAddress, Signed,
};
use std::{collections::HashMap, sync::Arc};
use storage::Storage;
use tokio::task::JoinSet;
use utils::Lock;

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

/// Checks a solution against state read VM and constraint VM.
///
/// Unlike `check_solution`, this function takes a set of intents to check against.
/// This is useful when intents have already been read from storage, e.g. during solution validation.
/// Returns utility score of solution.
pub async fn check_solution_with_intents<S>(
    storage: &S,
    solution: Arc<Solution>,
    intents: &HashMap<IntentAddress, Intent>,
) -> anyhow::Result<u64>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
{
    let utility = Arc::new(Lock::new(0));
    let mut set = JoinSet::new();

    for (intent_index, data) in solution.data.iter().enumerate() {
        let Some(intent) = intents.get(&data.intent_to_solve).cloned() else {
            anyhow::bail!("Intent in solution data not found in intents set");
        };
        let solution = solution.clone();
        let storage = storage.clone();
        let utility = utility.clone();

        set.spawn(async move {
            for state_read in &intent.state_read {
                let mut vm = Vm::default();
                let solution_access = SolutionAccess::new(&solution, intent_index.try_into().unwrap());
                let access = Access {
                    solution: solution_access,
                    state_slots: StateSlots::EMPTY,
                };

                match vm
                    .exec_bytecode_iter(
                        state_read.iter().cloned(),
                        access,
                        &storage,
                        &|_: &Op| 1,
                        GasLimit::UNLIMITED,
                    )
                    .await
                {
                    Ok(_gas) => {
                        match check_intent(&intent.constraints, access) {
                            Ok(()) => {
                                utility.apply(|inner| {
                                    let score = match &intent.directive {
                                        Directive::Satisfy => 100, // TODO: verify utility range
                                        Directive::Maximize(code) | Directive::Minimize(code) => {
                                            let Ok(mut stack) = exec_bytecode_iter(code.clone(), access) else {
                                                anyhow::bail!("Constraint VM execution failed processing directive");
                                            };
                                            stack.pop().unwrap() as u64 // TODO: verify utility type
                                        }
                                    };
                                    *inner += score;
                                    Ok(())
                                })?;
                            }
                            Err(e) => {
                                anyhow::bail!("Constraint VM execution failed: {}", e)
                            }
                        };
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

/// Checks a solution against state read VM and constraint VM after reading intents from storage.
///
/// Returns utility score of solution.
pub async fn check_solution<S>(storage: &S, solution: Arc<Solution>) -> anyhow::Result<u64>
where
    S: Storage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
{
    // Read intents from storage.
    let intents = read::read_intents_from_storage(&solution, storage).await?;
    check_solution_with_intents(storage, solution, &intents).await
}
