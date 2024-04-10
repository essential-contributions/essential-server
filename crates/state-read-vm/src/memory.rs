//! Memory operation implementations and related items.

use crate::{asm::Word, MemoryError, MemoryResult, OpSyncResult, Vm};

/// A type representing the VM's memory.
///
/// `Memory` is a thin wrapper around a `Vec<Option<Word>>`. The `Vec` mutable methods
/// are intentionally not exposed in order to maintain close control over capacity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Memory(Vec<Option<Word>>);

impl Memory {
    /// The maximum number of words stored in memory.
    ///
    /// This results in `4096` * the size of `Option<Word>`.
    pub const SIZE_LIMIT: usize = 4096;

    /// Allocate new capacity to the end of the memory.
    pub fn alloc(&mut self, size: usize) -> MemoryResult<()> {
        if self.capacity() + size > Self::SIZE_LIMIT {
            return Err(MemoryError::Overflow);
        }
        self.0.reserve_exact(size);
        Ok(())
    }

    /// Set the value at the given index to `None`.
    pub fn clear(&mut self, index: usize) -> MemoryResult<()> {
        *self.0.get_mut(index).ok_or(MemoryError::IndexOutOfBounds)? = None;
        Ok(())
    }

    /// Set the values over the given range to `None`.
    pub fn clear_range(&mut self, range: core::ops::Range<usize>) -> MemoryResult<()> {
        self.0
            .get_mut(range)
            .ok_or(MemoryError::IndexOutOfBounds)?
            .iter_mut()
            .for_each(|val| *val = None);
        Ok(())
    }

    /// Free the specified amount of memory from the end.
    pub fn free(&mut self, size: usize) {
        let new_size = self.capacity().saturating_sub(size);
        self.0.shrink_to(new_size);
    }

    /// Check whether the value at the given index is `Some`.
    pub fn is_some(&self, index: usize) -> MemoryResult<bool> {
        let opt = self.get(index).ok_or(MemoryError::IndexOutOfBounds)?;
        Ok(opt.is_some())
    }

    /// Load a word at the given index.
    pub fn load(&self, index: usize) -> MemoryResult<Word> {
        let opt = self.get(index).ok_or(MemoryError::IndexOutOfBounds)?;
        Ok(opt.unwrap_or(Word::default()))
    }

    /// Push a word to the stack.
    pub fn push(&mut self, opt: Option<Word>) -> MemoryResult<()> {
        if self.len() >= self.capacity() {
            return Err(MemoryError::Overflow);
        }
        self.0.push(opt);
        Ok(())
    }

    /// Extend memory with the given values.
    pub fn extend(&mut self, words: Vec<Option<Word>>) -> MemoryResult<()> {
        let new_len = self
            .len()
            .checked_add(words.len())
            .ok_or(MemoryError::Overflow)?;
        if new_len > self.capacity() {
            return Err(MemoryError::Overflow);
        }
        self.0.extend(words);
        Ok(())
    }

    /// Store the given `word` at the given `index`.
    pub fn store(&mut self, index: usize, value: Word) -> MemoryResult<()> {
        let opt = self.0.get_mut(index).ok_or(MemoryError::IndexOutOfBounds)?;
        *opt = Some(value);
        Ok(())
    }

    /// Truncate `Memory` to the given new length.
    pub fn truncate(&mut self, new_len: usize) {
        self.0.truncate(new_len);
    }
}

impl core::ops::Deref for Memory {
    type Target = Vec<Option<Word>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Memory> for Vec<Option<Word>> {
    fn from(memory: Memory) -> Self {
        memory.0
    }
}

/// `Memory::Alloc` operation.
pub fn alloc(vm: &mut Vm) -> OpSyncResult<()> {
    let size = vm.stack.pop()?;
    let size = usize::try_from(size).map_err(|_| MemoryError::IndexOutOfBounds)?;
    vm.memory.alloc(size)?;
    Ok(())
}

/// `Memory::Capacity` operation.
pub fn capacity(vm: &mut Vm) -> OpSyncResult<()> {
    let cap = Word::try_from(vm.memory.capacity()).map_err(|_| MemoryError::IndexOutOfBounds)?;
    vm.stack.push(cap)?;
    Ok(())
}

/// `Memory::Clear` operation.
pub fn clear(vm: &mut Vm) -> OpSyncResult<()> {
    let index = vm.stack.pop()?;
    let index = usize::try_from(index).map_err(|_| MemoryError::IndexOutOfBounds)?;
    vm.memory.clear(index)?;
    Ok(())
}

/// `Memory::Clear` operation.
pub fn clear_range(vm: &mut Vm) -> OpSyncResult<()> {
    let [index, len] = vm.stack.pop2()?;
    let range = range_from_start_len(index, len).ok_or(MemoryError::IndexOutOfBounds)?;
    vm.memory.clear_range(range)?;
    Ok(())
}

/// `Memory::Free` operation.
pub fn free(vm: &mut Vm) -> OpSyncResult<()> {
    let size = vm.stack.pop()?;
    let size = usize::try_from(size).map_err(|_| MemoryError::IndexOutOfBounds)?;
    vm.memory.free(size);
    Ok(())
}

// `Memory::IsSome` operation.
pub fn is_some(vm: &mut Vm) -> OpSyncResult<()> {
    let index = vm.stack.pop()?;
    let index = usize::try_from(index).map_err(|_| MemoryError::IndexOutOfBounds)?;
    let is_some = vm.memory.is_some(index)?;
    vm.stack.push(is_some.into())?;
    Ok(())
}

/// `Memory::Capacity` operation.
pub fn length(vm: &mut Vm) -> OpSyncResult<()> {
    let cap = Word::try_from(vm.memory.len()).map_err(|_| MemoryError::IndexOutOfBounds)?;
    vm.stack.push(cap)?;
    Ok(())
}

/// `Memory::Load` operation.
pub fn load(vm: &mut Vm) -> OpSyncResult<()> {
    let index = vm.stack.pop()?;
    let index = usize::try_from(index).map_err(|_| MemoryError::IndexOutOfBounds)?;
    let word = vm.memory.load(index)?;
    vm.stack.push(word)?;
    Ok(())
}

/// `Memory::Push` operation.
pub fn push(vm: &mut Vm) -> OpSyncResult<()> {
    let word = vm.stack.pop()?;
    vm.memory.push(Some(word))?;
    Ok(())
}

/// `Memory::PushNone` operation.
pub fn push_none(vm: &mut Vm) -> OpSyncResult<()> {
    vm.memory.push(None)?;
    Ok(())
}

/// `Memory::Store` operation.
pub fn store(vm: &mut Vm) -> OpSyncResult<()> {
    let [index, value] = vm.stack.pop2()?;
    let index = usize::try_from(index).map_err(|_| MemoryError::IndexOutOfBounds)?;
    vm.memory.store(index, value)?;
    Ok(())
}

/// `Memory::Truncate` operation.
pub fn truncate(vm: &mut Vm) -> OpSyncResult<()> {
    let len = vm.stack.pop()?;
    let len = usize::try_from(len).map_err(|_| MemoryError::IndexOutOfBounds)?;
    vm.memory.truncate(len);
    Ok(())
}

fn range_from_start_len(start: Word, len: Word) -> Option<std::ops::Range<usize>> {
    let start = usize::try_from(start).ok()?;
    let len = usize::try_from(len).ok()?;
    let end = start.checked_add(len)?;
    Some(start..end)
}

#[cfg(test)]
mod tests {
    use crate::{
        error::{MemoryError, OpSyncError},
        test_util::*,
        *,
    };

    #[tokio::test]
    async fn alloc() {
        let mut vm = Vm::default();
        let cap = 5;
        assert_eq!(vm.memory.capacity(), 0);
        let ops = &[asm::Stack::Push(cap).into(), asm::Memory::Alloc.into()];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        assert_eq!(vm.memory.capacity(), cap as usize);
    }

    #[tokio::test]
    async fn capacity() {
        let mut vm = Vm::default();
        let cap = 3;
        assert_eq!(vm.memory.capacity(), 0);
        let ops = &[
            asm::Stack::Push(cap).into(),
            asm::Memory::Alloc.into(),
            asm::Memory::Capacity.into(),
        ];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        assert_eq!(vm.memory.capacity(), cap as usize);
        assert_eq!(&vm.stack[..], &[cap]);
    }

    #[tokio::test]
    async fn clear() {
        let mut vm = Vm::default();
        // First, push a value.
        let ops = &[
            asm::Stack::Push(1).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Push.into(),
        ];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        assert_eq!(vm.memory.capacity(), 1);
        assert_eq!(&vm.memory[..], &[Some(42)]);
        // Next, clear the value.
        let ops = &[asm::Stack::Push(0).into(), asm::Memory::Clear.into()];
        vm.pc = 0;
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        // Capacity remains the same. But the value is `None`.
        assert_eq!(vm.memory.capacity(), 1);
        assert_eq!(&vm.memory[..], &[None]);
    }

    #[tokio::test]
    async fn clear_range() {
        let mut vm = Vm::default();
        let ops = &[
            asm::Stack::Push(4).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Push.into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Push.into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Push.into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Push.into(),
        ];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        assert_eq!(vm.memory.capacity(), 4);
        assert_eq!(&vm.memory[..], &[Some(42), Some(42), Some(42), Some(42)]);
        // Next, clear the values at indices 1 and 2.
        let ops = &[
            asm::Stack::Push(1).into(),
            asm::Stack::Push(2).into(),
            asm::Memory::ClearRange.into(),
        ];
        vm.pc = 0;
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        // Capacity remains the same, but middle values should be None.
        assert_eq!(vm.memory.capacity(), 4);
        assert_eq!(&vm.memory[..], &[Some(42), None, None, Some(42)]);
        assert!(vm.stack.is_empty());
    }

    #[tokio::test]
    async fn free() {
        let mut vm = Vm::default();
        let size = 3;
        assert_eq!(vm.memory.capacity(), 0);
        let ops = &[
            asm::Stack::Push(size).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(size).into(),
            asm::Memory::Free.into(),
        ];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        assert_eq!(vm.memory.capacity(), 0);
    }

    #[tokio::test]
    async fn is_some() {
        let mut vm = Vm::default();
        let ops = &[
            asm::Stack::Push(1).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Push.into(),
            asm::Stack::Push(0).into(), // Check if the value at index 0 is `Some`
            asm::Memory::IsSome.into(),
        ];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        assert_eq!(&vm.stack[..], &[1 /*true*/],);
    }

    #[tokio::test]
    async fn length() {
        let mut vm = Vm::default();
        let ops = &[
            asm::Stack::Push(6).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Push.into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Push.into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Push.into(),
            asm::Memory::Length.into(),
        ];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        // Make sure that capacity and length are tracked separately correctly.
        assert_eq!(vm.memory.capacity(), 6);
        assert_eq!(vm.memory.len(), 3);
        assert_eq!(&vm.stack[..], &[3]);
    }

    #[tokio::test]
    async fn load() {
        let mut vm = Vm::default();
        let ops = &[
            asm::Stack::Push(1).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Push.into(),
            asm::Stack::Push(0).into(), // Load the value at index 0
            asm::Memory::Load.into(),
        ];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        assert_eq!(&vm.memory[..], &[Some(42)]);
        assert_eq!(&vm.stack[..], &[42]);
    }

    #[tokio::test]
    async fn push() {
        let mut vm = Vm::default();
        let ops = &[
            asm::Stack::Push(1).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Push.into(),
        ];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        assert_eq!(vm.memory.capacity(), 1);
        assert_eq!(&vm.memory[..], &[Some(42)]);
        assert!(vm.stack.is_empty());
    }

    #[tokio::test]
    async fn push_none() {
        let mut vm = Vm::default();
        let ops = &[
            asm::Stack::Push(2).into(),
            asm::Memory::Alloc.into(),
            asm::Memory::PushNone.into(),
            asm::Memory::PushNone.into(),
        ];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        assert_eq!(vm.memory.capacity(), 2);
        assert_eq!(&vm.memory[..], &[None, None]);
        assert!(vm.stack.is_empty());
    }

    #[tokio::test]
    async fn store() {
        let mut vm = Vm::default();
        let ops = &[
            // Allocate two slots.
            asm::Stack::Push(2).into(),
            asm::Memory::Alloc.into(),
            // Push two `None`s onto the allocated memory.
            asm::Memory::PushNone.into(),
            asm::Memory::PushNone.into(),
            // Store `Some(42)` in the second slot.
            asm::Stack::Push(1).into(),
            asm::Stack::Push(42).into(),
            asm::Memory::Store.into(),
        ];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        assert_eq!(&vm.memory[..], &[None, Some(42)]);
    }

    #[tokio::test]
    async fn truncate() {
        let mut vm = Vm::default();
        let ops = &[
            // Push 3 `None`s.
            asm::Stack::Push(3).into(),
            asm::Memory::Alloc.into(),
            asm::Memory::PushNone.into(),
            asm::Memory::PushNone.into(),
            asm::Memory::PushNone.into(),
            // Truncate down to one `None`. Doesn't affect capacity.
            asm::Stack::Push(1).into(),
            asm::Memory::Truncate.into(),
        ];
        vm.exec_ops(
            ops,
            TEST_ACCESS,
            &State::EMPTY,
            &|_: &Op| 1,
            GasLimit::UNLIMITED,
        )
        .await
        .unwrap();
        assert_eq!(&vm.memory[..], &[None]);
        assert_eq!(vm.memory.capacity(), 3);
    }

    #[tokio::test]
    async fn load_index_oob() {
        let mut vm = Vm::default();
        let ops = &[asm::Stack::Push(0).into(), asm::Memory::Load.into()];
        let res = vm
            .exec_ops(
                ops,
                TEST_ACCESS,
                &State::EMPTY,
                &|_: &Op| 1,
                GasLimit::UNLIMITED,
            )
            .await;
        match res {
            Err(StateReadError::Op(
                _,
                OpError::Sync(OpSyncError::Memory(MemoryError::IndexOutOfBounds)),
            )) => (),
            _ => panic!("expected index out of bounds, found {:?}", res),
        }
    }

    #[tokio::test]
    async fn store_index_oob() {
        let mut vm = Vm::default();
        let ops = &[
            asm::Stack::Push(0).into(),
            asm::Stack::Push(0).into(),
            asm::Memory::Store.into(),
        ];
        let res = vm
            .exec_ops(
                ops,
                TEST_ACCESS,
                &State::EMPTY,
                &|_: &Op| 1,
                GasLimit::UNLIMITED,
            )
            .await;
        match res {
            Err(StateReadError::Op(
                _,
                OpError::Sync(OpSyncError::Memory(MemoryError::IndexOutOfBounds)),
            )) => (),
            _ => panic!("expected index out of bounds, found {:?}", res),
        }
    }

    #[tokio::test]
    async fn push_overflow() {
        let mut vm = Vm::default();
        let ops = &[asm::Stack::Push(42).into(), asm::Memory::Push.into()];
        let res = vm
            .exec_ops(
                ops,
                TEST_ACCESS,
                &State::EMPTY,
                &|_: &Op| 1,
                GasLimit::UNLIMITED,
            )
            .await;
        match res {
            Err(StateReadError::Op(
                _,
                OpError::Sync(OpSyncError::Memory(MemoryError::Overflow)),
            )) => (),
            _ => panic!("expected overflow, found {:?}", res),
        }
    }

    #[tokio::test]
    async fn alloc_overflow() {
        let mut vm = Vm::default();
        let overflow_cap = Word::try_from(Memory::SIZE_LIMIT.checked_add(1).unwrap()).unwrap();
        let ops = &[
            asm::Stack::Push(overflow_cap).into(),
            asm::Memory::Alloc.into(),
        ];
        let res = vm
            .exec_ops(
                ops,
                TEST_ACCESS,
                &State::EMPTY,
                &|_: &Op| 1,
                GasLimit::UNLIMITED,
            )
            .await;
        match res {
            Err(StateReadError::Op(
                _,
                OpError::Sync(OpSyncError::Memory(MemoryError::Overflow)),
            )) => (),
            _ => panic!("expected overflow, found {:?}", res),
        }
    }
}
