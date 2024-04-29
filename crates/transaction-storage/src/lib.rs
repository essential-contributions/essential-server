#![deny(missing_docs)]
//! # Transaction Storage
//!
//! Provides a transactional layer on top of a state storage.

use essential_state_read_vm::StateRead;
use essential_types::{ContentAddress, Key, Word};
use futures::future::FutureExt;
use std::{collections::HashMap, pin::Pin, sync::Arc};
use storage::StateStorage;
use thiserror::Error;
use utils::next_key;

#[cfg(test)]
mod tests;

/// Utility trait to provide transactional semantics on top of a state storage.
pub trait Transaction {
    /// Start a new transaction.
    fn transaction(self) -> TransactionStorage<Self>
    where
        Self: StateStorage + Sized;
}

impl<S> Transaction for S
where
    S: StateStorage,
{
    fn transaction(self) -> TransactionStorage<Self> {
        TransactionStorage::new(self)
    }
}

/// Wrapper around a state storage that provides transactional semantics.
pub struct TransactionStorage<S>
where
    S: StateStorage,
{
    state: HashMap<ContentAddress, HashMap<Key, Mutation>>,
    storage: S,
}

/// View of a transaction.
#[derive(Clone)]
pub struct TransactionView<S>(Arc<TransactionStorage<S>>)
where
    S: StateStorage;

/// Error for transaction view.
#[derive(Debug, Error)]
pub enum TransactionViewError {
    /// Error during read
    #[error("failed to read")]
    ReadError,
}

impl<S> StateRead for TransactionView<S>
where
    S: StateRead + StateStorage + Clone + Send + Sync + 'static,
{
    type Error = TransactionViewError;

    type Future =
        Pin<Box<dyn std::future::Future<Output = Result<Vec<Option<Word>>, Self::Error>> + Send>>;

    fn word_range(&self, set_addr: ContentAddress, key: Key, num_words: usize) -> Self::Future {
        let storage = self.clone();
        async move { transaction_view_word_range(storage, set_addr, key, num_words).await }.boxed()
    }
}

async fn transaction_view_word_range<S>(
    storage: TransactionView<S>,
    set_addr: ContentAddress,
    mut key: Key,
    num_words: usize,
) -> Result<Vec<Option<Word>>, TransactionViewError>
where
    S: StateStorage + Send,
{
    let mut words = vec![];
    for _ in 0..num_words {
        let opt = storage
            .0
            .query_state(&set_addr, &key)
            .await
            .map_err(|_| TransactionViewError::ReadError)?;
        words.push(opt);
        key = next_key(key).ok_or(TransactionViewError::ReadError)?
    }
    Ok(words)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Mutation {
    Insert(Word),
    Delete,
}

impl<S> TransactionStorage<S>
where
    S: StateStorage,
{
    /// Create a new transaction storage around the given state storage.
    pub fn new(storage: S) -> Self {
        Self {
            state: Default::default(),
            storage,
        }
    }

    /// Create a view of this transaction.
    pub fn view(&self) -> TransactionView<S>
    where
        S: Clone,
    {
        TransactionView(Arc::new(Self {
            state: self.state.clone(),
            storage: self.storage.clone(),
        }))
    }

    /// Commit the transaction.
    pub async fn commit(&mut self) -> anyhow::Result<()> {
        let updates = self.state.iter().flat_map(|(address, m)| {
            m.iter().map(move |(key, mutation)| {
                (
                    address.clone(),
                    *key,
                    match mutation {
                        Mutation::Insert(v) => Some(*v),
                        Mutation::Delete => None,
                    },
                )
            })
        });
        self.storage.update_state_batch(updates).await?;
        self.state.clear();
        Ok(())
    }

    /// Rollback the transaction.
    pub fn rollback(&mut self) {
        self.state.clear()
    }

    /// Update the state of this transaction.
    pub async fn update_state(
        &mut self,
        address: &ContentAddress,
        key: &Key,
        value: Option<Word>,
    ) -> anyhow::Result<Option<Word>> {
        let m = self.state.entry(address.clone()).or_default();
        let entry = m.entry(*key);
        let mutation = match entry {
            std::collections::hash_map::Entry::Occupied(mut v) => match value {
                Some(value) => Some(v.insert(Mutation::Insert(value))),
                None => Some(v.insert(Mutation::Delete)),
            },
            std::collections::hash_map::Entry::Vacant(v) => {
                match value {
                    Some(value) => {
                        v.insert(Mutation::Insert(value));
                    }
                    None => {
                        v.insert(Mutation::Delete);
                    }
                }
                None
            }
        };

        match mutation {
            Some(Mutation::Insert(v)) => Ok(Some(v)),
            Some(Mutation::Delete) => Ok(None),
            None => self.storage.query_state(address, key).await,
        }
    }

    /// Query the state of this transaction.
    pub async fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> anyhow::Result<Option<Word>> {
        let mutation = self.state.get(address).and_then(|m| m.get(key)).copied();
        match mutation {
            Some(Mutation::Insert(v)) => Ok(Some(v)),
            Some(Mutation::Delete) => Ok(None),
            None => self.storage.query_state(address, key).await,
        }
    }
}
