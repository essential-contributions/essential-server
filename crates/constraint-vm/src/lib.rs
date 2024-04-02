//! The essential constraint checking implementation.

pub use error::{CheckError, ConstraintError, ConstraintResult};
use error::{ConstraintErrors, ConstraintsUnsatisfied};
#[doc(inline)]
pub use essential_constraint_asm as asm;
use essential_constraint_asm::{Op, Word};
pub use essential_types as types;
use essential_types::{solution::SolutionData, ConstraintBytecode};
pub use stack::Stack;

mod access;
mod alu;
mod crypto;
pub mod error;
pub mod stack;

/// All required input data for access operations.
#[derive(Clone, Copy, Debug)]
pub struct Access<'a> {
    pub solution_data: &'a SolutionData,
    pub state_slots: StateSlots<'a>,
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

impl<'a> StateSlots<'a> {
    /// Empty state slots.
    pub const EMPTY: Self = Self {
        pre: &[],
        post: &[],
    };
}

/// Check whether the constraints of a single intent are met by the given
/// solution data and state.
///
/// Returns the `Directive`, indicating the quality of the solution.
pub fn check_intent(intent: &[ConstraintBytecode], access: Access) -> Result<(), CheckError> {
    let (unsatisfied, failed) = intent
        .iter()
        .map(|bytecode| eval_bytecode(bytecode.iter().copied(), access))
        .enumerate()
        .fold(
            (vec![], vec![]),
            |(mut unsatisfied, mut failed), (i, constraint_res)| {
                match constraint_res {
                    Ok(b) if !b => unsatisfied.push(i),
                    Err(err) => failed.push((i, err)),
                    _ => (),
                }
                (unsatisfied, failed)
            },
        );
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
    let mut stack = exec_bytecode(bytes, access)?;
    let word = stack.pop1()?;
    bool_from_word(word).map_err(ConstraintError::InvalidConstraintValue)
}

/// Evaluate the operations of a single constraint and return its boolean result.
///
/// This is the same as `exec_ops`, but retrieves the boolean result from the resulting stack.
pub fn eval_ops(ops: impl IntoIterator<Item = Op>, access: Access) -> ConstraintResult<bool> {
    let mut stack = exec_ops(ops, access)?;
    let word = stack.pop1()?;
    bool_from_word(word).map_err(ConstraintError::InvalidConstraintValue)
}

/// Execute the bytecode of a constraint and return the resulting stack.
pub fn exec_bytecode(
    bytes: impl IntoIterator<Item = u8>,
    access: Access,
) -> ConstraintResult<Stack> {
    let mut stack = Stack::default();
    for res in asm::from_bytes(bytes.into_iter()) {
        let op = res?;
        step_op(access, op, &mut stack)?;
        println!("{:016?}: {:016X?}", op, &stack);
    }
    Ok(stack)
}

/// Execute the operations of a constraint and return the resulting stack.
pub fn exec_ops(ops: impl IntoIterator<Item = Op>, access: Access) -> ConstraintResult<Stack> {
    let mut stack = Stack::default();
    for op in ops {
        step_op(access, op, &mut stack)?;
        println!("{:016X?}: {:016X?}", op, &stack);
    }
    Ok(stack)
}

/// Step forward constraint checking by the given operation.
pub fn step_op(access: Access, op: Op, stack: &mut Stack) -> ConstraintResult<()> {
    match op {
        Op::Access(op) => step_op_access(access, op, stack),
        Op::Alu(op) => step_op_alu(op, stack),
        Op::Crypto(op) => step_op_crypto(op, stack),
        Op::Pred(op) => step_op_pred(op, stack),
        Op::Stack(op) => step_op_stack(op, stack),
    }
}

/// Step forward constraint checking by the given access operation.
pub fn step_op_access(access: Access, op: asm::Access, stack: &mut Stack) -> ConstraintResult<()> {
    match op {
        asm::Access::DecisionVar => access::decision_var(&access.solution_data, stack),
        asm::Access::DecisionVarRange => access::decision_var_range(&access.solution_data, stack),
        asm::Access::MutKeysLen => todo!(),
        asm::Access::State => access::state(access.state_slots, stack),
        asm::Access::StateRange => access::state_range(access.state_slots, stack),
        asm::Access::StateIsSome => access::state_is_some(access.state_slots, stack),
        asm::Access::StateIsSomeRange => access::state_is_some_range(access.state_slots, stack),
        asm::Access::ThisAddress => Ok(access::this_address(access.solution_data, stack)),
        asm::Access::ThisSetAddress => Ok(access::this_set_address(access.solution_data, stack)),
    }
}

/// Step forward constraint checking by the given ALU operation.
pub fn step_op_alu(op: asm::Alu, stack: &mut Stack) -> ConstraintResult<()> {
    match op {
        asm::Alu::Add => stack.pop2_push1(alu::add),
        asm::Alu::Sub => stack.pop2_push1(alu::sub),
        asm::Alu::Mul => stack.pop2_push1(alu::mul),
        asm::Alu::Div => stack.pop2_push1(alu::div),
        asm::Alu::Mod => stack.pop2_push1(alu::mod_),
    }
}

/// Step forward constraint checking by the given crypto operation.
pub fn step_op_crypto(op: asm::Crypto, stack: &mut Stack) -> ConstraintResult<()> {
    match op {
        asm::Crypto::Sha256 => crypto::sha256(stack),
        asm::Crypto::VerifyEd25519 => crypto::verify_ed25519(stack),
    }
}

/// Step forward constraint checking by the given predicate operation.
pub fn step_op_pred(op: asm::Pred, stack: &mut Stack) -> ConstraintResult<()> {
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
pub fn step_op_stack(op: asm::Stack, stack: &mut Stack) -> ConstraintResult<()> {
    match op {
        asm::Stack::Dup => stack.pop1_push2(|w| Ok([w, w])),
        asm::Stack::DupFrom => stack.dup_from(),
        asm::Stack::Push(word) => Ok(stack.push(word)),
        asm::Stack::Pop => Ok(stack.pop1().map(|_| ())?),
        asm::Stack::Swap => stack.pop2_push2(|a, b| Ok([b, a])),
    }
}

/// Parse a `bool` from a word, where 0 is false, 1 is true and any other value is invalid.
fn bool_from_word(word: Word) -> Result<bool, Word> {
    match word {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(word),
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    use crate::{
        asm,
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
    pub(crate) const TEST_ACCESS: Access = Access {
        solution_data: &TEST_SOLUTION_DATA,
        state_slots: StateSlots::EMPTY,
    };

    // Similar to `eval_ops` but tests roundtrip convert to/from bytecode and checks results match.
    pub(crate) fn eval(ops: &[Op], access: Access) -> ConstraintResult<bool> {
        let ops_res = eval_ops(ops.iter().cloned(), access);
        // Ensure eval_bytecode produces the same result as eval_ops.
        let bytecode: Vec<u8> = asm::to_bytes(ops.iter().cloned()).collect();
        let bytecode_res = eval_bytecode(bytecode.iter().cloned(), access);
        if let (Ok(a), Ok(b)) = (&ops_res, &bytecode_res) {
            assert_eq!(a, b);
        }
        ops_res
    }
}
