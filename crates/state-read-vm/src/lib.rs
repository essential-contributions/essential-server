//! The essential state read VM implementation.

#[doc(inline)]
pub use constraint_vm::{self as constraint, Access, Stack};
use error::{ControlFlowError, MemoryError, StateReadError};
pub use error::{OpResult, StateReadResult};
#[doc(inline)]
pub use essential_state_asm as asm;
use essential_state_asm::Op;
pub use essential_types as types;
pub use memory::Memory;
pub use state_read::StateRead;

pub mod error;
mod memory;
mod state_read;

#[derive(Debug, Default)]
pub struct Vm {
    pub memory: Memory,
    pub stack: Stack,
    /// The "program counter", i.e. index of the current operation within the whole program.
    pub pc: usize,
}

// Whether or not to continue execution after successfully processing an operation.
pub type Continue = bool;

/// Execute the given operations list of operations starting from the first.
///
/// Upon reaching a `Halt` operation or reaching the end of the operation
/// sequence, returns the resulting state of the `Vm`.
pub fn exec_ops<S>(ops: &[Op], access: Access, state_read: &S) -> StateReadResult<Vm, S::Error>
where
    S: StateRead,
{
    let mut vm = Vm::default();
    while step_op(ops, state_read, &mut vm).map_err(|err| StateReadError::Op(vm.pc, err))? {}
    Ok(vm)
}

/// Step forward the state of the `Vm` by a single operation.
pub fn step_op<S>(ops: &[Op], state_read: &S, vm: &mut Vm) -> OpResult<Continue, S::Error>
where
    S: StateRead,
{
    let op = ops[vm.pc];
    match op {
        asm::Op::Constraint(op) => constraint_vm::step_op(todo!(), op, &mut vm.stack)?,
        asm::Op::ControlFlow(op) => {
            let cont = step_op_ctrl_flow(op, vm)?;
            return Ok(cont);
        }
        asm::Op::Memory(op) => step_op_memory(op, vm)?,
        asm::Op::WordRange => state_read::word_range(state_read, vm)?,
        asm::Op::WordRangeExtern => state_read::word_range_ext(state_read, vm)?,
    }
    // Every operation besides control flow steps forward program counter by 1.
    vm.pc += 1;
    Ok(true)
}

pub fn step_op_ctrl_flow(op: asm::ControlFlow, vm: &mut Vm) -> Result<Continue, ControlFlowError> {
    todo!()
}

pub fn step_op_memory(op: asm::Memory, vm: &mut Vm) -> Result<(), MemoryError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
}
