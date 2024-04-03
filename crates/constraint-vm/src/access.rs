//! Access operation implementations.

use crate::{bool_from_word, error::AccessError, OpResult, Stack};
use essential_constraint_asm::Word;
use essential_types::{
    convert::word_4_from_u8_32,
    solution::{DecisionVariable, SolutionData},
};

/// All necessary solution data and state access required to check an individual intent.
#[derive(Clone, Copy, Debug)]
pub struct Access<'a> {
    /// All necessary solution data access required to check an individual intent.
    pub solution: SolutionAccess<'a>,
    /// The pre and post mutation state slot values for the intent being solved.
    pub state_slots: StateSlots<'a>,
}

/// All necessary solution data access required to check an individual intent.
#[derive(Clone, Copy, Debug)]
pub struct SolutionAccess<'a> {
    /// The input data for each intent being solved within the solution.
    ///
    /// We require *all* intent solution data in order to handle transient
    /// decision variable access.
    pub data: &'a [SolutionData],
    /// Checking is performed for one intent at a time. This index refers to
    /// the checked intent's associated solution data within `data`.
    pub index: usize,
}

/// The pre and post mutation state slot values for the intent being solved.
#[derive(Clone, Copy, Debug)]
pub struct StateSlots<'a> {
    /// Intent state slot values before the solution's mutations are applied.
    pub pre: &'a StateSlotSlice,
    /// Intent state slot values after the solution's mutations are applied.
    pub post: &'a StateSlotSlice,
}

/// The state slots declared within the intent.
pub type StateSlotSlice = [Option<Word>];

impl<'a> SolutionAccess<'a> {
    /// The solution data associated with the intent currently being checked.
    ///
    /// **Panics** in the case that `self.index` is out of range of the `self.data` slice.
    pub fn this_data(&self) -> &SolutionData {
        self.data
            .get(self.index)
            .expect("intent index out of range of solution data")
    }
}

impl<'a> StateSlots<'a> {
    /// Empty state slots.
    pub const EMPTY: Self = Self {
        pre: &[],
        post: &[],
    };
}

/// `Access::DecisionVar` implementation.
pub(crate) fn decision_var(solution: SolutionAccess, stack: &mut Stack) -> OpResult<()> {
    stack.pop1_push1(|slot| {
        let ix = usize::try_from(slot).map_err(|_| AccessError::DecisionSlotOutOfBounds)?;
        let w = resolve_decision_var(solution.data, solution.index, ix)?;
        Ok(w)
    })
}

/// `Access::DecisionVarRange` implementation.
pub(crate) fn decision_var_range(solution: SolutionAccess, stack: &mut Stack) -> OpResult<()> {
    let [slot, len] = stack.pop2()?;
    let range = range_from_start_len(slot, len).ok_or(AccessError::DecisionSlotOutOfBounds)?;
    for dec_var_ix in range {
        let w = resolve_decision_var(solution.data, solution.index, dec_var_ix)?;
        stack.push(w);
    }
    Ok(())
}

/// `Access::State` implementation.
pub(crate) fn state(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    stack.pop2_push1(|slot, delta| {
        let slot = state_slot(slots, slot, delta)?;
        let word = slot.ok_or(AccessError::StateSlotWasNone)?;
        Ok(word)
    })
}

/// `Access::StateRange` implementation.
pub(crate) fn state_range(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    let [slot, len, delta] = stack.pop3()?;
    let slice = state_slot_range(slots, slot, len, delta)?;
    for slot in slice {
        let word = slot.ok_or(AccessError::StateSlotWasNone)?;
        stack.push(word);
    }
    Ok(())
}

/// `Access::StateIsSome` implementation.
pub(crate) fn state_is_some(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
    stack.pop2_push1(|slot, delta| {
        let slot = state_slot(slots, slot, delta)?;
        let is_some = Word::from(slot.is_some());
        Ok(is_some)
    })
}

/// `Access::StateIsSomeRange` implementation.
pub(crate) fn state_is_some_range(slots: StateSlots, stack: &mut Stack) -> OpResult<()> {
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

fn state_slot(slots: StateSlots, slot: Word, delta: Word) -> OpResult<&Option<Word>> {
    let delta = bool_from_word(delta).ok_or(AccessError::InvalidStateSlotDelta(delta))?;
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
) -> OpResult<&StateSlotSlice> {
    let delta = bool_from_word(delta).ok_or(AccessError::InvalidStateSlotDelta(slot))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        asm,
        error::{AccessError, ConstraintError, OpError},
        exec_ops,
        test_util::*,
    };
    use essential_types::solution::DecisionVariableIndex;

    #[test]
    fn decision_var_inline() {
        let access = Access {
            solution: SolutionAccess {
                data: &[SolutionData {
                    intent_to_solve: TEST_INTENT_ADDR,
                    decision_variables: vec![DecisionVariable::Inline(42)],
                }],
                index: 0,
            },
            state_slots: StateSlots::EMPTY,
        };
        let ops = &[
            asm::Stack::Push(0).into(), // Slot index.
            asm::Access::DecisionVar.into(),
        ];
        let stack = exec_ops(ops.iter().copied(), access).unwrap();
        assert_eq!(&stack[..], &[42]);
    }

    #[test]
    fn decision_var_transient() {
        // Test resolution of transient decision vars over the following path:
        // - Solution 1, Var 2 (start)
        // - Solution 0, Var 3
        // - Solution 2, Var 1
        let access = Access {
            solution: SolutionAccess {
                data: &[
                    SolutionData {
                        intent_to_solve: TEST_INTENT_ADDR,
                        decision_variables: vec![
                            DecisionVariable::Inline(0),
                            DecisionVariable::Inline(1),
                            DecisionVariable::Inline(2),
                            DecisionVariable::Transient(DecisionVariableIndex {
                                solution_data_index: 2,
                                variable_index: 1,
                            }),
                        ],
                    },
                    SolutionData {
                        intent_to_solve: TEST_INTENT_ADDR,
                        decision_variables: vec![
                            DecisionVariable::Inline(0),
                            DecisionVariable::Inline(1),
                            DecisionVariable::Transient(DecisionVariableIndex {
                                solution_data_index: 0,
                                variable_index: 3,
                            }),
                            DecisionVariable::Inline(3),
                        ],
                    },
                    SolutionData {
                        intent_to_solve: TEST_INTENT_ADDR,
                        decision_variables: vec![
                            DecisionVariable::Inline(0),
                            DecisionVariable::Inline(42),
                        ],
                    },
                ],
                // Solution data for intent being solved is at index 1.
                index: 1,
            },
            state_slots: StateSlots::EMPTY,
        };
        let ops = &[
            asm::Stack::Push(2).into(), // Slot index.
            asm::Access::DecisionVar.into(),
        ];
        let stack = exec_ops(ops.iter().copied(), access).unwrap();
        assert_eq!(&stack[..], &[42]);
    }

    #[test]
    fn decision_var_range() {
        let access = Access {
            solution: SolutionAccess {
                data: &[SolutionData {
                    intent_to_solve: TEST_INTENT_ADDR,
                    decision_variables: vec![
                        DecisionVariable::Inline(7),
                        DecisionVariable::Inline(8),
                        DecisionVariable::Inline(9),
                    ],
                }],
                index: 0,
            },
            state_slots: StateSlots::EMPTY,
        };
        let ops = &[
            asm::Stack::Push(0).into(), // Slot index.
            asm::Stack::Push(3).into(), // Range length.
            asm::Access::DecisionVarRange.into(),
        ];
        let stack = exec_ops(ops.iter().copied(), access).unwrap();
        assert_eq!(&stack[..], &[7, 8, 9]);
    }

    #[test]
    fn decision_var_range_transient() {
        let access = Access {
            solution: SolutionAccess {
                data: &[
                    SolutionData {
                        intent_to_solve: TEST_INTENT_ADDR,
                        decision_variables: vec![
                            DecisionVariable::Transient(DecisionVariableIndex {
                                solution_data_index: 1,
                                variable_index: 2,
                            }),
                            DecisionVariable::Transient(DecisionVariableIndex {
                                solution_data_index: 1,
                                variable_index: 1,
                            }),
                            DecisionVariable::Transient(DecisionVariableIndex {
                                solution_data_index: 1,
                                variable_index: 0,
                            }),
                        ],
                    },
                    SolutionData {
                        intent_to_solve: TEST_INTENT_ADDR,
                        decision_variables: vec![
                            DecisionVariable::Inline(7),
                            DecisionVariable::Inline(8),
                            DecisionVariable::Inline(9),
                        ],
                    },
                ],
                index: 0,
            },
            state_slots: StateSlots::EMPTY,
        };
        let ops = &[
            asm::Stack::Push(0).into(), // Slot index.
            asm::Stack::Push(3).into(), // Range length.
            asm::Access::DecisionVarRange.into(),
        ];
        let stack = exec_ops(ops.iter().copied(), access).unwrap();
        assert_eq!(&stack[..], &[9, 8, 7]);
    }

    #[test]
    fn decision_var_transient_cycle() {
        let access = Access {
            solution: SolutionAccess {
                data: &[
                    SolutionData {
                        intent_to_solve: TEST_INTENT_ADDR,
                        decision_variables: vec![DecisionVariable::Transient(
                            DecisionVariableIndex {
                                solution_data_index: 1,
                                variable_index: 0,
                            },
                        )],
                    },
                    SolutionData {
                        intent_to_solve: TEST_INTENT_ADDR,
                        decision_variables: vec![DecisionVariable::Transient(
                            DecisionVariableIndex {
                                solution_data_index: 0,
                                variable_index: 0,
                            },
                        )],
                    },
                ],
                index: 0,
            },
            state_slots: StateSlots::EMPTY,
        };
        let ops = &[
            asm::Stack::Push(0).into(), // Slot index.
            asm::Access::DecisionVar.into(),
        ];
        let res = exec_ops(ops.iter().copied(), access);
        match res {
            Err(ConstraintError::Op(
                _,
                OpError::Access(AccessError::TransientDecisionVariableCycle),
            )) => (),
            _ => panic!("expected transient decision variable cycle error, got {res:?}"),
        }
    }

    #[test]
    fn decision_var_slot_oob() {
        let access = Access {
            solution: SolutionAccess {
                data: &[SolutionData {
                    intent_to_solve: TEST_INTENT_ADDR,
                    decision_variables: vec![DecisionVariable::Inline(42)],
                }],
                index: 0,
            },
            state_slots: StateSlots::EMPTY,
        };
        let ops = &[
            asm::Stack::Push(1).into(), // Slot index.
            asm::Access::DecisionVar.into(),
        ];
        let res = exec_ops(ops.iter().copied(), access);
        match res {
            Err(ConstraintError::Op(_, OpError::Access(AccessError::DecisionSlotOutOfBounds))) => {
                ()
            }
            _ => panic!("expected transient decision variable cycle error, got {res:?}"),
        }
    }
}
