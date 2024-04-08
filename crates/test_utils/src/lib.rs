pub mod empty;
pub mod instantiate;
pub mod solution;

use essential_types::{Signature, Signed};
use secp256k1::{rand::rngs::OsRng, PublicKey, Secp256k1, SecretKey};
use serde::Serialize;
use utils::sign;

pub fn duration_secs(secs: u64) -> std::time::Duration {
    std::time::Duration::from_secs(secs)
}

pub fn random_keypair() -> (SecretKey, PublicKey) {
    let secp = Secp256k1::new();
    secp.generate_keypair(&mut OsRng)
}

pub fn keypair(key: [u8; 32]) -> (SecretKey, PublicKey) {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&key).unwrap();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    (secret_key, public_key)
}

pub fn sign_with_random_keypair<T: Serialize>(data: T) -> Signed<T> {
    sign(data, random_keypair().0)
}

pub fn sign_corrupted<T: Serialize>(data: T) -> Signed<T> {
    let mut signed = sign(data, random_keypair().0);
    // TODO: is this a good way to create a corrupted signature?
    signed.signature = Signature([0u8; 64], 0);
    signed
}
