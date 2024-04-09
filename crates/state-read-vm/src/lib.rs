//! The essential state read VM implementation.

#[doc(inline)]
pub use bytecode::{BytecodeMapped, BytecodeMappedSlice};
#[doc(inline)]
pub use constraint_vm::{
    self as constraint, Access, SolutionAccess, Stack, StateSlotSlice, StateSlots,
};
use error::{MemoryError, OpError, StateReadError};
pub use error::{MemoryResult, OpAsyncResult, OpResult, OpSyncResult, StateReadResult};
#[doc(inline)]
pub use essential_state_asm as asm;
use essential_state_asm::{Op, Word};
pub use essential_types::{self as types, ContentAddress};
pub use memory::Memory;
pub use state_read::StateRead;

mod bytecode;
mod ctrl_flow;
pub mod error;
pub mod future;
mod memory;
mod state_read;

/// The operation execution state of the State Read VM.
#[derive(Debug, Default)]
pub struct Vm {
    /// The "program counter", i.e. index of the current operation within the program.
    pub pc: usize,
    /// The stack machine.
    pub stack: Stack,
    /// The program memory, primarily used for collecting the state being read.
    pub memory: Memory,
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

/// Types that provide access to operations.
///
/// Implementations are included for `&[Op]`, `BytecodeMapped` and more.
pub trait OpAccess {
    /// Any error that might occur during access.
    type Error: std::error::Error;
    /// Access the operation at the given index.
    ///
    /// Mutable access to self is required in case operations are lazily parsed.
    ///
    /// Any implementation should ensure the same index always returns the same operation.
    fn op_access(&mut self, index: usize) -> Option<Result<Op, Self::Error>>;
}

/// A mapping from an operation to its gas cost.
pub trait OpGasCost {
    /// The gas cost associated with the given op.
    fn op_gas_cost(&self, op: &Op) -> Gas;
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

impl Vm {
    /// Execute the given operations from the current state of the VM.
    ///
    /// This is a wrapper around [`Vm::exec`] that expects operation access in the form
    /// of a `&[Op]`.
    pub async fn exec_ops<'a, S>(
        &mut self,
        ops: &[Op],
        access: Access<'a>,
        state_read: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, StateReadError<S::Error>>
    where
        S: StateRead,
    {
        self.exec(access, state_read, ops, op_gas_cost, gas_limit)
            .await
    }

    /// Execute the given mapped bytecode from the current state of the VM.
    ///
    /// This is a wrapper around `exec` that expects operation access in the form
    /// of `&BytecodeMapped`.
    ///
    /// This can be a more memory efficient alternative to `Vm::exec_ops` due
    /// to the compact representation of operations in the form of bytecode and
    /// indices.
    pub async fn exec_bytecode<'a, S>(
        &mut self,
        bytecode_mapped: &BytecodeMapped,
        access: Access<'a>,
        state_read: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, StateReadError<S::Error>>
    where
        S: StateRead,
    {
        self.exec(access, state_read, bytecode_mapped, op_gas_cost, gas_limit)
            .await
    }

    /// Execute the given bytecode from the current state of the VM.
    ///
    /// The given bytecode will be mapped lazily during execution. This
    /// can be more efficient than pre-mapping the bytecode and using
    /// [`Vm::exec_bytecode`] in the case that execution may fail early.
    ///
    /// However, successful execution still requires building the full
    /// [`BytecodeMapped`] instance internally. So if bytecode has already been
    /// mapped, [`Vm::exec_bytecode`] should be preferred.
    pub async fn exec_bytecode_iter<'a, S, I>(
        &mut self,
        bytecode_iter: I,
        access: Access<'a>,
        state_read: &S,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, StateReadError<S::Error>>
    where
        S: StateRead,
        I: IntoIterator<Item = u8>,
    {
        /// A type wrapper around `BytecodeMapped` that lazily constructs the
        /// map from the given bytecode as operations are accessed.
        struct BytecodeMappedLazy<I> {
            mapped: BytecodeMapped,
            iter: I,
        }

        // Op access lazily populates operations from the given byte iterator
        // as necessary.
        impl<I> OpAccess for BytecodeMappedLazy<I>
        where
            I: Iterator<Item = u8>,
        {
            type Error = asm::FromBytesError;
            fn op_access(&mut self, index: usize) -> Option<Result<Op, Self::Error>> {
                while self.mapped.op_indices().len() < index {
                    match bytecode::parse_op(&mut self.iter)? {
                        Err(err) => return Some(Err(err)),
                        Ok(op) => self.mapped.push_op(op),
                    }
                }
                self.mapped.op(index).map(Ok)
            }
        }

        let bytecode_lazy = BytecodeMappedLazy {
            mapped: BytecodeMapped::default(),
            iter: bytecode_iter.into_iter(),
        };
        self.exec(access, state_read, bytecode_lazy, op_gas_cost, gas_limit)
            .await
    }

    /// Execute over the given operation access from the current state of the VM.
    ///
    /// Upon reaching a `Halt` operation or reaching the end of the operation
    /// sequence, returns the gas spent and the `Vm` will be left in the
    /// resulting state.
    ///
    /// The type requirements for the `op_access` argument can make this
    /// finicky to use directly. You may prefer one of wrapper methods:
    /// [`Vm::exec_ops`], [`Vm::exec_bytecode`] or [`Vm::exec_bytecode_iter`].
    pub async fn exec<'a, S, OA>(
        &mut self,
        access: Access<'a>,
        state_read: &S,
        op_access: OA,
        op_gas_cost: &impl OpGasCost,
        gas_limit: GasLimit,
    ) -> Result<Gas, StateReadError<S::Error>>
    where
        S: StateRead,
        OA: OpAccess,
        OA::Error: Into<OpError<S::Error>>,
    {
        future::exec_boxed(self, access, state_read, op_access, op_gas_cost, gas_limit).await
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

impl<F> OpGasCost for F
where
    F: Fn(&Op) -> Gas,
{
    fn op_gas_cost(&self, op: &Op) -> Gas {
        (*self)(op)
    }
}

impl<'a> OpAccess for &'a [Op] {
    type Error = core::convert::Infallible;
    fn op_access(&mut self, index: usize) -> Option<Result<Op, Self::Error>> {
        self.get(index).copied().map(Ok)
    }
}

impl<'a> OpAccess for &'a BytecodeMapped {
    type Error = core::convert::Infallible;
    fn op_access(&mut self, index: usize) -> Option<Result<Op, Self::Error>> {
        self.op(index).map(Ok)
    }
}

/// Step forward the VM by a single synchronous operation.
///
/// Returns a `Some(usize)` representing the new program counter resulting from
/// this step, or `None` in the case that execution has halted.
pub fn step_op_sync(op: OpSync, access: Access, vm: &mut Vm) -> OpSyncResult<Option<usize>> {
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
    set_addr: &ContentAddress,
    state_read: &S,
    vm: &mut Vm,
) -> OpAsyncResult<usize, S::Error>
where
    S: StateRead,
{
    match op {
        OpAsync::StateReadWordRange => {
            state_read::word_range(state_read, set_addr, &mut *vm).await?
        }
        OpAsync::StateReadWordRangeExt => state_read::word_range_ext(state_read, &mut *vm).await?,
    }
    // Every operation besides control flow steps forward program counter by 1.
    let new_pc = vm.pc.checked_add(1).expect("pc can never exceed `usize`");
    Ok(new_pc)
}

/// Step forward state reading by the given control flow operation.
///
/// Returns a `bool` indicating whether or not to continue execution.
pub fn step_op_ctrl_flow(op: asm::ControlFlow, vm: &mut Vm) -> OpSyncResult<Option<usize>> {
    match op {
        asm::ControlFlow::Jump => ctrl_flow::jump(vm).map(Some).map_err(From::from),
        asm::ControlFlow::JumpIf => ctrl_flow::jump_if(vm).map(Some),
        asm::ControlFlow::Halt => Ok(None),
    }
}

/// Step forward state reading by the given memory operation.
pub fn step_op_memory(op: asm::Memory, vm: &mut Vm) -> OpSyncResult<()> {
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
pub(crate) mod test_util {
    use super::*;
    use crate::types::{solution::SolutionData, ContentAddress, IntentAddress, Key};

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

    // A test `StateRead` implementation that returns random yet deterministic
    // option values for every possible set, key and range combination.
    pub(crate) struct State;

    // A test `StateRead` implementation that always returns a
    // random-yet-deterministic `Some` value for every possible set, key and
    // range combination.
    pub(crate) struct StateSome;

    // A test `StateRead` implementation that always returns `None`.
    pub(crate) struct StateNone;

    // A test `StateRead` implementation that always returns `Some(42)`.
    pub(crate) struct State42;

    impl StateRead for State {
        type Error = core::convert::Infallible;
        async fn word_range(
            &self,
            set_addr: ContentAddress,
            key: Key,
            num_words: usize,
        ) -> Result<Vec<Option<Word>>, Self::Error> {
            use rand::{Rng, SeedableRng};
            let seed = state_rng_seed(set_addr, key);
            let mut rng = rand::rngs::SmallRng::from_seed(seed);
            let words = (0..num_words).map(move |_| rng.gen()).collect();
            Ok(words)
        }
    }

    impl StateRead for StateSome {
        type Error = core::convert::Infallible;
        async fn word_range(
            &self,
            set_addr: ContentAddress,
            key: Key,
            num_words: usize,
        ) -> Result<Vec<Option<Word>>, Self::Error> {
            use rand::{Rng, SeedableRng};
            let seed = state_rng_seed(set_addr, key);
            let mut rng = rand::rngs::SmallRng::from_seed(seed);
            let words = (0..num_words).map(move |_| Some(rng.gen())).collect();
            Ok(words)
        }
    }

    impl StateRead for StateNone {
        type Error = core::convert::Infallible;
        async fn word_range(
            &self,
            _set_addr: ContentAddress,
            _key: Key,
            num_words: usize,
        ) -> Result<Vec<Option<Word>>, Self::Error> {
            Ok(vec![None; num_words])
        }
    }

    impl StateRead for State42 {
        type Error = core::convert::Infallible;
        async fn word_range(
            &self,
            _set_addr: ContentAddress,
            _key: Key,
            num_words: usize,
        ) -> Result<Vec<Option<Word>>, Self::Error> {
            Ok(vec![Some(42); num_words])
        }
    }

    // For now, pretend all ops cost 1 gas.
    pub(crate) fn op_gas_cost(_op: &Op) -> Gas {
        1
    }

    // Derive a deterministic RNG seed from an intent set content address and key.
    fn state_rng_seed(set_addr: ContentAddress, key: Key) -> [u8; 32] {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::default();
        set_addr.hash(&mut hasher);
        key.hash(&mut hasher);
        let hash = hasher.finish().to_be_bytes();
        let mut seed = vec![];
        seed.extend(hash);
        seed.extend(hash);
        seed.extend(hash);
        seed.extend(hash);
        seed.try_into().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::*;

    // A simple sanity test to check basic functionality.
    #[tokio::test]
    async fn no_yield() {
        let mut vm = Vm::default();
        let ops = &[
            asm::Stack::Push(6).into(),
            asm::Stack::Push(7).into(),
            asm::Alu::Mul.into(),
        ];
        let spent = vm
            .exec_ops(ops, TEST_ACCESS, &State, &op_gas_cost, GasLimit::UNLIMITED)
            .await
            .unwrap();
        assert_eq!(spent, ops.iter().map(op_gas_cost).sum());
        assert_eq!(vm.pc, ops.len());
        assert_eq!(&vm.stack[..], &[42]);
    }

    // Test that we get exepcted results when yielding due to gas limits.
    #[tokio::test]
    async fn yield_per_op() {
        let mut vm = Vm::default();
        let ops = &[
            asm::Stack::Push(6).into(),
            asm::Stack::Push(7).into(),
            asm::Alu::Mul.into(),
        ];
        // Force the VM to yield after every op to test behaviour.
        let op_gas_cost = |_op: &_| GasLimit::DEFAULT_PER_YIELD;
        let spent = vm
            .exec_ops(ops, TEST_ACCESS, &State, &op_gas_cost, GasLimit::UNLIMITED)
            .await
            .unwrap();
        assert_eq!(spent, ops.iter().map(op_gas_cost).sum());
        assert_eq!(vm.pc, ops.len());
        assert_eq!(&vm.stack[..], &[42]);
    }

    // Test VM behaves as expected when continuing execution over more operations.
    #[tokio::test]
    async fn continue_execution() {
        let mut vm = Vm::default();

        // Execute first set of ops.
        let ops = &[
            asm::Stack::Push(6).into(),
            asm::Stack::Push(7).into(),
            asm::Alu::Mul.into(),
        ];
        let spent = vm
            .exec_ops(ops, TEST_ACCESS, &State, &op_gas_cost, GasLimit::UNLIMITED)
            .await
            .unwrap();
        assert_eq!(spent, ops.iter().map(op_gas_cost).sum());
        assert_eq!(vm.pc, ops.len());
        assert_eq!(&vm.stack[..], &[42]);

        // Continue executing from current state over the new ops.
        vm.pc = 0;
        let ops = &[asm::Stack::Push(6).into(), asm::Alu::Div.into()];
        let spent = vm
            .exec_ops(ops, TEST_ACCESS, &State, &op_gas_cost, GasLimit::UNLIMITED)
            .await
            .unwrap();
        assert_eq!(spent, ops.iter().map(op_gas_cost).sum());
        assert_eq!(vm.pc, ops.len());
        assert_eq!(&vm.stack[..], &[7]);
    }
}
