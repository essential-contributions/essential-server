use self::validate::validate_solution_with_deps;
use essential_constraint_vm::{Access, SolutionAccess, StateSlots};
use essential_state_read_vm::{asm::Op, GasLimit, StateRead, Vm};
use essential_types::{solution::Solution, Hash, Signed, Word};
use storage::Storage;
use tokio::task::JoinSet;

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

// TODO: S: Send + 'static
// TODO: Vm does not implement Copy
// TODO: f64 to return?
pub async fn check_solution<S>(storage: &S, solution: Solution) -> anyhow::Result<f64>
where
    S: Storage + StateRead + Clone + Send + 'static,
{
    let intents = read::read_intents_from_storage(&solution, storage).await?;
    let vm = Vm::default();

    let mut set = JoinSet::new();

    for (index, data) in solution.data.iter().enumerate() {
        let Some(intent) = intents.get(&data.intent_to_solve).cloned() else {
            anyhow::bail!("Intent in solution data not found in intents set");
        };

        let mut_keys_len = solution
            .state_mutations
            .iter()
            .filter(|sm| sm.pathway as usize == index)
            .count();
        let mut_keys_len: Word = mut_keys_len.try_into()?;

        let data = solution.data.clone();
        let storage = storage.clone();

        set.spawn(async move {
            let pre_state = vec![];
            let post_state = vec![];

            for state_read in &intent.state_read {
                let solution_access = SolutionAccess {
                    data: data.as_slice(),
                    index,
                    mut_keys_len,
                };

                let access = Access {
                    solution: solution_access,
                    state_slots: StateSlots {
                        pre: pre_state.as_slice(),
                        post: post_state.as_slice(),
                    },
                };

                let result = vm
                    .exec_bytecode_iter(
                        state_read.iter().cloned(),
                        access,
                        &storage,
                        &|_: &Op| 1,
                        GasLimit::UNLIMITED,
                    )
                    .await;
            }
        });
    }

    Ok(0.0)
}
