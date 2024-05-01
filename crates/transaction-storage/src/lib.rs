#![deny(missing_docs)]
//! # Transaction Storage
//!
//! Provides a transactional layer on top of a state storage.

use essential_state_read_vm::StateRead;
use essential_types::{ContentAddress, Key, Word};
use futures::future::FutureExt;
use imbl::HashMap;
use std::{pin::Pin, sync::Arc};
use storage::{word_range, QueryState, StateStorage};
use thiserror::Error;

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
pub struct TransactionStorage<S> {
    state: HashMap<ContentAddress, HashMap<Key, Mutation>>,
    storage: S,
}

/// View of a transaction.
#[derive(Clone)]
pub struct TransactionView<S>(Arc<TransactionStorage<S>>);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Mutation {
    Insert(Word),
    Delete,
}

/// Error for transaction view.
#[derive(Debug, Error)]
pub enum TransactionViewError {
    /// Error during read
    #[error("failed to read")]
    ReadError(#[from] anyhow::Error),
}

impl<S> StateRead for TransactionView<S>
where
    S: StateStorage + Clone + Send + Sync + 'static,
{
    type Error = TransactionViewError;

    type Future =
        Pin<Box<dyn std::future::Future<Output = Result<Vec<Option<Word>>, Self::Error>> + Send>>;

    fn word_range(&self, set_addr: ContentAddress, key: Key, num_words: usize) -> Self::Future {
        let storage = self.clone();
        async move { word_range(&storage, set_addr, key, num_words).await }.boxed()
    }
}

impl<S> TransactionStorage<S> {
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

    // TODO: or we can make the fields `pub`
    /// Get a clone of this transaction's storage.
    pub fn storage(&self) -> S
    where
        S: Clone,
    {
        self.storage.clone()
    }

    /// Commit the transaction.
    pub async fn commit(&mut self) -> anyhow::Result<()>
    where
        S: StateStorage,
    {
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
    ) -> anyhow::Result<Option<Word>>
    where
        S: QueryState,
    {
        let m = self.state.entry(address.clone()).or_default();
        let entry = m.entry(*key);
        let mutation = match entry {
            imbl::hashmap::Entry::Occupied(mut v) => match value {
                Some(value) => Some(v.insert(Mutation::Insert(value))),
                None => Some(v.insert(Mutation::Delete)),
            },
            imbl::hashmap::Entry::Vacant(v) => {
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

    /// Apply state changes without returning the previous value.
    pub fn apply_state(&mut self, address: &ContentAddress, key: &Key, value: Option<Word>) {
        let m = self.state.entry(address.clone()).or_default();
        let entry = m.entry(*key);
        match entry {
            imbl::hashmap::Entry::Occupied(mut v) => match value {
                Some(value) => {
                    v.insert(Mutation::Insert(value));
                }
                None => {
                    v.insert(Mutation::Delete);
                }
            },
            imbl::hashmap::Entry::Vacant(v) => match value {
                Some(value) => {
                    v.insert(Mutation::Insert(value));
                }
                None => {
                    v.insert(Mutation::Delete);
                }
            },
        }
    }

    /// Query the state of this transaction.
    pub async fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> anyhow::Result<Option<Word>>
    where
        S: QueryState,
    {
        let mutation = self.state.get(address).and_then(|m| m.get(key)).copied();
        match mutation {
            Some(Mutation::Insert(v)) => Ok(Some(v)),
            Some(Mutation::Delete) => Ok(None),
            None => self.storage.query_state(address, key).await,
        }
    }
}

impl<S> QueryState for TransactionView<S>
where
    S: QueryState + Clone + Send + Sync + 'static,
{
    async fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> anyhow::Result<Option<Word>> {
        self.0.query_state(address, key).await
    }
}
