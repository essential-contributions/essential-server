#![deny(missing_docs)]
//! # Transaction Storage
//!
//! Provides a transactional layer on top of a state storage.

use std::collections::HashMap;

use essential_types::{ContentAddress, Key, Word};
use storage::StateStorage;

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
