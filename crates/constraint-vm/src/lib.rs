//! The essential constraint checking implementation.

pub use error::CheckError;
use error::{AluError, StackError};
pub use essential_constraint_asm as asm;
use essential_constraint_asm::{Op, Word};
pub use essential_types::{
    intent::Directive,
    solution::{DecisionVariable, SolutionData},
    ConstraintBytecode,
};

mod access;
mod crypto;
pub mod error;

/// All required input data for checking an intent's constraints.
#[derive(Clone, Copy, Debug)]
pub struct CheckInput<'a> {
    pub solution_data: &'a SolutionData,
    pub pre_state: &'a StateSlots,
    pub post_state: &'a StateSlots,
}

/// Shorthand for a `Result` where the error type is a `CheckError`.
pub type CheckResult<T> = Result<T, CheckError>;

/// The VM's `Stack` is just a `Vec` of `Word`s.
pub type Stack = Vec<Word>;

/// The state slots declared within the intent.
pub type StateSlots = [Option<Word>];

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
    let mut stack = exec_ops(ops, input)?;
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
fn bool_from_word(word: Word) -> Result<bool, Word> {
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

/// Step forward constraint checking by the given access operation.
pub fn step_op_access(input: CheckInput, op: asm::Access, stack: &mut Stack) -> CheckResult<()> {
    match op {
        asm::Access::DecisionVar => access::decision_var(input, stack),
        asm::Access::DecisionVarRange => access::decision_var_range(input, stack),
        asm::Access::MutKeysLen => todo!(),
        asm::Access::State => access::state(input, stack),
        asm::Access::StateRange => access::state_range(input, stack),
        asm::Access::StateIsSome => access::state_is_some(input, stack),
        asm::Access::StateIsSomeRange => access::state_is_some_range(input, stack),
        asm::Access::ThisAddress => Ok(access::this_address(input, stack)),
        asm::Access::ThisSetAddress => Ok(access::this_set_address(input, stack)),
    }
}

/// Step forward constraint checking by the given ALU operation.
pub fn step_op_alu(op: asm::Alu, stack: &mut Stack) -> CheckResult<()> {
    match op {
        asm::Alu::Add => pop2_push1(stack, alu_add),
        asm::Alu::Sub => pop2_push1(stack, alu_sub),
        asm::Alu::Mul => pop2_push1(stack, alu_mul),
        asm::Alu::Div => pop2_push1(stack, alu_div),
        asm::Alu::Mod => pop2_push1(stack, alu_mod),
    }
}

/// Step forward constraint checking by the given crypto operation.
pub fn step_op_crypto(op: asm::Crypto, stack: &mut Stack) -> CheckResult<()> {
    match op {
        asm::Crypto::Sha256 => crypto::sha256(stack),
        asm::Crypto::VerifyEd25519 => crypto::verify_ed25519(stack),
    }
}

/// Step forward constraint checking by the given predicate operation.
pub fn step_op_pred(op: asm::Pred, stack: &mut Stack) -> CheckResult<()> {
    match op {
        asm::Pred::Eq => pop2_push1(stack, |a, b| Ok((a == b).into())),
        asm::Pred::Eq4 => pop8_push1(stack, |ws| Ok((ws[0..4] == ws[4..8]).into())),
        asm::Pred::Gt => pop2_push1(stack, |a, b| Ok((a > b).into())),
        asm::Pred::Lt => pop2_push1(stack, |a, b| Ok((a < b).into())),
        asm::Pred::Gte => pop2_push1(stack, |a, b| Ok((a >= b).into())),
        asm::Pred::Lte => pop2_push1(stack, |a, b| Ok((a <= b).into())),
        asm::Pred::And => pop2_push1(stack, |a, b| Ok((a != 0 && b != 0).into())),
        asm::Pred::Or => pop2_push1(stack, |a, b| Ok((a != 0 || b != 0).into())),
        asm::Pred::Not => pop1_push1(stack, |a| Ok((a == 0).into())),
    }
}

/// Step forward constraint checking by the given stack operation.
pub fn step_op_stack(op: asm::Stack, stack: &mut Stack) -> CheckResult<()> {
    match op {
        asm::Stack::Dup => pop1_push2(stack, |w| Ok([w, w])),
        asm::Stack::DupFrom => dup_from(stack),
        asm::Stack::Push(word) => Ok(stack.push(word)),
        asm::Stack::Pop => pop(stack).map(|_| ()),
        asm::Stack::Swap => pop2_push2(stack, |a, b| Ok([b, a])),
    }
}

fn pop(stack: &mut Stack) -> CheckResult<Word> {
    Ok(stack.pop().ok_or(StackError::Empty)?)
}

fn pop2(stack: &mut Stack) -> CheckResult<[Word; 2]> {
    let w1 = pop(stack)?;
    let w0 = pop(stack)?;
    Ok([w0, w1])
}

fn pop3(stack: &mut Stack) -> CheckResult<[Word; 3]> {
    let w2 = pop(stack)?;
    let [w0, w1] = pop2(stack)?;
    Ok([w0, w1, w2])
}

fn pop4(stack: &mut Stack) -> CheckResult<[Word; 4]> {
    let w3 = pop(stack)?;
    let [w0, w1, w2] = pop3(stack)?;
    Ok([w0, w1, w2, w3])
}

fn pop8(stack: &mut Stack) -> CheckResult<[Word; 8]> {
    let [w4, w5, w6, w7] = pop4(stack)?;
    let [w0, w1, w2, w3] = pop4(stack)?;
    Ok([w0, w1, w2, w3, w4, w5, w6, w7])
}

pub fn pop1_push1<F>(stack: &mut Stack, f: F) -> CheckResult<()>
where
    F: FnOnce(Word) -> CheckResult<Word>,
{
    let w = pop(stack)?;
    let x = f(w)?;
    stack.push(x);
    Ok(())
}

pub fn pop2_push1<F>(stack: &mut Stack, f: F) -> CheckResult<()>
where
    F: FnOnce(Word, Word) -> CheckResult<Word>,
{
    let [w0, w1] = pop2(stack)?;
    let x = f(w0, w1)?;
    stack.push(x);
    Ok(())
}

pub fn pop8_push1<F>(stack: &mut Stack, f: F) -> CheckResult<()>
where
    F: FnOnce([Word; 8]) -> CheckResult<Word>,
{
    let ws = pop8(stack)?;
    let x = f(ws)?;
    stack.push(x);
    Ok(())
}

pub fn pop1_push2<F>(stack: &mut Stack, f: F) -> CheckResult<()>
where
    F: FnOnce(Word) -> CheckResult<[Word; 2]>,
{
    let w = pop(stack)?;
    let xs = f(w)?;
    stack.extend(xs);
    Ok(())
}

pub fn pop2_push2<F>(stack: &mut Stack, f: F) -> CheckResult<()>
where
    F: FnOnce(Word, Word) -> CheckResult<[Word; 2]>,
{
    let [w0, w1] = pop2(stack)?;
    let xs = f(w0, w1)?;
    stack.extend(xs);
    Ok(())
}

pub fn pop2_push4<F>(stack: &mut Stack, f: F) -> CheckResult<()>
where
    F: FnOnce(Word, Word) -> CheckResult<[Word; 4]>,
{
    let [w0, w1] = pop2(stack)?;
    let xs = f(w0, w1)?;
    stack.extend(xs);
    Ok(())
}

/// Pop a length value from the top of the stack.
pub fn pop_len(stack: &mut Stack) -> CheckResult<usize> {
    let len_word = pop(stack)?;
    let len = usize::try_from(len_word).map_err(|_| StackError::IndexOutOfBounds)?;
    Ok(len)
}

/// Pop the length from the top of the stack, then pop and return that many words.
pub fn pop_len_words<F, O>(stack: &mut Stack, f: F) -> CheckResult<O>
where
    F: FnOnce(&[Word]) -> CheckResult<O>,
{
    let len = pop_len(stack)?;
    let ix = stack
        .len()
        .checked_sub(len)
        .ok_or(StackError::IndexOutOfBounds)?;
    f(&stack[ix..])
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
        let pre_state = &[];
        let post_state = &[];
        let input = CheckInput {
            solution_data,
            pre_state,
            post_state,
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
