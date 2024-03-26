use crate::{hash, serialize, sign, verify};
use secp256k1::hashes::hex::DisplayHex;
use test_utils::{intent_with_vars, keypair};

#[test]
fn test_serialize_intent() {
    let serialization = serialize(&intent_with_vars(1));
    let expected_serialization = "0100000000";
    assert_eq!(expected_serialization, serialization.to_lower_hex_string());
}

#[test]
fn test_hash_intent() {
    let hash = hash(&intent_with_vars(1));
    let expected_hash = "957b88b12730e646e0f33d3618b77dfa579e8231e3c59c7104be7165611c8027";
    assert_eq!(expected_hash, hash.to_lower_hex_string());
}

#[test]
fn test_sign_intent() {
    let signed = sign(intent_with_vars(1), keypair([0xcd; 32]).0);
    let expected_signature = concat!(
        "60b75a25dfa1a5b55d1b38dfdf2ff0f1ddf9028ac5ed282071ef5a766db8031d",
        "60b40d7b69691598f154860bf59be18ec822232dfe58ced59bf68e2522303688"
    );
    assert_eq!(expected_signature, signed.signature.to_lower_hex_string());
}

#[test]
fn test_verify_signature_intent() {
    let (sk, pk) = keypair([0xcd; 32]);
    let signed_by_first_keypair = sign(intent_with_vars(1), sk);
    assert!(verify(
        intent_with_vars(1),
        signed_by_first_keypair.signature,
        pk
    ));
    // verify against a different signature
    assert!(!verify(intent_with_vars(1), [0u8; 64], pk));
    // verify against another public key
    assert!(!verify(
        intent_with_vars(1),
        signed_by_first_keypair.signature,
        keypair([0xef; 32]).1
    ));
}
