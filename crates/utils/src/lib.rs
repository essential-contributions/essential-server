#[cfg(test)]
mod tests;

use essential_types::{Hash, Signature, Signed};
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, PublicKey, Secp256k1, SecretKey,
};
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

    let (rec_id, sig) = secp
        .sign_ecdsa_recoverable(&message, &sk)
        .serialize_compact();
    let signature: Signature = Signature(sig, rec_id.to_i32().try_into().unwrap());
    Signed { data, signature }
}

/// Verify signature against data
pub fn verify<T: Serialize>(data: T, sig: Signature) -> bool {
    let secp = Secp256k1::new();
    let hashed_data = hash(&data);
    let message = Message::from_digest(hashed_data);
    if let Ok(pk) = recover_from_message(message, sig.clone()) {
        secp.verify_ecdsa(
            &message,
            &secp256k1::ecdsa::Signature::from_compact(&sig.0).unwrap(),
            &pk,
        )
        .is_ok()
    } else {
        false
    }
}

pub fn recover<T: Serialize>(signed: Signed<T>) -> anyhow::Result<PublicKey> {
    let hashed_data = hash(&signed.data);
    let message = Message::from_digest(hashed_data);
    recover_from_message(message, signed.signature)
}

pub fn recover_from_message(message: Message, signature: Signature) -> anyhow::Result<PublicKey> {
    let recovery_id = RecoveryId::from_i32(i32::from(signature.1 as u16))?;
    let recoverable_signature = RecoverableSignature::from_compact(&signature.0, recovery_id)?;
    let secp = Secp256k1::new();
    let public_key = secp.recover_ecdsa(&message, &recoverable_signature)?;
    Ok(public_key)
}
