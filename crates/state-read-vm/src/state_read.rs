//! State read operation implementations.

use crate::{
    error::{OpError, StackError},
    OpResult, Vm,
};
use essential_types::{convert::u8_32_from_word_4, ContentAddress, Key, Word};

/// Access to state required by the state read VM.
// NOTE: Keep an eye on tokio issues related to auto-traits on the returned futures.
#[allow(async_fn_in_trait)]
pub trait StateRead {
    /// An error type describing any cases that might occur during state reading.
    type Error: std::error::Error;

    /// Read the given number of words from state at the given key.
    async fn word_range(
        &self,
        key: Key,
        num_words: usize,
    ) -> Result<Vec<Option<Word>>, Self::Error>;

    /// Read the given number of words from state at the given external key.
    async fn word_range_ext(
        &self,
        set_addr: ContentAddress,
        key: Key,
        num_words: usize,
    ) -> Result<Vec<Option<Word>>, Self::Error>;
}

/// `StateRead::WordRange` operation.
pub async fn word_range<S>(state_read: &S, vm: &mut Vm) -> OpResult<(), S::Error>
where
    S: StateRead,
{
    let len_word = vm.stack.pop()?;
    let len = usize::try_from(len_word).map_err(|_| StackError::IndexOutOfBounds)?;
    let key = vm.stack.pop4()?;
    let words = state_read
        .word_range(key, len)
        .await
        .map_err(OpError::StateRead)?;
    write_words_to_memory(vm, words)
}

/// `StateRead::WordRangeExtern` operation.
pub async fn word_range_ext<S>(state_read: &S, vm: &mut Vm) -> OpResult<(), S::Error>
where
    S: StateRead,
{
    let len_word = vm.stack.pop()?;
    let len = usize::try_from(len_word).map_err(|_| StackError::IndexOutOfBounds)?;
    let key = vm.stack.pop4()?;
    let set_addr = ContentAddress(u8_32_from_word_4(vm.stack.pop4()?));
    let words = state_read
        .word_range_ext(set_addr, key, len)
        .await
        .map_err(OpError::StateRead)?;
    write_words_to_memory(vm, words)
}

/// Write the given words to the end of memory and push the starting memory address to the stack.
fn write_words_to_memory<E>(vm: &mut Vm, words: Vec<Option<Word>>) -> OpResult<(), E> {
    let start = Word::try_from(vm.memory.len()).map_err(|_| StackError::IndexOutOfBounds)?;
    vm.memory.extend(words)?;
    vm.stack.push(start)?;
    Ok(())
}
