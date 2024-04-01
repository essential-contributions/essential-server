//! Access operation functions.

use crate::{pop2, CheckInput, CheckResult, Stack};
use essential_constraint_asm::Word;
use essential_types::solution::DecisionVariable;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AccessError {
    #[error("decision variable slot out of bounds")]
    DecisionSlotOutOfBounds,
}

pub(crate) fn decision_var(input: CheckInput, slot: Word) -> CheckResult<Word> {
    let ix = usize::try_from(slot).map_err(|_| AccessError::DecisionSlotOutOfBounds)?;
    let dec_var = input
        .solution_data
        .decision_variables
        .get(ix)
        .ok_or(AccessError::DecisionSlotOutOfBounds)?;
    match *dec_var {
        DecisionVariable::Inline(w) => Ok(w),
        DecisionVariable::Transient(ref _dec_var_ix) => {
            todo!("we must pass in all solution data to support transient decision variables")
        }
    }
}

pub(crate) fn decision_var_range(input: CheckInput, stack: &mut Stack) -> CheckResult<()> {
    let [slot, len] = pop2(stack)?;
    let len = usize::try_from(len).map_err(|_| AccessError::DecisionSlotOutOfBounds)?;
    let start = usize::try_from(slot).map_err(|_| AccessError::DecisionSlotOutOfBounds)?;
    let end = start
        .checked_add(len)
        .ok_or(AccessError::DecisionSlotOutOfBounds)?;
    let range = start..end;
    let iter = input
        .solution_data
        .decision_variables
        .get(range)
        .ok_or(AccessError::DecisionSlotOutOfBounds)?;
    for dec_var in iter {
        let w = match *dec_var {
            DecisionVariable::Inline(w) => w,
            DecisionVariable::Transient(ref _dec_var_ix) => {
                todo!("we must pass in all solution data to support transient decision variables")
            }
        };
        stack.push(w);
    }
    Ok(())
}

pub(crate) fn state(input: CheckInput, slot: Word, delta: Word) -> CheckResult<Word> {
    todo!()
}
