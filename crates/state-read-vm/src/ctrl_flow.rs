//! ControlFlow operation implementations.

use crate::{
    bool_from_word,
    error::{ControlFlowError, OpResult, StackError},
    Vm,
};

/// `ControlFlow::Jump` operation.
pub fn jump(vm: &mut Vm) -> Result<usize, StackError> {
    let new_pc = vm.stack.pop()?;
    usize::try_from(new_pc).map_err(|_| StackError::IndexOutOfBounds)
}

/// `ControlFlow::JumpIf` operation.
pub fn jump_if<E>(vm: &mut Vm) -> OpResult<usize, E> {
    let [new_pc, cond] = vm.stack.pop2()?;
    let cond = bool_from_word(cond).ok_or(ControlFlowError::InvalidJumpIfCondition(cond))?;
    let new_pc = match cond {
        true => usize::try_from(new_pc).map_err(|_| StackError::IndexOutOfBounds)?,
        false => vm.pc.checked_add(1).expect("pc can never exceeds `usize`"),
    };
    Ok(new_pc)
}
