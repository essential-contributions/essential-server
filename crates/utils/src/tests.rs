use crate::hash;
use test_utils::empty_intent;

#[test]
fn test_hash() {
    let expected_output = [
        136, 85, 80, 138, 173, 225, 110, 197, 115, 210, 30, 106, 72, 93, 253, 10, 118, 36, 8, 92,
        26, 20, 181, 236, 221, 100, 133, 222, 12, 104, 57, 164,
    ];
    let intent = empty_intent();
    let hash = hash(&intent);

    assert_eq!(expected_output, hash);
}

#[test]
fn test_sign() {}

#[test]
fn test_verify_signature() {}
