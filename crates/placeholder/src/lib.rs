//! This module contains place holder types that will exist in `essential-types` crate.

use essential_types::solution::Solution;

/// Placeholder for real type that will be in `essential-types` crate.
pub type Signature = ();

/// Placeholder for real type that will be in `essential-types` crate.
pub type PublicKey = ();

/// Placeholder for real type that will be in `essential-types` crate.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Signed<T> {
    pub data: T,
    pub signature: Signature,
    pub public_key: PublicKey,
}

/// Placeholder for real type that will be in `essential-types` crate.
pub struct Batch {
    pub solutions: Vec<Signed<Solution>>,
}

/// Placeholder for real type that will be in `essential-types` crate.
pub type StorageLayout = ();

/// Placeholder for real type that will be in `essential-types` crate.
pub type EoaPermit = ();
