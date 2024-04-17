use super::*;
use essential_types::intent::Intent;
use test_utils::empty::Empty;

#[test]
fn test_intent() {
    let data = encode(&Intent::empty());
    let r: Intent = decode(&data).unwrap();
    assert_eq!(r, Intent::empty());
}

#[test]
fn test_intent_set() {
    let data = encode(&vec![Intent::empty()]);
    let r: Vec<Intent> = decode(&data).unwrap();
    assert_eq!(r, vec![Intent::empty()]);
}
