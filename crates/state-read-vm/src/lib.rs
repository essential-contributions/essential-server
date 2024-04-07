//! The essential state read VM implementation.

#[doc(inline)]
pub use constraint_vm::{self as constraint, Access, Stack};
use core::iter::Enumerate;
use error::{MemoryError, StateReadError};
pub use error::{MemoryResult, OpAsyncResult, OpResult, OpSyncResult, StateReadResult};
#[doc(inline)]
pub use essential_state_asm as asm;
use essential_state_asm::{FromBytesError, Op, Word};
pub use essential_types as types;
pub use memory::Memory;
pub use state_read::StateRead;

mod ctrl_flow;
pub mod error;
pub mod future;
mod memory;
mod state_read;

/// The operation execution state of the State Read VM.
#[derive(Debug, Default)]
pub struct OpVm {
    /// The "program counter", i.e. index of the current operation within the program.
    pub pc: usize,
    /// The stack machine.
    pub stack: Stack,
    /// The program memory, primarily used for collecting the state being read.
    pub memory: Memory,
}

/// A wrapper around the `OpVm` dedicated to lazily parsing and executing from bytecode.
#[derive(Debug, Default)]
pub struct BytecodeVm<I> {
    /// The iterator producing ops by parsing bytes.
    from_bytes: I,
    /// The Vec used to lazily collect ops in case control flow would require jumping back.
    pub ops: Vec<Op>,
    /// The main VM execution state.
    pub op_vm: OpVm,
}

/// Returned by gas-bounded execution, describes the reason for yielding.
#[derive(Debug, Eq, Hash, PartialEq)]
pub enum Yield {
    /// Reached the end of the sequence of operations.
    Complete,
    /// Not enough gas to execute the operation at the current program counter.
    OutOfGas {
        /// The amount of remaining gas that was insufficient to execute the op.
        remaining_gas: Gas,
    },
}

/// Unit used to measure gas.
pub type Gas = u64;

/// Gas limits.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GasLimit {
    /// The amount that may be spent synchronously until the execution future should yield.
    pub per_yield: Gas,
    /// The total amount of gas that may be spent.
    pub total: Gas,
}

/// Distinguish between sync and async ops to ease `Future` implementation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum OpKind {
    /// Operations that yield immediately.
    Sync(OpSync),
    /// Operations returning a future.
    Async(OpAsync),
}

/// The set of operations performed synchronously.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum OpSync {
    Constraint(asm::Constraint),
    ControlFlow(asm::ControlFlow),
    Memory(asm::Memory),
}

/// The set of operations that are performed asynchronously.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum OpAsync {
    StateReadWordRange,
    StateReadWordRangeExt,
}

impl GasLimit {
    /// Adjust this to match recommended poll time limit on supported validator
    /// hardware.
    pub const DEFAULT_PER_YIELD: Gas = 4_096;

    /// Unlimited gas limit with default gas-per-yield.
    pub const UNLIMITED: Self = Self {
        per_yield: Self::DEFAULT_PER_YIELD,
        total: Gas::MAX,
    };
}

impl OpVm {
    /// Execute the given operations from the current state of the VM.
    ///
    /// Upon reaching a `Halt` operation or reaching the end of the operation
    /// sequence, returns the gas spent and the `Vm` will be left in the
    /// resulting state.
    pub async fn exec<S, F>(
        &mut self,
        ops: Vec<Op>,
        access: Access<'static>,
        state_read: S,
        op_gas_cost: F,
        gas_limit: GasLimit,
    ) -> Result<Gas, StateReadError<S::Error>>
    where
        S: 'static + Clone + StateRead,
        F: Fn(&Op) -> Gas,
    {
        (async {
            let vm = std::mem::replace(self, Self::default());
            let (vm, res) =
                future::ExecFuture::boxed(vm, ops, access, op_gas_cost, state_read, gas_limit)
                    .await;
            *self = vm;
            res
        })
        .await
    }
}

impl<I> BytecodeVm<I> {
    /// Execute the given bytecode from the current state of the given VM until
    /// either the given `remaining_gas` limit is spent, a `Halt` instruction is
    /// reached, or there are no more ops.
    pub async fn exec_bounded<'a, S>(
        &mut self,
        access: Access<'a>,
        state_read: &S,
        op_gas_cost: impl Fn(&Op) -> Gas,
        mut remaining_gas: Gas,
    ) -> StateReadResult<Yield, S::Error>
    where
        S: StateRead,
        I: Iterator<Item = (usize, Result<Op, asm::FromBytesError>)>,
    {
        loop {
            // Lazily collect enough ops to continue execution.
            while self.ops.len() <= self.op_vm.pc {
                let Some((ix, res)) = self.from_bytes.next() else {
                    break;
                };
                let op = res.map_err(|err| StateReadError::Op(ix, err.into()))?;
                self.ops.push(op);
            }

            // Retrieve the current operation.
            let op = self.ops[self.op_vm.pc];

            // Yield early if we have insufficient gas.
            remaining_gas = match remaining_gas.checked_sub(op_gas_cost(&op)) {
                None => return Ok(Yield::OutOfGas { remaining_gas }),
                Some(gas) => gas,
            };

            // Step forward the virtual machine by the operation.
            match step_op(op, access, state_read, &mut self.op_vm)
                .await
                .map_err(|err| StateReadError::Op(self.op_vm.pc, err))?
            {
                None => break Ok(Yield::Complete),
                Some(new_pc) => self.op_vm.pc = new_pc,
            }
        }
    }
}

impl From<Op> for OpKind {
    fn from(op: Op) -> Self {
        match op {
            Op::Constraint(op) => OpKind::Sync(OpSync::Constraint(op)),
            Op::ControlFlow(op) => OpKind::Sync(OpSync::ControlFlow(op)),
            Op::Memory(op) => OpKind::Sync(OpSync::Memory(op)),
            Op::WordRange => OpKind::Async(OpAsync::StateReadWordRange),
            Op::WordRangeExtern => OpKind::Async(OpAsync::StateReadWordRangeExt),
        }
    }
}

/// Construct a State Read VM from bytes.
pub fn from_bytes<I>(
    bytes: I,
) -> BytecodeVm<Enumerate<impl Iterator<Item = Result<Op, FromBytesError>>>>
where
    I: IntoIterator<Item = u8>,
{
    BytecodeVm {
        from_bytes: asm::from_bytes(bytes.into_iter()).enumerate(),
        ops: vec![],
        op_vm: OpVm::default(),
    }
}

/// Execute the given bytecode starting from the first operation.
///
/// Upon reaching a `Halt` operation or reaching the end of the operation
/// sequence, returns the resulting parsed `Op`s along with the end state of
/// the VM.
pub async fn exec_bytecode<'a, S>(
    bytes: impl IntoIterator<Item = u8>,
    access: Access<'a>,
    state_read: &S,
) -> StateReadResult<(Vec<Op>, OpVm), S::Error>
where
    S: StateRead,
{
    let mut vm = from_bytes(bytes);
    loop {
        if let Yield::Complete = vm
            .exec_bounded(access, state_read, |_op| 1, Gas::MAX)
            .await?
        {
            return Ok((vm.ops, vm.op_vm));
        }
    }
}

/// Execute the given list of operations starting from the first.
///
/// Upon reaching a `Halt` operation or reaching the end of the operation
/// sequence, returns the resulting state of the `Vm`.
pub async fn exec_ops<'a, S>(
    ops: &[Op],
    access: Access<'a>,
    state_read: &S,
) -> StateReadResult<OpVm, S::Error>
where
    S: StateRead,
{
    todo!()
    // let mut vm = OpVm::default();
    // let ops = ops.to_vec();
    // vm.exec(ops, access, state_read, |_op| 1, GasLimit::UNLIMITED)
    //     .await
    //     .map(|_gas| vm)
}

/// Step forward the VM by a single operation.
///
/// Returns a `Some(usize)` representing the new program counter resulting from
/// this step, or `None` in the case that execution has halted.
pub async fn step_op<'a, S>(
    op: Op,
    access: Access<'a>,
    state_read: S,
    vm: &mut OpVm,
) -> OpResult<Option<usize>, S::Error>
where
    S: StateRead,
{
    match OpKind::from(op) {
        OpKind::Sync(op) => step_op_sync(op, access, vm).map_err(From::from),
        OpKind::Async(op) => step_op_async(op, state_read, vm)
            .await
            .map(Some)
            .map_err(From::from),
    }
}

/// Step forward the VM by a single synchronous operation.
///
/// Returns a `Some(usize)` representing the new program counter resulting from
/// this step, or `None` in the case that execution has halted.
pub fn step_op_sync(op: OpSync, access: Access, vm: &mut OpVm) -> OpSyncResult<Option<usize>> {
    match op {
        OpSync::Constraint(op) => constraint_vm::step_op(access, op, &mut vm.stack)?,
        OpSync::ControlFlow(op) => return step_op_ctrl_flow(op, vm).map_err(From::from),
        OpSync::Memory(op) => step_op_memory(op, &mut *vm)?,
    }
    // Every operation besides control flow steps forward program counter by 1.
    let new_pc = vm.pc.checked_add(1).expect("pc can never exceed `usize`");
    Ok(Some(new_pc))
}

/// Step forward the VM by a single asynchronous operation.
///
/// Returns a `usize` representing the new program counter resulting from this step.
pub async fn step_op_async<S>(
    op: OpAsync,
    state_read: S,
    vm: &mut OpVm,
) -> OpAsyncResult<usize, S::Error>
where
    S: StateRead,
{
    match op {
        OpAsync::StateReadWordRange => state_read::word_range(state_read, &mut *vm).await?,
        OpAsync::StateReadWordRangeExt => state_read::word_range_ext(state_read, &mut *vm).await?,
    }
    // Every operation besides control flow steps forward program counter by 1.
    let new_pc = vm.pc.checked_add(1).expect("pc can never exceed `usize`");
    Ok(new_pc)
}

/// Step forward state reading by the given control flow operation.
///
/// Returns a `bool` indicating whether or not to continue execution.
pub fn step_op_ctrl_flow(op: asm::ControlFlow, vm: &mut OpVm) -> OpSyncResult<Option<usize>> {
    match op {
        asm::ControlFlow::Jump => ctrl_flow::jump(vm).map(Some).map_err(From::from),
        asm::ControlFlow::JumpIf => ctrl_flow::jump_if(vm).map(Some),
        asm::ControlFlow::Halt => Ok(None),
    }
}

/// Step forward state reading by the given memory operation.
pub fn step_op_memory(op: asm::Memory, vm: &mut OpVm) -> OpSyncResult<()> {
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
