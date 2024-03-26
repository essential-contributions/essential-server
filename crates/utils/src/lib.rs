#[cfg(test)]
mod tests;

use essential_types::{Hash, Signature, Signed};
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use serde::Serialize;
use sha2::Digest;
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

/// Serialize data using postcard
pub fn serialize<T: Serialize>(t: &T) -> Vec<u8> {
    postcard::to_allocvec(t).expect("serde::Serialize trait should prevent serialization failure")
}

/// Hash data using SHA-256
pub fn hash<T: Serialize>(t: &T) -> Hash {
    let data = serialize(t);
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    hasher.update(&data);
    hasher.finalize().into()
}

/// Sign over data with secret key using secp256k1 curve
pub fn sign<T: Serialize>(data: T, sk: SecretKey) -> Signed<T> {
    let secp = Secp256k1::new();
    let hashed_data = hash(&data);
    let message = Message::from_digest(hashed_data);
    let signature: Signature = secp.sign_ecdsa(&message, &sk).serialize_compact();

    Signed { data, signature }
}

/// Verify signature against public key
pub fn verify<T: Serialize>(data: T, sig: Signature, pk: PublicKey) -> bool {
    let secp = Secp256k1::new();
    let hashed_data = hash(&data);
    let message = Message::from_digest(hashed_data);
    secp.verify_ecdsa(
        &message,
        &secp256k1::ecdsa::Signature::from_compact(&sig).unwrap(),
        &pk,
    )
    .is_ok()
}
