#![deny(missing_docs)]
//! # Transaction Storage
//!
//! Provides a transactional layer on top of a state storage.

use std::{collections::HashMap, sync::Arc};

use essential_types::{ContentAddress, Key, Word};
use storage::StateStorage;
use utils::Lock;

#[cfg(test)]
mod tests;

#[derive(Clone)]
/// Wrapper around a state storage that provides transactional semantics.
pub struct TransactionStorage<S>
where
    S: StateStorage,
{
    inner: Arc<Lock<Inner>>,
    storage: S,
}

struct Inner {
    state: HashMap<ContentAddress, HashMap<Key, Mutation>>,
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
            inner: Arc::new(Lock::new(Inner {
                state: Default::default(),
            })),
            storage,
        }
    }

    /// Commit the transaction.
    pub async fn commit(&self) -> anyhow::Result<()> {
        let state = self.inner.apply(|i| i.state.clone());
        let updates = state.into_iter().flat_map(|(address, m)| {
            m.into_iter().map(move |(key, mutation)| {
                (
                    address.clone(),
                    key,
                    match mutation {
                        Mutation::Insert(v) => Some(v),
                        Mutation::Delete => None,
                    },
                )
            })
        });
        self.storage.update_state_batch(updates).await?;
        self.inner.apply(|i| i.state.clear());
        Ok(())
    }

    /// Rollback the transaction.
    pub fn rollback(&self) {
        self.inner.apply(|i| i.state.clear());
    }
}

impl<S> StateStorage for TransactionStorage<S>
where
    S: StateStorage,
{
    async fn update_state(
        &self,
        address: &ContentAddress,
        key: &Key,
        value: Option<Word>,
    ) -> anyhow::Result<Option<Word>> {
        let mutation = self.inner.apply(|i| {
            let m = i.state.entry(address.clone()).or_default();
            let entry = m.entry(*key);
            match entry {
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
            }
        });

        match mutation {
            Some(Mutation::Insert(v)) => Ok(Some(v)),
            Some(Mutation::Delete) => Ok(None),
            None => self.storage.query_state(address, key).await,
        }
    }

    async fn update_state_batch<U>(&self, updates: U) -> anyhow::Result<Vec<Option<Word>>>
    where
        U: IntoIterator<Item = (ContentAddress, Key, Option<Word>)>,
    {
        let mutations = updates.into_iter().map(|(address, key, value)| {
            let mutation = match value {
                Some(value) => Mutation::Insert(value),
                None => Mutation::Delete,
            };
            (address, key, mutation)
        });

        let results: Vec<_> = self.inner.apply(|i| {
            mutations
                .map(|(address, key, mutation)| {
                    let m = i.state.entry(address.clone()).or_default();
                    let entry = m.entry(key);
                    let result = match entry {
                        std::collections::hash_map::Entry::Occupied(mut v) => match mutation {
                            Mutation::Insert(word) => Some(v.insert(Mutation::Insert(word))),
                            Mutation::Delete => Some(v.insert(Mutation::Delete)),
                        },
                        std::collections::hash_map::Entry::Vacant(v) => {
                            match mutation {
                                Mutation::Insert(word) => {
                                    v.insert(Mutation::Insert(word));
                                }
                                Mutation::Delete => {
                                    v.insert(Mutation::Delete);
                                }
                            }
                            None
                        }
                    };
                    (address, key, result)
                })
                .collect()
        });
        let mut out = Vec::with_capacity(results.len());
        for (address, key, result) in results {
            let r = match result {
                Some(Mutation::Insert(v)) => Some(v),
                Some(Mutation::Delete) => None,
                None => self.storage.query_state(&address, &key).await?,
            };
            out.push(r);
        }

        Ok(out)
    }

    async fn query_state(
        &self,
        address: &ContentAddress,
        key: &Key,
    ) -> anyhow::Result<Option<Word>> {
        let mutation = self
            .inner
            .apply(|i| i.state.get(address).and_then(|m| m.get(key)).copied());
        match mutation {
            Some(Mutation::Insert(v)) => Ok(Some(v)),
            Some(Mutation::Delete) => Ok(None),
            None => self.storage.query_state(address, key).await,
        }
    }
}
