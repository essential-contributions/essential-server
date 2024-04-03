//! Access operation implementations.

use crate::{
    bool_from_word, error::AccessError, ConstraintResult, SolutionAccess, Stack, StateSlotSlice,
    StateSlots,
};
use essential_constraint_asm::Word;
use essential_types::{
    convert::word_4_from_u8_32,
    solution::{DecisionVariable, SolutionData},
};

/// `Access::DecisionVar` implementation.
pub(crate) fn decision_var(solution: SolutionAccess, stack: &mut Stack) -> ConstraintResult<()> {
    stack.pop1_push1(|slot| {
        let ix = usize::try_from(slot).map_err(|_| AccessError::DecisionSlotOutOfBounds)?;
        let w = resolve_decision_var(solution.data, solution.index, ix)?;
        Ok(w)
    })
}

/// `Access::DecisionVarRange` implementation.
pub(crate) fn decision_var_range(
    solution: SolutionAccess,
    stack: &mut Stack,
) -> ConstraintResult<()> {
    let [slot, len] = stack.pop2()?;
    let range = range_from_start_len(slot, len).ok_or(AccessError::DecisionSlotOutOfBounds)?;
    for dec_var_ix in range {
        let w = resolve_decision_var(solution.data, solution.index, dec_var_ix)?;
        stack.push(w);
    }
    Ok(())
}

/// `Access::State` implementation.
pub(crate) fn state(slots: StateSlots, stack: &mut Stack) -> ConstraintResult<()> {
    stack.pop2_push1(|slot, delta| {
        let slot = state_slot(slots, slot, delta)?;
        let word = slot.ok_or(AccessError::StateSlotWasNone)?;
        Ok(word)
    })
}

/// `Access::StateRange` implementation.
pub(crate) fn state_range(slots: StateSlots, stack: &mut Stack) -> ConstraintResult<()> {
    let [slot, len, delta] = stack.pop3()?;
    let slice = state_slot_range(slots, slot, len, delta)?;
    for slot in slice {
        let word = slot.ok_or(AccessError::StateSlotWasNone)?;
        stack.push(word);
    }
    Ok(())
}

/// `Access::StateIsSome` implementation.
pub(crate) fn state_is_some(slots: StateSlots, stack: &mut Stack) -> ConstraintResult<()> {
    stack.pop2_push1(|slot, delta| {
        let slot = state_slot(slots, slot, delta)?;
        let is_some = Word::from(slot.is_some());
        Ok(is_some)
    })
}

/// `Access::StateIsSomeRange` implementation.
pub(crate) fn state_is_some_range(slots: StateSlots, stack: &mut Stack) -> ConstraintResult<()> {
    let [slot, len, delta] = stack.pop3()?;
    let slice = state_slot_range(slots, slot, len, delta)?;
    for slot in slice {
        let is_some = Word::from(slot.is_some());
        stack.push(is_some);
    }
    Ok(())
}

/// `Access::ThisAddress` implementation.
pub(crate) fn this_address(data: &SolutionData, stack: &mut Stack) {
    let words = word_4_from_u8_32(data.intent_to_solve.intent.0);
    stack.extend(words);
}

/// `Access::ThisSetAddress` implementation.
pub(crate) fn this_set_address(data: &SolutionData, stack: &mut Stack) {
    let words = word_4_from_u8_32(data.intent_to_solve.set.0);
    stack.extend(words);
}

/// Resolve the decision variable by traversing any necessary transient data.
///
/// Errors if the solution data or decision var indices are out of bounds
/// (whether provided directly or via a transient decision var) or if a cycle
/// occurs between transient decision variables.
fn resolve_decision_var(
    data: &[SolutionData],
    mut data_ix: usize,
    mut var_ix: usize,
) -> Result<Word, AccessError> {
    // Track visited vars `(data_ix, var_ix)` to ensure we do not enter a cycle.
    let mut visited = std::collections::HashSet::new();
    loop {
        let solution_data = data
            .get(data_ix)
            .ok_or(AccessError::SolutionDataOutOfBounds)?;
        let dec_var = solution_data
            .decision_variables
            .get(var_ix)
            .ok_or(AccessError::DecisionSlotOutOfBounds)?;
        match *dec_var {
            DecisionVariable::Inline(w) => return Ok(w),
            DecisionVariable::Transient(ref transient) => {
                // We're traversing transient data, so make sure we track vars already visited.
                if !visited.insert((data_ix, var_ix)) {
                    return Err(AccessError::TransientDecisionVariableCycle);
                }
                data_ix = transient.solution_data_index.into();
                var_ix = transient.variable_index.into();
            }
        }
    }
}

fn state_slot(slots: StateSlots, slot: Word, delta: Word) -> ConstraintResult<&Option<Word>> {
    let delta = bool_from_word(delta).map_err(AccessError::InvalidStateSlotDelta)?;
    let slots = state_slots_from_delta(slots, delta);
    let ix = usize::try_from(slot).map_err(|_| AccessError::StateSlotOutOfBounds)?;
    let slot = slots.get(ix).ok_or(AccessError::StateSlotOutOfBounds)?;
    Ok(slot)
}

fn state_slot_range(
    slots: StateSlots,
    slot: Word,
    len: Word,
    delta: Word,
) -> ConstraintResult<&StateSlotSlice> {
    let delta = bool_from_word(delta).map_err(AccessError::InvalidStateSlotDelta)?;
    let slots = state_slots_from_delta(slots, delta);
    let range = range_from_start_len(slot, len).ok_or(AccessError::StateSlotOutOfBounds)?;
    let subslice = slots
        .get(range)
        .ok_or(AccessError::DecisionSlotOutOfBounds)?;
    Ok(subslice)
}

fn range_from_start_len(start: Word, len: Word) -> Option<std::ops::Range<usize>> {
    let start = usize::try_from(start).ok()?;
    let len = usize::try_from(len).ok()?;
    let end = start.checked_add(len)?;
    Some(start..end)
}

fn state_slots_from_delta(slots: StateSlots, delta: bool) -> &StateSlotSlice {
    if delta {
        slots.post
    } else {
        slots.pre
    }
}
