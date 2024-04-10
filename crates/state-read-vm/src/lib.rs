//! The essential state read VM implementation.
//!
//! ## Reading State
//!
//! The primary entrypoint for this crate is the [`Vm` type][Vm].
//!
//! The `Vm` allows for executing operations that read state and apply any
//! necessary operations in order to form the final, expected state slot layout
//! within the VM's [`Memory`]. The `Vm`'s memory can be accessed directly
//! from the `Vm`, or the `Vm` can be consumed and state slots returned with
//! [`Vm::into_state_slots`].
//!
//! ## Executing Ops
//!
//! There are three primary methods available for executing operations:
//!
//! - [`Vm::exec_ops`]
//! - [`Vm::exec_bytecode`]
//! - [`Vm::exec_bytecode_iter`]
//!
//! Each have slightly different performance implications, so be sure to read
//! the docs before selecting a method.
//!
//! ## Execution Future
//!
//! The `Vm::exec_*` functions all return `Future`s that not only yield on
//! async operations, but yield based on a user-specified gas limit too. See the
//! [`ExecFuture`] docs for further details on the implementation.

#![deny(missing_docs, unsafe_code)]

#[doc(inline)]
pub use bytecode::{BytecodeMapped, BytecodeMappedSlice};
#[doc(inline)]
pub use constraint_vm::{
    self as constraint, Access, SolutionAccess, Stack, StateSlotSlice, StateSlots,
};
use error::{MemoryError, OpError, StateReadError};
#[doc(inline)]
pub use error::{MemoryResult, OpAsyncResult, OpResult, OpSyncResult, StateReadResult};
#[doc(inline)]
pub use essential_state_asm as asm;
use essential_state_asm::Op;
pub use essential_types as types;
use essential_types::{ContentAddress, Word};
#[doc(inline)]
pub use future::ExecFuture;
pub use memory::Memory;
pub use state_read::StateRead;

mod bytecode;
mod ctrl_flow;
pub mod error;
mod future;
mod memory;
mod state_read;

/// The operation execution state of the State Read VM.
#[derive(Debug, Default, PartialEq)]
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
    /// All operations available to the constraint checker.
    Constraint(asm::Constraint),
    /// Operations for controlling the flow of the program.
    ControlFlow(asm::ControlFlow),
    /// Operations for controlling the flow of the program.
    Memory(asm::Memory),
}

/// The set of operations that are performed asynchronously.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum OpAsync {
    /// Read a range of words from state starting at the key.
    StateReadWordRange,
    /// Read a range of words from external state starting at the key.
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
    /// This is a wrapper around [`Vm::exec`] that expects operation access in
    /// the form of a `&[Op]`.
    ///
    /// If memory bloat is a concern, consider using the [`Vm::exec_bytecode`]
    /// or [`Vm::exec_bytecode_iter`] methods which allow for providing a more
    /// compact representation of the operations in the form of mapped bytecode.
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
    /// of [`&BytecodeMapped`][BytecodeMapped].
    ///
    /// This can be a more memory efficient alternative to [`Vm::exec_ops`] due
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
                while self.mapped.op_indices().len() <= index {
                    match Op::from_bytes(&mut self.iter)? {
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

    /// Consumes the `Vm` and returns the read state slots.
    ///
    /// The returned slots correlate directly with the memory content.
    pub fn into_state_slots(self) -> Vec<Option<Word>> {
        self.memory.into()
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
pub(crate) fn step_op_sync(op: OpSync, access: Access, vm: &mut Vm) -> OpSyncResult<Option<usize>> {
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
pub(crate) async fn step_op_async<S>(
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
pub(crate) fn step_op_ctrl_flow(op: asm::ControlFlow, vm: &mut Vm) -> OpSyncResult<Option<usize>> {
    match op {
        asm::ControlFlow::Jump => ctrl_flow::jump(vm).map(Some).map_err(From::from),
        asm::ControlFlow::JumpIf => ctrl_flow::jump_if(vm).map(Some),
        asm::ControlFlow::Halt => Ok(None),
    }
}

/// Step forward state reading by the given memory operation.
pub(crate) fn step_op_memory(op: asm::Memory, vm: &mut Vm) -> OpSyncResult<()> {
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

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;
    use crate::types::{solution::SolutionData, ContentAddress, IntentAddress, Key, Word};
    use std::collections::BTreeMap;
    use thiserror::Error;

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

    // A test `StateRead` implementation represented using a map.
    #[derive(Clone)]
    pub(crate) struct State(BTreeMap<ContentAddress, BTreeMap<Key, Word>>);

    #[derive(Debug, Error)]
    #[error("no value for the given intent set, key pair")]
    pub struct InvalidStateRead;

    impl State {
        // Empry state, fine for tests unrelated to reading state.
        pub(crate) const EMPTY: Self = State(BTreeMap::new());

        // Shorthand test state constructor.
        pub(crate) fn new(sets: Vec<(ContentAddress, Vec<(Key, Word)>)>) -> Self {
            State(
                sets.into_iter()
                    .map(|(addr, vec)| {
                        let map: BTreeMap<_, _> = vec.into_iter().collect();
                        (addr, map)
                    })
                    .collect(),
            )
        }

        // Update the value at the given key within the given intent set address.
        pub(crate) fn set(&mut self, set_addr: ContentAddress, key: &Key, value: Option<Word>) {
            let set = self.0.entry(set_addr).or_default();
            match value {
                None => {
                    set.remove(key);
                }
                Some(value) => {
                    set.insert(*key, value);
                }
            }
        }
    }

    impl core::ops::Deref for State {
        type Target = BTreeMap<ContentAddress, BTreeMap<Key, Word>>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl StateRead for State {
        type Error = InvalidStateRead;
        async fn word_range(
            &self,
            set_addr: ContentAddress,
            mut key: Key,
            num_words: usize,
        ) -> Result<Vec<Option<Word>>, Self::Error> {
            // Get the key that follows this one.
            fn next_key(mut key: Key) -> Option<Key> {
                for w in key.iter_mut().rev() {
                    match *w {
                        Word::MAX => *w = Word::MIN,
                        _ => {
                            *w += 1;
                            return Some(key);
                        }
                    }
                }
                None
            }

            // Collect the words.
            let mut words = vec![];
            for _ in 0..num_words {
                let opt = self
                    .get(&set_addr)
                    .ok_or(InvalidStateRead)?
                    .get(&key)
                    .cloned();
                words.push(opt);
                key = next_key(key).ok_or(InvalidStateRead)?;
            }
            Ok(words)
        }
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
        let op_gas_cost = &|_: &Op| 1;
        let spent = vm
            .exec_ops(
                ops,
                TEST_ACCESS,
                &State::EMPTY,
                op_gas_cost,
                GasLimit::UNLIMITED,
            )
            .await
            .unwrap();
        assert_eq!(spent, ops.iter().map(op_gas_cost).sum());
        assert_eq!(vm.pc, ops.len());
        assert_eq!(&vm.stack[..], &[42]);
    }

    // Test that we get expected results when yielding due to gas limits.
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
            .exec_ops(
                ops,
                TEST_ACCESS,
                &State::EMPTY,
                &op_gas_cost,
                GasLimit::UNLIMITED,
            )
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
        let op_gas_cost = &|_: &Op| 1;
        let spent = vm
            .exec_ops(
                ops,
                TEST_ACCESS,
                &State::EMPTY,
                op_gas_cost,
                GasLimit::UNLIMITED,
            )
            .await
            .unwrap();
        assert_eq!(spent, ops.iter().map(op_gas_cost).sum());
        assert_eq!(vm.pc, ops.len());
        assert_eq!(&vm.stack[..], &[42]);

        // Continue executing from current state over the new ops.
        vm.pc = 0;
        let ops = &[asm::Stack::Push(6).into(), asm::Alu::Div.into()];
        let spent = vm
            .exec_ops(
                ops,
                TEST_ACCESS,
                &State::EMPTY,
                &op_gas_cost,
                GasLimit::UNLIMITED,
            )
            .await
            .unwrap();
        assert_eq!(spent, ops.iter().map(op_gas_cost).sum());
        assert_eq!(vm.pc, ops.len());
        assert_eq!(&vm.stack[..], &[7]);
    }

    // Ensure basic programs evaluate to the same thing
    #[tokio::test]
    async fn exec_method_behaviours_match() {
        // The operations of the test program.
        let ops: &[Op] = &[
            asm::Stack::Push(6).into(),
            asm::Stack::Push(7).into(),
            asm::Alu::Mul.into(),
        ];

        // Execute the ops using `exec_ops`.
        let mut vm_ops = Vm::default();
        let spent_ops = vm_ops
            .exec_ops(
                ops,
                TEST_ACCESS,
                &State::EMPTY,
                &|_: &Op| 1,
                GasLimit::UNLIMITED,
            )
            .await
            .unwrap();

        // Execute the same ops but as mapped bytecode.
        let mapped: BytecodeMapped = ops.iter().copied().collect();
        let mut vm_bc = Vm::default();
        let spent_bc = vm_bc
            .exec_bytecode(
                &mapped,
                TEST_ACCESS,
                &State::EMPTY,
                &|_: &Op| 1,
                GasLimit::UNLIMITED,
            )
            .await
            .unwrap();
        assert_eq!(spent_ops, spent_bc);
        assert_eq!(vm_ops, vm_bc);

        // Execute the same ops, but from a bytes iterator.
        let bc_iter = mapped.bytecode().iter().copied();
        let mut vm_bc_iter = Vm::default();
        let spent_bc_iter = vm_bc_iter
            .exec_bytecode_iter(
                bc_iter,
                TEST_ACCESS,
                &State::EMPTY,
                &|_: &Op| 1,
                GasLimit::UNLIMITED,
            )
            .await
            .unwrap();
        assert_eq!(spent_ops, spent_bc_iter);
        assert_eq!(vm_ops, vm_bc_iter);
    }

    // Emulate the process of reading pre state, applying mutations to produce
    // post state, and checking the constraints afterwards.
    #[tokio::test]
    async fn read_pre_post_state_and_check_constraints() {
        use crate::types::solution::{Mutation, Solution, SolutionData, StateMutation};

        let intent_addr = TEST_INTENT_ADDR;

        // In the pre-state, we have [Some(40), None, Some(42)].
        let pre_state = State::new(vec![(
            intent_addr.set.clone(),
            vec![([0, 0, 0, 0], 40), ([0, 0, 0, 2], 42)],
        )]);

        // The full solution that we're checking.
        let solution = Solution {
            data: vec![SolutionData {
                intent_to_solve: intent_addr.clone(),
                decision_variables: vec![],
            }],
            // We have one mutation that sets a missing value to 41.
            state_mutations: vec![StateMutation {
                pathway: 0,
                mutations: vec![Mutation {
                    key: [0, 0, 0, 1],
                    value: Some(41),
                }],
            }],
            partial_solutions: vec![],
        };

        // Construct access to the necessary solution data for the VM.
        let mut access = Access {
            solution: SolutionAccess {
                data: &solution.data,
                index: 0,
            },
            // Haven't calculated these yet.
            state_slots: StateSlots::EMPTY,
        };

        // A simple state read program that reads words directly to the slots.
        let ops = &[
            asm::Stack::Push(3).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(0).into(), // Key0
            asm::Stack::Push(0).into(), // Key1
            asm::Stack::Push(0).into(), // Key2
            asm::Stack::Push(0).into(), // Key3
            asm::Stack::Push(3).into(), // Num words
            asm::StateRead::WordRange,
        ];

        // Execute the program.
        let mut vm = Vm::default();
        vm.exec_ops(ops, access, &pre_state, &|_: &Op| 1, GasLimit::UNLIMITED)
            .await
            .unwrap();

        // Collect the state slots.
        let pre_state_slots = vm.into_state_slots();

        // Apply the state mutations to the state to produce the post state.
        let mut post_state = pre_state.clone();
        for mutation in solution.state_mutations {
            let solution_data = &solution.data[usize::from(mutation.pathway)];
            let set_addr = &solution_data.intent_to_solve.set;
            for Mutation { key, value } in mutation.mutations.iter() {
                post_state.set(set_addr.clone(), key, *value);
            }
        }

        // Execute the program with the post state.
        let mut vm = Vm::default();
        vm.exec_ops(ops, access, &post_state, &|_: &Op| 1, GasLimit::UNLIMITED)
            .await
            .unwrap();

        // Collect the state slots.
        let post_state_slots = vm.into_state_slots();

        // State slots should have updated.
        assert_eq!(&pre_state_slots[..], &[Some(40), None, Some(42)]);
        assert_eq!(&post_state_slots[..], &[Some(40), Some(41), Some(42)]);

        // Now, they can be used for constraint checking.
        access.state_slots = StateSlots {
            pre: &pre_state_slots[..],
            post: &post_state_slots[..],
        };
        let constraints: &[Vec<u8>] = &[
            // Check that the first pre and post slots are equal.
            constraint_vm::asm::to_bytes(vec![
                asm::Stack::Push(0).into(), // slot
                asm::Stack::Push(0).into(), // pre
                asm::Access::State.into(),
                asm::Stack::Push(0).into(), // slot
                asm::Stack::Push(1).into(), // post
                asm::Access::State.into(),
                asm::Pred::Eq.into(),
            ])
            .collect(),
            // Check that the second pre state is none, but post is some.
            constraint_vm::asm::to_bytes(vec![
                asm::Stack::Push(1).into(), // slot
                asm::Stack::Push(0).into(), // pre
                asm::Access::StateIsSome.into(),
                asm::Pred::Not.into(),
                asm::Stack::Push(1).into(), // slot
                asm::Stack::Push(1).into(), // post
                asm::Access::StateIsSome.into(),
                asm::Pred::And.into(),
            ])
            .collect(),
            // Check that the third pre and post slots are equal.
            constraint_vm::asm::to_bytes(vec![
                asm::Stack::Push(2).into(), // slot
                asm::Stack::Push(0).into(), // pre
                asm::Access::State.into(),
                asm::Stack::Push(2).into(), // slot
                asm::Stack::Push(1).into(), // post
                asm::Access::State.into(),
                asm::Pred::Eq.into(),
            ])
            .collect(),
        ];
        constraint_vm::check_intent(constraints, access).unwrap();

        // Constraints pass - we're free to apply the updated state!
    }
}
