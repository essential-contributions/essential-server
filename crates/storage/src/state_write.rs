use core::future::Future;
use essential_types::{ContentAddress, Key, Word};

/// Update to state required for checking solutions.
pub trait StateWrite {
    /// An error type describing any cases that might occur during updating state.
    type Error: std::error::Error;
    /// The future type returned from the `update_state_batch` method.
    type Future: Future<Output = Result<Vec<Option<Word>>, Self::Error>> + Unpin;

    /// Per update in batch, write the given word to state at the given key
    /// associated with the given intent set address.
    fn update_state_batch<U>(&self, updates: U) -> Self::Future
    where
        U: IntoIterator<Item = (ContentAddress, Key, Option<Word>)> + Send + 'static;
}
