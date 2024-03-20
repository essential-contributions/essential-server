#[cfg(test)]
mod tests;

use essential_types::Hash as EssentialHash;
use placeholder::{Signature, Signed};
use postcard::ser_flavors::{AllocVec, Flavor};
use secp256k1::hashes::{sha256::Hash as HashOutput, Hash};
use std::sync::Mutex;

pub struct Lock<T> {
    data: Mutex<T>,
}

impl<T> Lock<T> {
    pub fn new(data: T) -> Self {
        Lock {
            data: Mutex::new(data),
        }
    }

    pub fn apply<U>(&self, f: impl FnOnce(&mut T) -> U) -> U {
        f(&mut self.data.lock().unwrap())
    }
}

pub fn hash<T: serde::ser::Serialize>(t: &T) -> EssentialHash {
    let mut serializer = postcard::Serializer {
        output: AllocVec::default(),
    };
    t.serialize(&mut serializer).unwrap();
    let serialized_data: Vec<u8> = serializer.output.finalize().unwrap();
    let hash: HashOutput = Hash::hash(serialized_data.as_slice());
    return hash.as_byte_array().to_owned();
}

pub fn sign<T>(data: T) -> Signed<T> {
    Signed {
        data,
        signature: (),
    }
}
