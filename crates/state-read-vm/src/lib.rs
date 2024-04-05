//! The essential state read VM implementation.

#[doc(inline)]
pub use constraint_vm::{self as constraint, Access, Stack};
use error::{MemoryError, OpError, StateReadError};
pub use error::{MemoryResult, OpResult, StateReadResult};
#[doc(inline)]
pub use essential_state_asm as asm;
use essential_state_asm::{Op, Word};
pub use essential_types as types;
pub use memory::Memory;
pub use state_read::StateRead;

mod ctrl_flow;
pub mod error;
mod memory;
mod state_read;

/// The execution state of the State Read VM.
#[derive(Debug, Default)]
pub struct Vm {
    /// The "program counter", i.e. index of the current operation within the program.
    pub pc: usize,
    /// The stack machine.
    pub stack: Stack,
    /// The program memory, primarily used for collecting the state being read.
    pub memory: Memory,
}

// Whether or not to continue execution after successfully processing an operation.
pub type Continue = bool;

/// Execute the given bytecode starting from the first operation.
///
/// Upon reaching a `Halt` operation or reaching the end of the operation
/// sequence, returns the resulting state of the `Vm`.
pub async fn exec_bytecode<'a, S>(
    bytes: impl IntoIterator<Item = u8>,
    access: Access<'a>,
    state_read: &S,
) -> StateReadResult<Vm, S::Error>
where
    S: StateRead,
{
    // Lazily collect ops in case we need to jump back due to control flow.
    let mut iter = asm::from_bytes(bytes).enumerate();
    let mut ops = vec![];
    let mut vm = Vm::default();
    loop {
        // Ensure we have parsed enough operations to continue execution.
        while ops.len() <= vm.pc {
            let Some((ix, res)) = iter.next() else {
                break;
            };
            let op = res.map_err(|err| StateReadError::Op(ix, err.into()))?;
            ops.push(op);
        }
        let op = ops[vm.pc];
        match step_op(op, access, state_read, &mut vm)
            .await
            .map_err(|err| StateReadError::Op(vm.pc, err))?
        {
            None => break,
            Some(new_pc) => vm.pc = new_pc,
        }
    }
    Ok(vm)
}

/// Execute the given list of operations starting from the first.
///
/// Upon reaching a `Halt` operation or reaching the end of the operation
/// sequence, returns the resulting state of the `Vm`.
pub async fn exec_ops<'a, S>(
    ops: &[Op],
    access: Access<'a>,
    state_read: &S,
) -> StateReadResult<Vm, S::Error>
where
    S: StateRead,
{
    let mut vm = Vm::default();
    while let Some(&op) = ops.get(vm.pc) {
        match step_op(op, access, state_read, &mut vm)
            .await
            .map_err(|err| StateReadError::Op(vm.pc, err))?
        {
            None => break,
            Some(new_pc) => vm.pc = new_pc,
        }
    }
    Ok(vm)
}

/// Step forward state read execution by a single operation.
///
/// Returns a `Some(usize)` representing the new program counter resulting from
/// this step, or `None` in the case that execution has halted.
pub async fn step_op<'a, S>(
    op: Op,
    access: Access<'a>,
    state_read: &S,
    vm: &mut Vm,
) -> OpResult<Option<usize>, S::Error>
where
    S: StateRead,
{
    match op {
        asm::Op::Constraint(op) => constraint_vm::step_op(access, op, &mut vm.stack)?,
        asm::Op::ControlFlow(op) => return step_op_ctrl_flow(op, vm).map_err(From::from),
        asm::Op::Memory(op) => step_op_memory(op, vm)?,
        asm::Op::WordRange => state_read::word_range(state_read, vm).await?,
        asm::Op::WordRangeExtern => state_read::word_range_ext(state_read, vm).await?,
    }
    // Every operation besides control flow steps forward program counter by 1.
    let new_pc = vm.pc.checked_add(1).expect("pc can never exceeds `usize`");
    Ok(Some(new_pc))
}

/// Step forward state reading by the given control flow operation.
///
/// Returns a `bool` indicating whether or not to continue execution.
pub fn step_op_ctrl_flow<E>(
    op: asm::ControlFlow,
    vm: &mut Vm,
) -> Result<Option<usize>, OpError<E>> {
    match op {
        asm::ControlFlow::Jump => ctrl_flow::jump(vm).map(Some).map_err(From::from),
        asm::ControlFlow::JumpIf => ctrl_flow::jump_if(vm).map(Some),
        asm::ControlFlow::Halt => Ok(None),
    }
}

/// Step forward state reading by the given memory operation.
pub fn step_op_memory<E>(op: asm::Memory, vm: &mut Vm) -> Result<(), OpError<E>> {
    match op {
        asm::Memory::Alloc => memory::alloc(vm),
        asm::Memory::Capacity => memory::capacity(vm),
        asm::Memory::Clear => memory::clear(vm),
        asm::Memory::ClearRange => memory::clear_range(vm),
        asm::Memory::Free => memory::free(vm),
        asm::Memory::IsSome => memory::is_some(vm),
        asm::Memory::Length => memory::length(vm),
        asm::Memory::Load => memory::load(vm),
        asm::Memory::Push => memory::push(vm),
        asm::Memory::PushNone => memory::push_none(vm),
        asm::Memory::Store => memory::store(vm),
        asm::Memory::Truncate => memory::truncate(vm),
    }
}

/// Parse a `bool` from a word, where 0 is false, 1 is true and any other value is invalid.
fn bool_from_word(word: Word) -> Option<bool> {
    match word {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
