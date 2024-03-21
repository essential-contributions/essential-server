//! This module contains place holder types that will exist in `essential-types` crate.

use essential_types::{solution::Solution, Key, KeyRange};
use serde::{Deserialize, Serialize};

/// Placeholder for real type that will be in `essential-types` crate.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Signature;

/// Placeholder for real type that will be in `essential-types` crate.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Signed<T> {
    pub data: T,
    pub signature: Signature,
}

/// Placeholder for real type that will be in `essential-types` crate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Batch {
    pub solutions: Vec<Signed<Solution>>,
}

/// Placeholder for real type that will be in `essential-types` crate.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StorageLayout;

/// Placeholder for real type that will be in `essential-types` crate.
#[allow(dead_code)]
pub struct KeyRangeIter<'a>(&'a KeyRange);

/// Placeholder for real functionality that will be in `essential-types` crate.
pub fn key_range_length(_range: &KeyRange) -> usize {
    todo!()
}

/// Placeholder for real functionality that will be in `essential-types` crate.
pub fn key_range_iter(range: &KeyRange) -> impl Iterator<Item = &Key> {
    KeyRangeIter(range)
}

/// Placeholder for real functionality that will be in `essential-types` crate.
impl<'a> Iterator for KeyRangeIter<'a> {
    type Item = &'a Key;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
