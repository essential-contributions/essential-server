//! The essential constraint checking implementation.

use essential_constraint_asm::{self as asm, Op};
pub use essential_types::{intent::Directive, solution::SolutionData, ConstraintBytecode, Word};
use thiserror::Error;

/// All required input data for checking an intent's constraints.
#[derive(Clone, Copy, Debug)]
pub struct CheckInput<'a> {
    pub solution_data: &'a SolutionData,
    pub state_slots: &'a [Option<Word>],
}

#[derive(Debug, Error)]
pub enum CheckError {
    #[error("ALU operation error: {0}")]
    Alu(#[from] AluError),
    #[error("stack operation error: {0}")]
    Stack(#[from] StackError),
    #[error("encountered bytecode error: {0}")]
    FromBytes(#[from] asm::FromBytesError),
    #[error("invalid constraint evaluation result {0}, exepcted `0` (false) or `1` (true)")]
    InvalidConstraintValue(Word),
}

#[derive(Debug, Error)]
pub enum StackError {
    #[error("attempted to pop an empty stack")]
    Empty,
    #[error("indexed stack out of bounds")]
    IndexOutOfBounds,
}

#[derive(Debug, Error)]
pub enum AluError {
    #[error("word overflow")]
    Overflow,
    #[error("word underflow")]
    Underflow,
    #[error("attempted to divide by zero")]
    DivideByZero,
}

pub type CheckResult<T> = Result<T, CheckError>;

pub type AluResult<T> = Result<T, AluError>;

pub type Stack = Vec<Word>;

/// Check whether the constraints of a single intent are met by the given
/// solution data and state.
///
/// Returns the `Directive`, indicating the quality of the solution.
pub fn check_intent(
    intent: &[ConstraintBytecode],
    input: CheckInput,
) -> Result<Directive, CheckError> {
    intent
        .iter()
        .map(|bytecode| eval_bytecode(bytecode.iter().copied(), input))
        .fold(Ok(true), |acc, res| acc.and_then(|b| res.map(|b2| b && b2)))?;
    todo!()
}

/// Evaluate the bytecode of a single constraint and return its boolean result.
///
/// This is the same as `exec_bytecode`, but retrieves the boolean result from the resulting stack.
pub fn eval_bytecode(bytes: impl IntoIterator<Item = u8>, input: CheckInput) -> CheckResult<bool> {
    let mut stack = exec_bytecode(bytes, input)?;
    let word = pop(&mut stack)?;
    bool_from_word(word).map_err(CheckError::InvalidConstraintValue)
}

/// Evaluate the operations of a single constraint and return its boolean result.
///
/// This is the same as `exec_ops`, but retrieves the boolean result from the resulting stack.
pub fn eval_ops(ops: impl IntoIterator<Item = Op>, input: CheckInput) -> CheckResult<bool> {
    let mut stack: Stack = vec![];
    for op in ops {
        step_op(input, op, &mut stack)?;
        println!("{:?}: {:?}", op, &stack);
    }
    let word = pop(&mut stack)?;
    bool_from_word(word).map_err(CheckError::InvalidConstraintValue)
}

/// Execute the bytecode of a constraint and return the resulting stack.
pub fn exec_bytecode(bytes: impl IntoIterator<Item = u8>, input: CheckInput) -> CheckResult<Stack> {
    let mut stack: Stack = vec![];
    for res in asm::from_bytes(bytes.into_iter()) {
        let op = res?;
        step_op(input, op, &mut stack)?;
        println!("{:?}: {:?}", op, &stack);
    }
    Ok(stack)
}

/// Execute the operations of a constraint and return the resulting stack.
pub fn exec_ops(ops: impl IntoIterator<Item = Op>, input: CheckInput) -> CheckResult<Stack> {
    let mut stack: Stack = vec![];
    for op in ops {
        step_op(input, op, &mut stack)?;
        println!("{:?}: {:?}", op, &stack);
    }
    Ok(stack)
}

/// Parse a `bool` from a word, where 0 is false, 1 is true and any other value is invalid.
pub fn bool_from_word(word: Word) -> Result<bool, Word> {
    match word {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(word),
    }
}

/// Step forward constraint checking by the given operation.
pub fn step_op(input: CheckInput, op: Op, stack: &mut Stack) -> CheckResult<()> {
    match op {
        Op::Access(op) => step_op_access(input, op, stack),
        Op::Alu(op) => step_op_alu(op, stack),
        Op::Crypto(op) => step_op_crypto(op, stack),
        Op::Pred(op) => step_op_pred(op, stack),
        Op::Stack(op) => step_op_stack(op, stack),
    }
}

pub fn step_op_access(input: CheckInput, op: asm::Access, stack: &mut Stack) -> CheckResult<()> {
    match op {
        asm::Access::DecisionVar => todo!(),
        asm::Access::DecisionVarRange => todo!(),
        asm::Access::MutKeysLen => todo!(),
        asm::Access::State => todo!(),
        asm::Access::StateRange => todo!(),
        asm::Access::StateIsSome => todo!(),
        asm::Access::StateIsSomeRange => todo!(),
        asm::Access::ThisAddress => todo!(),
        asm::Access::ThisSetAddress => todo!(),
    }
}

pub fn step_op_alu(op: asm::Alu, stack: &mut Stack) -> CheckResult<()> {
    match op {
        asm::Alu::Add => pop_2_push_1(stack, alu_add),
        asm::Alu::Sub => pop_2_push_1(stack, alu_sub),
        asm::Alu::Mul => pop_2_push_1(stack, alu_mul),
        asm::Alu::Div => pop_2_push_1(stack, alu_div),
        asm::Alu::Mod => pop_2_push_1(stack, alu_mod),
    }
}

pub fn step_op_crypto(op: asm::Crypto, stack: &mut Stack) -> CheckResult<()> {
    match op {
        asm::Crypto::Sha256 => todo!(),
        asm::Crypto::VerifyEd25519 => todo!(),
    }
}

pub fn step_op_pred(op: asm::Pred, stack: &mut Stack) -> CheckResult<()> {
    match op {
        asm::Pred::Eq => pop_2_push_1(stack, |a, b| Ok((a == b).into())),
        asm::Pred::Eq4 => pop_8_push_1(stack, |[a0, a1, a2, a3, b0, b1, b2, b3]| {
            Ok(([a0, a1, a2, a3] == [b0, b1, b2, b3]).into())
        }),
        asm::Pred::Gt => pop_2_push_1(stack, |a, b| Ok((a > b).into())),
        asm::Pred::Lt => pop_2_push_1(stack, |a, b| Ok((a < b).into())),
        asm::Pred::Gte => pop_2_push_1(stack, |a, b| Ok((a >= b).into())),
        asm::Pred::Lte => pop_2_push_1(stack, |a, b| Ok((a <= b).into())),
        asm::Pred::And => pop_2_push_1(stack, |a, b| Ok((a != 0 && b != 0).into())),
        asm::Pred::Or => pop_2_push_1(stack, |a, b| Ok((a != 0 || b != 0).into())),
        asm::Pred::Not => pop_1_push_1(stack, |a| Ok((a == 0).into())),
    }
}

pub fn step_op_stack(op: asm::Stack, stack: &mut Stack) -> CheckResult<()> {
    match op {
        asm::Stack::Dup => pop_1_push_2(stack, |w| Ok([w, w])),
        asm::Stack::DupFrom => dup_from(stack),
        asm::Stack::Push(word) => Ok(stack.push(word)),
        asm::Stack::Pop => pop(stack).map(|_| ()),
        asm::Stack::Swap => pop_2_push_2(stack, |a, b| Ok([b, a])),
    }
}

fn pop(stack: &mut Stack) -> CheckResult<Word> {
    Ok(stack.pop().ok_or(StackError::Empty)?)
}

pub fn pop_1_push_1<F>(stack: &mut Stack, f: F) -> CheckResult<()>
where
    F: FnOnce(Word) -> CheckResult<Word>,
{
    let w = pop(stack)?;
    let x = f(w)?;
    stack.push(x);
    Ok(())
}

pub fn pop_2_push_1<F>(stack: &mut Stack, f: F) -> CheckResult<()>
where
    F: FnOnce(Word, Word) -> CheckResult<Word>,
{
    let w1 = pop(stack)?;
    let w0 = pop(stack)?;
    let x = f(w0, w1)?;
    stack.push(x);
    Ok(())
}

pub fn pop_8_push_1<F>(stack: &mut Stack, f: F) -> CheckResult<()>
where
    F: FnOnce([Word; 8]) -> CheckResult<Word>,
{
    let w7 = pop(stack)?;
    let w6 = pop(stack)?;
    let w5 = pop(stack)?;
    let w4 = pop(stack)?;
    let w3 = pop(stack)?;
    let w2 = pop(stack)?;
    let w1 = pop(stack)?;
    let w0 = pop(stack)?;
    let x = f([w0, w1, w2, w3, w4, w5, w6, w7])?;
    stack.push(x);
    Ok(())
}

pub fn pop_1_push_2<F>(stack: &mut Stack, f: F) -> CheckResult<()>
where
    F: FnOnce(Word) -> CheckResult<[Word; 2]>,
{
    let w = pop(stack)?;
    let [w0, w1] = f(w)?;
    stack.push(w0);
    stack.push(w1);
    Ok(())
}

pub fn pop_2_push_2<F>(stack: &mut Stack, f: F) -> CheckResult<()>
where
    F: FnOnce(Word, Word) -> CheckResult<[Word; 2]>,
{
    let w1 = pop(stack)?;
    let w0 = pop(stack)?;
    let [w0, w1] = f(w0, w1)?;
    stack.push(w0);
    stack.push(w1);
    Ok(())
}

fn dup_from(stack: &mut Stack) -> CheckResult<()> {
    let rev_ix_w = pop(stack)?;
    let rev_ix = usize::try_from(rev_ix_w).map_err(|_| StackError::IndexOutOfBounds)?;
    let ix = stack
        .len()
        .checked_sub(rev_ix)
        .and_then(|i| i.checked_sub(1))
        .ok_or(StackError::IndexOutOfBounds)?;
    let w = *stack.get(ix).ok_or(StackError::IndexOutOfBounds)?;
    stack.push(w);
    Ok(())
}

pub fn alu_add(a: Word, b: Word) -> CheckResult<Word> {
    a.checked_add(b).ok_or(AluError::Overflow.into())
}

pub fn alu_sub(a: Word, b: Word) -> CheckResult<Word> {
    a.checked_sub(b).ok_or(AluError::Underflow.into())
}

pub fn alu_mul(a: Word, b: Word) -> CheckResult<Word> {
    a.checked_mul(b).ok_or(AluError::Overflow.into())
}

pub fn alu_div(a: Word, b: Word) -> CheckResult<Word> {
    a.checked_div(b).ok_or(AluError::DivideByZero.into())
}

pub fn alu_mod(a: Word, b: Word) -> CheckResult<Word> {
    a.checked_rem(b).ok_or(AluError::DivideByZero.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use essential_constraint_asm::{to_bytes, Alu, Op, Pred, Stack};
    use essential_types::{solution::SolutionData, ContentAddress, IntentAddress};

    const TEST_SET_CA: ContentAddress = ContentAddress([0xFF; 32]);
    const TEST_INTENT_CA: ContentAddress = ContentAddress([0xAA; 32]);
    const TEST_INTENT_ADDR: IntentAddress = IntentAddress {
        set: TEST_SET_CA,
        intent: TEST_INTENT_CA,
    };

    fn empty_solution_data() -> SolutionData {
        SolutionData {
            intent_to_solve: TEST_INTENT_ADDR,
            decision_variables: vec![],
        }
    }

    fn with_test_input<O>(f: impl FnOnce(CheckInput) -> O) -> O {
        let solution_data = &empty_solution_data();
        let state_slots = &[];
        let input = CheckInput {
            solution_data,
            state_slots,
        };
        f(input)
    }

    fn eval(ops: &[Op]) -> CheckResult<bool> {
        with_test_input(|input| {
            let ops_res = eval_ops(ops.iter().cloned(), input);
            // Ensure eval_bytecode produces the same result as eval_ops.
            let bytecode: Vec<u8> = to_bytes(ops.iter().cloned()).collect();
            let bytecode_res = eval_bytecode(bytecode.iter().cloned(), input);
            if let (Ok(a), Ok(b)) = (&ops_res, &bytecode_res) {
                assert_eq!(a, b);
            }
            ops_res
        })
    }

    #[test]
    fn eval_6_mul_7_eq_42() {
        eval(&[
            Stack::Push(6).into(),
            Stack::Push(7).into(),
            Alu::Mul.into(),
            Stack::Push(42).into(),
            Pred::Eq.into(),
        ])
        .unwrap();
    }

    #[test]
    fn eval_42_div_6_eq_7() {
        eval(&[
            Stack::Push(42).into(),
            Stack::Push(7).into(),
            Alu::Div.into(),
            Stack::Push(6).into(),
            Pred::Eq.into(),
        ])
        .unwrap();
    }

    #[test]
    fn eval_divide_by_zero() {
        let res = eval(&[
            Stack::Push(42).into(),
            Stack::Push(0).into(),
            Alu::Div.into(),
        ]);
        match res {
            Err(CheckError::Alu(AluError::DivideByZero)) => (),
            _ => panic!("Unexpected error variant"),
        }
    }

    #[test]
    fn eval_add_overflow() {
        let res = eval(&[
            Stack::Push(Word::MAX).into(),
            Stack::Push(1).into(),
            Alu::Add.into(),
        ]);
        match res {
            Err(CheckError::Alu(AluError::Overflow)) => (),
            _ => panic!("Unexpected error variant"),
        }
    }

    #[test]
    fn eval_mul_overflow() {
        let res = eval(&[
            Stack::Push(Word::MAX).into(),
            Stack::Push(2).into(),
            Alu::Mul.into(),
        ]);
        match res {
            Err(CheckError::Alu(AluError::Overflow)) => (),
            _ => panic!("Unexpected error variant"),
        }
    }

    #[test]
    fn eval_sub_underflow() {
        let res = eval(&[
            Stack::Push(Word::MIN).into(),
            Stack::Push(1).into(),
            Alu::Sub.into(),
        ]);
        match res {
            Err(CheckError::Alu(AluError::Underflow)) => (),
            _ => panic!("Unexpected error variant"),
        }
    }
}
