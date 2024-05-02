use anyhow::ensure;
use essential_constraint_vm::{eval_bytecode_iter, exec_bytecode_iter};
use essential_state_read_vm::{
    asm::Op, Access, BytecodeMapped, GasLimit, SolutionAccess, StateRead, StateSlots, Vm,
};
use essential_types::{
    intent::{Directive, Intent},
    slots::{state_len, StateSlot},
    solution::{Solution, SolutionDataIndex},
    Hash, IntentAddress, Signed, Word,
};
use std::{collections::HashMap, sync::Arc};
use storage::{StateStorage, Storage};
use tokio::task::JoinSet;
use transaction_storage::TransactionStorage;

pub use validate::validate_solution_with_deps;

pub(crate) mod read;
#[cfg(test)]
mod tests;
pub(crate) mod validate;

pub struct Output<S: StateStorage> {
    pub transaction: TransactionStorage<S>,
    pub utility: f64,
    pub gas_used: u64,
}

enum ReadType {
    Pre,
    Post,
}

struct Slots<'a> {
    pre: &'a mut [Option<Word>],
    post: &'a mut [Option<Word>],
}

struct SlotsRead<'a> {
    slots: Slots<'a>,
    read_type: ReadType,
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

/// Checks constraints of a solution.
///
/// Simulates state mutations proposed by the solution and performs constraint checks
/// over pre-mutation and post-mutation state.
///
/// Unlike `check_solution`, this function takes a set of intents to check against.
/// This is useful when intents have already been read from storage, e.g. during solution validation.
///
/// Returns utility score of solution.
pub async fn check_solution_with_intents<S>(
    mut transaction: TransactionStorage<S>,
    solution: Arc<Solution>,
    intents: &HashMap<IntentAddress, Arc<Intent>>,
) -> anyhow::Result<Output<S>>
where
    S: StateStorage + StateRead + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
{
    // Create a view of the transaction before state mutations.
    let pre_state = transaction.view();

    // Apply state mutations.
    for state_mutation in &solution.state_mutations {
        let set = &solution
            .data
            .get(state_mutation.pathway as usize)
            .ok_or(anyhow::anyhow!("Intent in solution data not found"))?
            .intent_to_solve
            .set;
        for mutation in state_mutation.mutations.iter() {
            transaction.apply_state(set, &mutation.key, mutation.value);
        }
    }

    // Create a view of the transaction after state mutations.
    let post_state = transaction.view();

    // Read pre and post states then check constraints.
    let mut set: JoinSet<anyhow::Result<_>> = JoinSet::new();
    for (solution_data_index, data) in solution.data.iter().enumerate() {
        let Some(intent) = intents.get(&data.intent_to_solve).cloned() else {
            anyhow::bail!("Intent in solution data not found in intents set");
        };
        let solution = solution.clone();
        let pre_state = pre_state.clone();
        let post_state = post_state.clone();
        let solution_data_index: SolutionDataIndex = solution_data_index.try_into()?;

        set.spawn(async move {
            // Get the length of state slots for this intent.
            let intent_state_len: usize = state_len(&intent.slots.state)
                .ok_or(anyhow::anyhow!("State slots have no length"))?
                .try_into()?;

            let mut total_gas = 0;

            // Initialize pre and post slots.
            let mut pre_slots: Vec<Option<Word>> = vec![None; intent_state_len];
            let mut post_slots: Vec<Option<Word>> = vec![None; intent_state_len];
            let solution_access = SolutionAccess::new(&solution, solution_data_index);

            // Read pre and post states.
            for (state_read_index, state_read) in intent.state_read.iter().enumerate() {
                let state_read_index: u16 = state_read_index.try_into()?;

                // Map the bytecode ops ahead of execution to share the mapping
                // between both pre and post state slot reads.
                let state_read_mapped = BytecodeMapped::try_from(&state_read[..])?;

                // Read pre state
                let slots = SlotsRead {
                    slots: Slots {
                        pre: &mut pre_slots,
                        post: &mut post_slots,
                    },
                    read_type: ReadType::Pre,
                };
                total_gas += read_state_for(
                    solution_access,
                    &pre_state,
                    &state_read_mapped,
                    slots,
                    &intent.slots.state,
                    state_read_index,
                )
                .await?;

                // Read post state
                let slots = SlotsRead {
                    slots: Slots {
                        pre: &mut pre_slots,
                        post: &mut post_slots,
                    },
                    read_type: ReadType::Post,
                };
                total_gas += read_state_for(
                    solution_access,
                    &post_state,
                    &state_read_mapped,
                    slots,
                    &intent.slots.state,
                    state_read_index,
                )
                .await?;
            }

            // Check constraints.
            let utility = check_constraints(
                intent.clone(),
                pre_slots,
                post_slots,
                solution,
                solution_data_index,
            )
            .await?;
            Ok((utility, total_gas))
        });
    }

    // Calculate total utility and gas used.
    // TODO: Gas is only calculated for state reads.
    // Add gas tracking for constraint checking.
    let mut total_gas: u64 = 0;
    let mut utility: f64 = 0.0;
    while let Some(res) = set.join_next().await {
        let (u, g) = res??;
        utility += u;

        // Ensure utility does not overflow.
        ensure!(utility != f64::INFINITY, "Utility overflow");

        total_gas = total_gas
            .checked_add(g)
            .ok_or(anyhow::anyhow!("Gas overflow"))?;
    }

    Ok(Output {
        transaction,
        utility,
        gas_used: total_gas,
    })
}

/// Read the state for the pre or post state.
async fn read_state_for<S>(
    solution_access: SolutionAccess<'_>,
    storage: &S,
    state_read: &BytecodeMapped<&[u8]>,
    mut slots: SlotsRead<'_>,
    state_slots: &[StateSlot],
    state_read_index: u16,
) -> anyhow::Result<u64>
where
    S: StateRead + Send + Sync + 'static,
{
    // Create a new state read VM.
    let mut vm = Vm::default();
    let access = Access {
        solution: solution_access,
        state_slots: StateSlots {
            pre: slots.slots.pre,
            post: slots.slots.post,
        },
    };

    // Read the state.
    match read_state(&mut vm, state_read, access, storage).await {
        Ok(gas) => {
            // Write to the correct post/pre slots.
            write_slots(
                state_read_index,
                state_slots,
                slots.get_mut(),
                &vm.into_state_slots(),
            )?;
            Ok(gas)
        }
        Err(e) => anyhow::bail!("State read VM execution failed: {}", e),
    }
}

impl SlotsRead<'_> {
    /// Get mutable reference to slots based on read type.
    fn get_mut(&mut self) -> &mut [Option<Word>] {
        match self.read_type {
            ReadType::Pre => self.slots.pre,
            ReadType::Post => self.slots.post,
        }
    }
}

/// Write to the correct slots based on state read index.
fn write_slots(
    state_read_index: u16,
    state_slots: &[StateSlot],
    slots: &mut [Option<Word>],
    output_slots: &[Option<Word>],
) -> anyhow::Result<()> {
    // Find the correct state slot based matching the state read index
    // with the program index.
    let Some(slots) = state_slots
        .iter()
        .find(|slot| slot.program_index == state_read_index)
        .and_then(|slot| {
            let start: usize = slot.index.try_into().ok()?;
            let end: usize = slot.amount.try_into().ok()?;
            let end = end.checked_add(start)?;

            slots.get_mut(start..end)
        })
    else {
        anyhow::bail!("State slot not found for state read program");
    };

    // The length of the output slots must match the length of the slots
    // that are being written to.
    anyhow::ensure!(
        slots.len() == output_slots.len(),
        "State slot length mismatch"
    );

    // Write the output slots to the correct position in the slots.
    for (i, o) in slots.iter_mut().zip(output_slots.iter()) {
        *i = *o;
    }
    Ok(())
}

/// Reads state slots from storage using state read program.
///
/// The result is written to VM's memory and can be accessed
/// either using `Vm::into_state_slots` or by reading memory directly.
///
/// Returns gas used by VM.
async fn read_state<S>(
    vm: &mut Vm,
    state_read: &BytecodeMapped<&[u8]>,
    access: Access<'_>,
    storage: &S,
) -> anyhow::Result<u64>
where
    S: StateRead,
{
    vm.exec_bytecode(
        state_read,
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
async fn check_constraints(
    intent: Arc<Intent>,
    pre_slots: Vec<Option<Word>>,
    post_slots: Vec<Option<Word>>,
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
) -> anyhow::Result<f64> {
    let pre_slots = Arc::new(pre_slots);
    let post_slots = Arc::new(post_slots);
    match check_intent(
        intent.clone(),
        pre_slots.clone(),
        post_slots.clone(),
        solution.clone(),
        solution_data_index,
    )
    .await
    {
        Ok(()) => Ok(calculate_utility(
            intent.clone(),
            pre_slots,
            post_slots,
            solution,
            solution_data_index,
        )
        .await?),
        Err(e) => {
            anyhow::bail!("Constraint VM execution failed: {}", e)
        }
    }
}

/// Check intents in parallel without sleeping
/// any threads.
async fn check_intent(
    intent: Arc<Intent>,
    pre_slots: Arc<Vec<Option<Word>>>,
    post_slots: Arc<Vec<Option<Word>>>,
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
) -> anyhow::Result<()> {
    let mut handles = Vec::with_capacity(intent.constraints.len());

    // Spawn each constraint onto a rayon thread and
    // check them in parallel.
    for i in 0..intent.constraints.len() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        handles.push(rx);

        // These are all cheap Arc clones.
        let solution = solution.clone();
        let pre_slots = pre_slots.clone();
        let post_slots = post_slots.clone();
        let intent = intent.clone();

        // Spawn this sync code onto a rayon thread.
        // This is a non-blocking operation.
        rayon::spawn(move || {
            let solution_access = SolutionAccess::new(&solution, solution_data_index);
            let access = Access {
                solution: solution_access,
                state_slots: StateSlots {
                    pre: &pre_slots,
                    post: &post_slots,
                },
            };
            let res = eval_bytecode_iter(
                intent
                    .constraints
                    .get(i)
                    .expect("Safe due to above len check")
                    .iter()
                    .copied(),
                access,
            );

            // Send the result back to the main thread.
            // Send errors are ignored as if the recv is gone there's no one to send to.
            let _ = tx.send((i, res));
        })
    }

    // There's no way to know the size of these.
    let mut failed = Vec::new();
    let mut unsatisfied = Vec::new();

    // Wait for all constraints to finish.
    // The order of waiting on handles is not important as all
    // constraints make progress independently.
    for handle in handles {
        // Get the index and result from the handle.
        let (i, res): (usize, Result<bool, _>) = handle.await?;
        match res {
            // If the constraint failed, add it to the failed list.
            Err(err) => failed.push((i, err)),
            // If the constraint was unsatisfied, add it to the unsatisfied list.
            Ok(b) if !b => unsatisfied.push(i),
            // Otherwise, the constraint was satisfied.
            _ => (),
        }
    }

    // If there are any failed constraints, return an error.
    if !failed.is_empty() {
        return Err(essential_constraint_vm::error::ConstraintErrors(failed).into());
    }

    // If there are any unsatisfied constraints, return an error.
    if !unsatisfied.is_empty() {
        return Err(essential_constraint_vm::error::ConstraintsUnsatisfied(unsatisfied).into());
    }
    Ok(())
}

/// Calculates utility of solution for intent.
///
/// Returns utility.
async fn calculate_utility(
    intent: Arc<Intent>,
    pre_slots: Arc<Vec<Option<Word>>>,
    post_slots: Arc<Vec<Option<Word>>>,
    solution: Arc<Solution>,
    solution_data_index: SolutionDataIndex,
) -> anyhow::Result<f64> {
    match &intent.directive {
        Directive::Satisfy => Ok(1.0),
        Directive::Maximize(_) | Directive::Minimize(_) => {
            // Spawn this sync code onto a rayon thread.
            let (tx, rx) = tokio::sync::oneshot::channel();
            rayon::spawn(move || {
                let solution_access = SolutionAccess::new(&solution, solution_data_index);
                let access = Access {
                    solution: solution_access,
                    state_slots: StateSlots {
                        pre: &pre_slots,
                        post: &post_slots,
                    },
                };
                // Extract the directive code.
                let code = match intent.directive {
                    Directive::Maximize(ref code) | Directive::Minimize(ref code) => code,
                    _ => unreachable!("As this is already checked above"),
                };

                // Execute the directive code.
                match exec_bytecode_iter(code.iter().copied(), access) {
                    Ok(mut stack) => match stack.pop3() {
                        Ok([start, end, value]) => {
                            // Return the normalized value back to the main thread.
                            // Send errors are ignored as if the recv is gone there's no one to send to.
                            let _ = tx.send(normalize(value, start, end));
                        }
                        Err(e) => {
                            // Return the error back to the main thread.
                            // Send errors are ignored as if the recv is gone there's no one to send to.
                            let _ = tx.send(Err(e.into()));
                        }
                    },
                    Err(e) => {
                        // Return the error back to the main thread.
                        // Send errors are ignored as if the recv is gone there's no one to send to.
                        let _ = tx.send(Err(e.into()));
                    }
                }
            });

            // Await the result of the utility calculation.
            rx.await?
        }
    }
}

fn normalize(value: i64, start: i64, end: i64) -> anyhow::Result<f64> {
    anyhow::ensure!(start < end, "Invalid range for directive");

    let normalized = (value - start) as f64 / (end - start) as f64;

    Ok(normalized.clamp(0.0, 1.0))
}
