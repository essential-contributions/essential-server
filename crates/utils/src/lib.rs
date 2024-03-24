#[cfg(test)]
mod tests;

use essential_types::Hash as EssentialHash;
use placeholder::{Signature as EssentialSignature, Signed};
use postcard::ser_flavors::{AllocVec, Flavor};
use secp256k1::{
    ecdsa::Signature,
    hashes::{sha256::Hash as HashOutput, Hash},
    Message, PublicKey, Secp256k1, SecretKey,
};
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

pub fn serialize<T: serde::ser::Serialize>(t: &T) -> Vec<u8> {
    let mut serializer = postcard::Serializer {
        output: AllocVec::default(),
    };
    t.serialize(&mut serializer).unwrap();
    serializer.output.finalize().unwrap()
}

pub fn hash<T: serde::ser::Serialize>(t: &T) -> EssentialHash {
    let serialized_data: Vec<u8> = serialize(t);
    let hash: HashOutput = Hash::hash(serialized_data.as_slice());
    return hash.as_byte_array().to_owned();
}

pub fn sign<T: serde::ser::Serialize>(data: T, sk: SecretKey) -> Signed<T> {
    let secp = Secp256k1::new();
    let hashed_data = hash(&data);
    let message = Message::from_digest(hashed_data);
    let signature: EssentialSignature = EssentialSignature {
        bytes: secp.sign_ecdsa(&message, &sk).serialize_compact(),
    };

    Signed { data, signature }
}

pub fn verify<T: serde::ser::Serialize>(data: T, sig: EssentialSignature, pk: PublicKey) -> bool {
    let secp = Secp256k1::new();
    let hashed_data = hash(&data);
    let message = Message::from_digest(hashed_data);
    secp.verify_ecdsa(&message, &Signature::from_compact(&sig.bytes).unwrap(), &pk)
        .is_ok()
}
