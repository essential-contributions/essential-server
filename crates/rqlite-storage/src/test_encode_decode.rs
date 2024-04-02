use essential_types::intent::Intent;
use test_utils::empty_intent;

use super::*;

#[test]
fn test_intent() {
    let data = encode(&empty_intent());
    let r: Intent = decode(&data).unwrap();
    assert_eq!(r, empty_intent());
}

#[test]
fn test_intent_set() {
    let data = encode(&vec![empty_intent()]);
    let r: Vec<Intent> = decode(&data).unwrap();
    assert_eq!(r, vec![empty_intent()]);
}
