use super::*;
use essential_types::{contract::Contract, predicate::Predicate};
use test_utils::empty::Empty;

#[test]
fn test_predicate() {
    let data = encode(&Predicate::empty());
    let r: Predicate = decode(&data).unwrap();
    assert_eq!(r, Predicate::empty());
}

#[test]
fn test_contract() {
    let data = encode(&Contract::without_salt(vec![Predicate::empty()]));
    let r: Contract = decode(&data).unwrap();
    assert_eq!(r, Contract::without_salt(vec![Predicate::empty()]));
}
