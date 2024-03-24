//! This module contains place holder types that will exist in `essential-types` crate.

use essential_types::{solution::Solution, Key, KeyRange};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Placeholder for real type that will be in `essential-types` crate.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Signature {
    #[serde(
        serialize_with = "serialize_signature",
        deserialize_with = "deserialize_signature"
    )]
    pub bytes: [u8; 64],
}

fn serialize_signature<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bytes(bytes)
}

fn deserialize_signature<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
    if bytes.len() == 64 {
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    } else {
        Err(D::Error::custom("Expected a byte array of length 64"))
    }
}

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
