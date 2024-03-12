use ed25519_dalek::{Signature, VerifyingKey};

#[derive(Clone)]
pub struct Signed<T> {
    pub data: T,
    pub signature: Signature,
    pub public_key: VerifyingKey,
}
