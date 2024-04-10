//! State read operation implementations.

use crate::{
    error::{OpAsyncError, StackError},
    OpAsyncResult, Vm,
};
use essential_types::{convert::u8_32_from_word_4, ContentAddress, Key, Word};

/// Access to state required by the state read VM.
// NOTE: Keep an eye on tokio issues related to auto-traits on the returned future.
#[allow(async_fn_in_trait)]
pub trait StateRead {
    /// An error type describing any cases that might occur during state reading.
    type Error: std::error::Error;

    /// Read the given number of words from state at the given key associated
    /// with the given intent set address.
    async fn word_range(
        &self,
        set_addr: ContentAddress,
        key: Key,
        num_words: usize,
    ) -> Result<Vec<Option<Word>>, Self::Error>;
}

/// `StateRead::WordRange` operation.
pub async fn word_range<S>(
    state_read: &S,
    set_addr: &ContentAddress,
    vm: &mut Vm,
) -> OpAsyncResult<(), S::Error>
where
    S: StateRead,
{
    let words = read_word_range(state_read, set_addr, vm).await?;
    write_words_to_memory(vm, words)
}

/// `StateRead::WordRangeExtern` operation.
pub async fn word_range_ext<S>(state_read: &S, vm: &mut Vm) -> OpAsyncResult<(), S::Error>
where
    S: StateRead,
{
    let words = read_word_range_ext(state_read, vm).await?;
    write_words_to_memory(vm, words)
}

/// Read the length and key from the top of the stack and read the associated words from state.
async fn read_word_range<S>(
    state_read: &S,
    set_addr: &ContentAddress,
    vm: &mut Vm,
) -> OpAsyncResult<Vec<Option<Word>>, S::Error>
where
    S: StateRead,
{
    let len_word = vm.stack.pop()?;
    let len = usize::try_from(len_word).map_err(|_| StackError::IndexOutOfBounds)?;
    let key = vm.stack.pop4()?;
    state_read
        .word_range(set_addr.clone(), key, len)
        .await
        .map_err(OpAsyncError::StateRead)
}

/// Read the length, key and external set address from the top of the stack and
/// read the associated words from state.
async fn read_word_range_ext<S>(
    state_read: &S,
    vm: &mut Vm,
) -> OpAsyncResult<Vec<Option<Word>>, S::Error>
where
    S: StateRead,
{
    let len_word = vm.stack.pop()?;
    let len = usize::try_from(len_word).map_err(|_| StackError::IndexOutOfBounds)?;
    let key = vm.stack.pop4().map_err(OpAsyncError::from)?;
    let set_addr = ContentAddress(u8_32_from_word_4(vm.stack.pop4()?));
    state_read
        .word_range(set_addr, key, len)
        .await
        .map_err(OpAsyncError::StateRead)
}

/// Write the given words to the end of memory and push the starting memory address to the stack.
fn write_words_to_memory<E>(vm: &mut Vm, words: Vec<Option<Word>>) -> OpAsyncResult<(), E> {
    let start = Word::try_from(vm.memory.len()).map_err(|_| StackError::IndexOutOfBounds)?;
    vm.memory.extend(words)?;
    vm.stack.push(start)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use types::convert::word_4_from_u8_32;

    use crate::{test_util::*, *};

    #[tokio::test]
    async fn state_read_3_42s() {
        let access = TEST_ACCESS;
        let state = State::new(vec![(
            access.solution.this_data().intent_to_solve.set.clone(),
            vec![([0, 0, 0, 0], 42), ([0, 0, 0, 1], 42), ([0, 0, 0, 2], 42)],
        )]);
        let mut vm = Vm::default();
        let num_words = 3;
        let ops = &[
            asm::Stack::Push(num_words).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(0).into(), // Key0
            asm::Stack::Push(0).into(), // Key1
            asm::Stack::Push(0).into(), // Key2
            asm::Stack::Push(0).into(), // Key3
            asm::Stack::Push(num_words).into(),
            asm::StateRead::WordRange,
        ];
        vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
            .await
            .unwrap();
        assert_eq!(&vm.memory[..], &[Some(42), Some(42), Some(42)]);
        assert_eq!(vm.memory.capacity(), 3);
    }

    #[tokio::test]
    async fn state_read_some_none_some() {
        let access = TEST_ACCESS;
        let state = State::new(vec![(
            access.solution.this_data().intent_to_solve.set.clone(),
            vec![([0, 0, 0, 0], 42), ([0, 0, 0, 2], 42)],
        )]);
        let mut vm = Vm::default();
        let num_words = 3;
        let ops = &[
            asm::Stack::Push(num_words).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(0).into(), // Key0
            asm::Stack::Push(0).into(), // Key1
            asm::Stack::Push(0).into(), // Key2
            asm::Stack::Push(0).into(), // Key3
            asm::Stack::Push(num_words).into(),
            asm::StateRead::WordRange,
        ];
        vm.exec_ops(ops, access, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
            .await
            .unwrap();
        assert_eq!(&vm.memory[..], &[Some(42), None, Some(42)]);
        assert_eq!(vm.memory.capacity(), 3);
    }

    #[tokio::test]
    async fn state_read_ext() {
        let ext_set_addr = ContentAddress([0x12; 32]);
        let state = State::new(vec![(
            ext_set_addr.clone(),
            vec![([1, 2, 3, 4], 40), ([1, 2, 3, 5], 41), ([1, 2, 3, 6], 42)],
        )]);
        let mut vm = Vm::default();
        let num_words = 3;
        let [addr0, addr1, addr2, addr3] = word_4_from_u8_32(ext_set_addr.0);
        let ops = &[
            asm::Stack::Push(num_words).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(addr0).into(),
            asm::Stack::Push(addr1).into(),
            asm::Stack::Push(addr2).into(),
            asm::Stack::Push(addr3).into(),
            asm::Stack::Push(1).into(), // Key0
            asm::Stack::Push(2).into(), // Key1
            asm::Stack::Push(3).into(), // Key2
            asm::Stack::Push(4).into(), // Key3
            asm::Stack::Push(num_words).into(),
            asm::StateRead::WordRangeExtern,
        ];
        vm.exec_ops(ops, TEST_ACCESS, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
            .await
            .unwrap();
        assert_eq!(&vm.memory[..], &[Some(40), Some(41), Some(42)]);
        assert_eq!(vm.memory.capacity(), 3);
    }

    #[tokio::test]
    async fn state_read_ext_nones() {
        let ext_set_addr = ContentAddress([0x12; 32]);
        let state = State::new(vec![(ext_set_addr.clone(), vec![])]);
        let mut vm = Vm::default();
        let num_words = 3;
        let [addr0, addr1, addr2, addr3] = word_4_from_u8_32(ext_set_addr.0);
        let ops = &[
            asm::Stack::Push(num_words).into(),
            asm::Memory::Alloc.into(),
            asm::Stack::Push(addr0).into(),
            asm::Stack::Push(addr1).into(),
            asm::Stack::Push(addr2).into(),
            asm::Stack::Push(addr3).into(),
            asm::Stack::Push(1).into(), // Key0
            asm::Stack::Push(2).into(), // Key1
            asm::Stack::Push(3).into(), // Key2
            asm::Stack::Push(4).into(), // Key3
            asm::Stack::Push(num_words).into(),
            asm::StateRead::WordRangeExtern,
        ];
        vm.exec_ops(ops, TEST_ACCESS, &state, &|_: &Op| 1, GasLimit::UNLIMITED)
            .await
            .unwrap();
        assert_eq!(&vm.memory[..], &[None, None, None]);
        assert_eq!(vm.memory.capacity(), 3);
    }
}
