use anyhow::ensure;
use essential_constraint_vm::{check_intent, exec_bytecode_iter};
use essential_state_read_vm::{
    asm::Op, Access, GasLimit, SolutionAccess, StateRead, StateSlots, Vm,
};
use essential_types::{
    intent::{Directive, Intent},
    slots::{state_len, StateSlot},
    solution::{Solution, SolutionDataIndex},
    Hash, IntentAddress, Signed, Word,
};
use std::{collections::HashMap, sync::Arc};
use storage::{state_write::StateWrite, StateStorage, Storage};
use tokio::task::JoinSet;
use transaction_storage::{Transaction, TransactionStorage};

pub use validate::validate_solution_with_deps;

mod read;
#[cfg(test)]
mod tests;
mod validate;

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
    let intents = read::read_intents_from_storage(&solution, storage)
        .await?
        .into_iter()
        .map(|(k, v)| (k, Arc::new(v)))
        .collect();
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
    intents: &HashMap<IntentAddress, Arc<Intent>>,
) -> anyhow::Result<Output<S>>
where
    S: Storage + StateStorage + StateRead + StateWrite + Clone + Send + Sync + 'static,
    <S as StateRead>::Future: Send,
    <S as StateRead>::Error: Send,
    <S as StateWrite>::Future: Send,
    <S as StateWrite>::Error: Send,
{
    // Create a transaction from storage.
    let mut transaction = storage.clone().transaction();

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

    // Create a view of the transaction.
    // TODO: This involves a single clone of the transaction.
    // Find a way to avoid this.
    let view = transaction.view();

    // Read pre and post states then check constraints.
    let mut set: JoinSet<anyhow::Result<_>> = JoinSet::new();
    for (solution_data_index, data) in solution.data.iter().enumerate() {
        let Some(intent) = intents.get(&data.intent_to_solve).cloned() else {
            anyhow::bail!("Intent in solution data not found in intents set");
        };
        let solution = solution.clone();
        let view = view.clone();
        let storage = storage.clone();
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
                    &storage,
                    state_read,
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
                    &view,
                    state_read,
                    slots,
                    &intent.slots.state,
                    state_read_index,
                )
                .await?;
            }

            // Check constraints.
            let utility = tokio::task::spawn_blocking(move || {
                let solution_access = SolutionAccess::new(&solution, solution_data_index);
                let access = Access {
                    solution: solution_access,
                    state_slots: StateSlots {
                        pre: &pre_slots,
                        post: &post_slots,
                    },
                };
                check_constraints(&intent, access)
            })
            .await??;
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
    state_read: &[u8],
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
fn check_constraints(intent: &Intent, access: Access) -> anyhow::Result<f64> {
    match check_intent(&intent.constraints, access) {
        Ok(()) => Ok(calculate_utility(intent.directive.clone(), access)?),
        Err(e) => {
            anyhow::bail!("Constraint VM execution failed: {}", e)
        }
    }
}

/// Calculates utility of solution for intent.
///
/// Returns utility.
fn calculate_utility(directive: Directive, access: Access) -> anyhow::Result<f64> {
    match directive {
        Directive::Satisfy => Ok(1.0),
        Directive::Maximize(code) | Directive::Minimize(code) => {
            let Ok(mut stack) = exec_bytecode_iter(code, access) else {
                anyhow::bail!("Constraint VM execution failed processing directive");
            };
            let [start, end, value] = stack.pop3()?;
            normalize(value, start, end)
        }
    }
}

fn normalize(value: i64, start: i64, end: i64) -> anyhow::Result<f64> {
    anyhow::ensure!(start < end, "Invalid range for directive");

    let normalized = (value - start) as f64 / (end - start) as f64;

    Ok(normalized.clamp(0.0, 1.0))
}
