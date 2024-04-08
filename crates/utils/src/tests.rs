use crate::{hash, recover, serialize, sign, verify};
use essential_types::intent::Intent;
use secp256k1::hashes::hex::DisplayHex;
use test_utils::{instantiate::Instantiate, keypair, sign_corrupted, sign_with_random_keypair};

#[test]
fn test_serialize_intent() {
    let serialization = serialize(&Intent::with_decision_variables(1));
    let expected_serialization = "0100000000";
    assert_eq!(expected_serialization, serialization.to_lower_hex_string());
}

#[test]
fn test_hash_intent() {
    let hash = hash(&Intent::with_decision_variables(1));
    let expected_hash = "957b88b12730e646e0f33d3618b77dfa579e8231e3c59c7104be7165611c8027";
    assert_eq!(expected_hash, hash.to_lower_hex_string());
}

#[test]
fn test_sign_intent() {
    let signed = sign(Intent::with_decision_variables(1), keypair([0xcd; 32]).0);
    let expected_signature = concat!(
        "60b75a25dfa1a5b55d1b38dfdf2ff0f1ddf9028ac5ed282071ef5a766db8031d",
        "60b40d7b69691598f154860bf59be18ec822232dfe58ced59bf68e2522303688"
    );
    assert_eq!(expected_signature, signed.signature.0.to_lower_hex_string());
}

#[test]
fn test_recover() {
    let (sk, pk) = keypair([0xcd; 32]);
    let data = Intent::with_decision_variables(1);
    let signed = sign(data.clone(), sk);
    let recovered_pk = recover(signed).unwrap();
    assert_eq!(pk, recovered_pk);
}

#[test]
fn test_fail_to_recover() {
    let (sk, _pk) = keypair([0xcd; 32]);
    let data = Intent::with_decision_variables(1);
    let signed = sign(data.clone(), sk);
    let mut corrupted_signed = signed.clone();
    corrupted_signed.signature.1 = (corrupted_signed.signature.1 + 1) % 4;
    assert!(recover(corrupted_signed).is_err());
}

#[test]
fn test_verify_signature() {
    let signed = sign_with_random_keypair(Intent::with_decision_variables(1));
    let signed_corrupted = sign_corrupted(Intent::with_decision_variables(1));
    assert!(verify(signed));
    assert!(!verify(signed_corrupted));
}
