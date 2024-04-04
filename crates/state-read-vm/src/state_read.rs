//! State read operation implementations.

use crate::{error::StackError, OpResult, Vm};
use essential_types::{convert::u8_32_from_word_4, ContentAddress, Key, Word};

/// Access to state required by the state read VM.
// NOTE: Keep an eye on tokio issues related to auto-traits on the returned futures.
#[allow(async_fn_in_trait)]
pub trait StateRead {
    /// An error type describing any cases that might occur during state reading.
    type Error: core::fmt::Debug + std::error::Error;

    /// Read the given number of words from state at the given key.
    async fn word_range(&self, key: Key, num_words: usize) -> Result<Vec<Word>, Self::Error>;

    /// Read the given number of words from state at the given external key.
    async fn word_range_ext(
        &self,
        set_addr: ContentAddress,
        key: Key,
        num_words: usize,
    ) -> Result<Vec<Word>, Self::Error>;
}

/// `StateRead::WordRange` operation.
pub async fn word_range<S>(state_read: &S, vm: &mut Vm) -> OpResult<(), S::Error>
where
    S: StateRead,
{
    let len_word = vm.stack.pop()?;
    let key_words = vm.stack.pop4()?;
    let len = usize::try_from(len_word).map_err(|_| StackError::IndexOutOfBounds)?;
    let key = u8_32_from_word_4(key_words);
    let words = state_read.word_range(key, len).await?;
    todo!()
}

/// `StateRead::WordRangeExtern` operation.
pub async fn word_range_ext<S>(state_read: &S, vm: &mut Vm) -> OpResult<(), S::Error>
where
    S: StateRead,
{
    let len_word = vm.stack.pop()?;
    let key_words = vm.stack.pop4()?;
    let set_addr_words = vm.stack.pop4()?;
    todo!()
}
