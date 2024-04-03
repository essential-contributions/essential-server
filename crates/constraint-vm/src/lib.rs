//! The essential constraint checking implementation.
//!
//! ## Checking Intents
//!
//! The primary entrypoint for this crate is the [`check_intent`] function
//! which allows for checking a set of constraints associated with a single
//! intent against some provided solution data and state slot mutations in
//! parallel.
//!
//! ## Checking Individual Constraints
//!
//! Functions are also exposed for checking constraints individually.
//!
//! - The [`exec_bytecode`] and [`exec_ops`] functions allow for executing the
//!   constraint and returning the resulting `Stack`.
//! - The [`eval_bytecode`] and [`eval_ops`] functions are similar to their
//!   `exec_*` counterparts, but expect the top of the `Stack` to contain a
//!   single boolean value indicating whether the constraint was satisfied (`0`
//!   for `false`, `1` for `true`) and returns this value.
//!
//! ## Performing a Single Operation
//!
//! The [`step_op`] function (and related `step_op_*` functions) are exposed to
//! allow for applying a single operation to the given stack. This can be useful
//! in the case of integrating constraint operations in a downstream VM (e.g.
//! the essential state read VM).
//!
//! ## Understanding the Assembly
//!
//! The `essential-constraint-asm` crate is re-exported as the [`asm`] module.
//! See [this module's documentation][asm] for information about the expected
//! behaviour of individual operations.
#![deny(missing_docs)]

pub use access::{Access, SolutionAccess, StateSlotSlice, StateSlots};
#[doc(inline)]
pub use error::{CheckResult, ConstraintResult, OpResult};
use error::{ConstraintError, ConstraintErrors, ConstraintsUnsatisfied};
#[doc(inline)]
pub use essential_constraint_asm as asm;
use essential_constraint_asm::{Op, Word};
pub use essential_types as types;
use essential_types::ConstraintBytecode;
#[doc(inline)]
pub use stack::Stack;

mod access;
mod alu;
mod crypto;
pub mod error;
mod stack;

/// Check whether the constraints of a single intent are met for the given
/// solution data and state slot mutations. All constraints are checked in
/// parallel.
///
/// In the case that one or more constraints fail or are unsatisfied, the
/// whole set of failed/unsatisfied constraint indices are returned within the
/// `CheckError` type.
///
/// The intent is considered to be satisfied if this function returns `Ok(())`.
pub fn check_intent(intent: &[ConstraintBytecode], access: Access) -> CheckResult<()> {
    use rayon::{iter::Either, prelude::*};
    let (failed, unsatisfied): (Vec<_>, Vec<_>) = intent
        .par_iter()
        .map(|bytecode| eval_bytecode(bytecode.iter().copied(), access))
        .enumerate()
        .filter_map(|(i, constraint_res)| match constraint_res {
            Err(err) => Some(Either::Left((i, err))),
            Ok(b) if !b => Some(Either::Right(i)),
            _ => None,
        })
        .partition_map(|either| either);
    if !failed.is_empty() {
        return Err(ConstraintErrors(failed).into());
    }
    if !unsatisfied.is_empty() {
        return Err(ConstraintsUnsatisfied(unsatisfied).into());
    }
    Ok(())
}

/// Evaluate the bytecode of a single constraint and return its boolean result.
///
/// This is the same as `exec_bytecode`, but retrieves the boolean result from the resulting stack.
pub fn eval_bytecode(
    bytes: impl IntoIterator<Item = u8>,
    access: Access,
) -> ConstraintResult<bool> {
    let stack = exec_bytecode(bytes, access)?;
    let word = match stack.last() {
        Some(&w) => w,
        None => return Err(ConstraintError::InvalidEvaluation(stack)),
    };
    bool_from_word(word).ok_or_else(|| ConstraintError::InvalidEvaluation(stack))
}

/// Evaluate the operations of a single constraint and return its boolean result.
///
/// This is the same as `exec_ops`, but retrieves the boolean result from the resulting stack.
pub fn eval_ops(ops: impl IntoIterator<Item = Op>, access: Access) -> ConstraintResult<bool> {
    let stack = exec_ops(ops, access)?;
    let word = match stack.last() {
        Some(&w) => w,
        None => return Err(ConstraintError::InvalidEvaluation(stack)),
    };
    bool_from_word(word).ok_or_else(|| ConstraintError::InvalidEvaluation(stack))
}

/// Execute the bytecode of a constraint and return the resulting stack.
pub fn exec_bytecode(
    bytes: impl IntoIterator<Item = u8>,
    access: Access,
) -> ConstraintResult<Stack> {
    let mut stack = Stack::default();
    for (ix, res) in asm::from_bytes(bytes.into_iter()).enumerate() {
        let op = res.map_err(|err| ConstraintError::Op(ix, err.into()))?;
        step_op(access, op, &mut stack).map_err(|err| ConstraintError::Op(ix, err))?;
        println!("{ix:02X}: {:016?} -> {:016X?}", op, &stack);
    }
    Ok(stack)
}

/// Execute the operations of a constraint and return the resulting stack.
pub fn exec_ops(ops: impl IntoIterator<Item = Op>, access: Access) -> ConstraintResult<Stack> {
    let mut stack = Stack::default();
    for (ix, op) in ops.into_iter().enumerate() {
        step_op(access, op, &mut stack).map_err(|err| ConstraintError::Op(ix, err))?;
        println!("{ix:02X}: {:016X?} -> {:016X?}", op, &stack);
    }
    Ok(stack)
}

/// Step forward constraint checking by the given operation.
pub fn step_op(access: Access, op: Op, stack: &mut Stack) -> OpResult<()> {
    match op {
        Op::Access(op) => step_op_access(access, op, stack),
        Op::Alu(op) => step_op_alu(op, stack),
        Op::Crypto(op) => step_op_crypto(op, stack),
        Op::Pred(op) => step_op_pred(op, stack),
        Op::Stack(op) => step_op_stack(op, stack),
    }
}

/// Step forward constraint checking by the given access operation.
pub fn step_op_access(access: Access, op: asm::Access, stack: &mut Stack) -> OpResult<()> {
    match op {
        asm::Access::DecisionVar => access::decision_var(access.solution, stack),
        asm::Access::DecisionVarRange => access::decision_var_range(access.solution, stack),
        asm::Access::MutKeysLen => todo!(),
        asm::Access::State => access::state(access.state_slots, stack),
        asm::Access::StateRange => access::state_range(access.state_slots, stack),
        asm::Access::StateIsSome => access::state_is_some(access.state_slots, stack),
        asm::Access::StateIsSomeRange => access::state_is_some_range(access.state_slots, stack),
        asm::Access::ThisAddress => {
            access::this_address(access.solution.this_data(), stack);
            Ok(())
        }
        asm::Access::ThisSetAddress => {
            access::this_set_address(access.solution.this_data(), stack);
            Ok(())
        }
    }
}

/// Step forward constraint checking by the given ALU operation.
pub fn step_op_alu(op: asm::Alu, stack: &mut Stack) -> OpResult<()> {
    match op {
        asm::Alu::Add => stack.pop2_push1(alu::add),
        asm::Alu::Sub => stack.pop2_push1(alu::sub),
        asm::Alu::Mul => stack.pop2_push1(alu::mul),
        asm::Alu::Div => stack.pop2_push1(alu::div),
        asm::Alu::Mod => stack.pop2_push1(alu::mod_),
    }
}

/// Step forward constraint checking by the given crypto operation.
pub fn step_op_crypto(op: asm::Crypto, stack: &mut Stack) -> OpResult<()> {
    match op {
        asm::Crypto::Sha256 => crypto::sha256(stack),
        asm::Crypto::VerifyEd25519 => crypto::verify_ed25519(stack),
    }
}

/// Step forward constraint checking by the given predicate operation.
pub fn step_op_pred(op: asm::Pred, stack: &mut Stack) -> OpResult<()> {
    match op {
        asm::Pred::Eq => stack.pop2_push1(|a, b| Ok((a == b).into())),
        asm::Pred::Eq4 => stack.pop8_push1(|ws| Ok((ws[0..4] == ws[4..8]).into())),
        asm::Pred::Gt => stack.pop2_push1(|a, b| Ok((a > b).into())),
        asm::Pred::Lt => stack.pop2_push1(|a, b| Ok((a < b).into())),
        asm::Pred::Gte => stack.pop2_push1(|a, b| Ok((a >= b).into())),
        asm::Pred::Lte => stack.pop2_push1(|a, b| Ok((a <= b).into())),
        asm::Pred::And => stack.pop2_push1(|a, b| Ok((a != 0 && b != 0).into())),
        asm::Pred::Or => stack.pop2_push1(|a, b| Ok((a != 0 || b != 0).into())),
        asm::Pred::Not => stack.pop1_push1(|a| Ok((a == 0).into())),
    }
}

/// Step forward constraint checking by the given stack operation.
pub fn step_op_stack(op: asm::Stack, stack: &mut Stack) -> OpResult<()> {
    match op {
        asm::Stack::Dup => stack.pop1_push2(|w| Ok([w, w])),
        asm::Stack::DupFrom => stack.dup_from(),
        asm::Stack::Push(word) => {
            stack.push(word);
            Ok(())
        }
        asm::Stack::Pop => Ok(stack.pop1().map(|_| ())?),
        asm::Stack::Swap => stack.pop2_push2(|a, b| Ok([b, a])),
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
pub(crate) mod test_util {
    use crate::{
        types::{solution::SolutionData, ContentAddress, IntentAddress},
        *,
    };

    pub(crate) const TEST_SET_CA: ContentAddress = ContentAddress([0xFF; 32]);
    pub(crate) const TEST_INTENT_CA: ContentAddress = ContentAddress([0xAA; 32]);
    pub(crate) const TEST_INTENT_ADDR: IntentAddress = IntentAddress {
        set: TEST_SET_CA,
        intent: TEST_INTENT_CA,
    };
    pub(crate) const TEST_SOLUTION_DATA: SolutionData = SolutionData {
        intent_to_solve: TEST_INTENT_ADDR,
        decision_variables: vec![],
    };
    pub(crate) const TEST_SOLUTION_ACCESS: SolutionAccess = SolutionAccess {
        data: &[TEST_SOLUTION_DATA],
        index: 0,
    };
    pub(crate) const TEST_ACCESS: Access = Access {
        solution: TEST_SOLUTION_ACCESS,
        state_slots: StateSlots::EMPTY,
    };
}
