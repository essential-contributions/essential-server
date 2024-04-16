use crate::deploy::validate::{
    validate_intent, validate_intents, MAX_CONSTRAINTS, MAX_CONSTRAINT_SIZE_IN_BYTES,
    MAX_DECISION_VARIABLES, MAX_DIRECTIVE_SIZE, MAX_INTENTS, MAX_NUM_STATE_SLOTS, MAX_STATE_LEN,
    MAX_STATE_READS, MAX_STATE_READ_SIZE_IN_BYTES,
};
use essential_types::{
    intent::{Directive, Intent},
    slots::{Slots, StateSlot},
};
use test_utils::{empty::Empty, sign_corrupted, sign_with_random_keypair};

#[test]
#[should_panic(expected = "Too many decision variables")]
fn test_fail_too_many_decision_variables() {
    let mut intent = Intent::empty();
    intent.slots = Slots {
        decision_variables: MAX_DECISION_VARIABLES + 1,
        state: Default::default(),
    };
    validate_intent(&intent).unwrap();
}

#[test]
#[should_panic(expected = "Too many state slots")]
fn test_fail_too_many_state_slots() {
    let mut intent = Intent::empty();
    intent.slots = Slots {
        decision_variables: Default::default(),
        state: (0..MAX_NUM_STATE_SLOTS + 1)
            .map(|_| StateSlot::empty())
            .collect(),
    };
    validate_intent(&intent).unwrap();
}

#[test]
#[should_panic(expected = "Invalid slots state length")]
fn test_fail_invalid_state_slots_length() {
    let mut intent = Intent::empty();
    intent.slots = Slots {
        decision_variables: Default::default(),
        state: vec![StateSlot {
            index: u32::MAX,
            amount: 1,
            program_index: Default::default(),
        }],
    };
    validate_intent(&intent).unwrap();
}

#[test]
#[should_panic(expected = "Slots state length too large")]
fn test_fail_state_slots_length_too_large() {
    let mut intent = Intent::empty();
    intent.slots = Slots {
        decision_variables: Default::default(),
        state: vec![StateSlot {
            index: Default::default(),
            amount: MAX_STATE_LEN as u32 + 1,
            program_index: Default::default(),
        }],
    };
    validate_intent(&intent).unwrap();
}

#[test]
fn test_empty_intent() {
    let intent = Intent::empty();
    let intent = sign_with_random_keypair(vec![intent]);
    validate_intents(&intent).unwrap();
}

#[test]
#[should_panic(expected = "Failed to verify intent set signature")]
fn test_fail_invalid_signature() {
    let intent = Intent::empty();
    let intent = sign_corrupted(vec![intent]);
    validate_intents(&intent).unwrap();
}

#[test]
#[should_panic(expected = "Too many intents")]
fn test_fail_too_many_intents() {
    let intents: Vec<Intent> = (0..MAX_INTENTS + 1).map(|_| Intent::empty()).collect();
    let intents = sign_with_random_keypair(intents);
    validate_intents(&intents).unwrap();
}

#[test]
#[should_panic(expected = "Directive too large")]
fn test_fail_directive_too_large() {
    let mut intent = Intent::empty();
    intent.directive = Directive::Maximize(vec![0; MAX_DIRECTIVE_SIZE + 1]);
    validate_intent(&intent).unwrap();
}

#[test]
#[should_panic(expected = "Too many state reads")]
fn test_fail_too_many_state_reads() {
    let mut intent = Intent::empty();
    intent.state_read = (0..MAX_STATE_READS + 1).map(|_| vec![]).collect();
    validate_intent(&intent).unwrap();
}

#[test]
#[should_panic(expected = "State read too large")]
fn test_fail_state_read_too_large() {
    let mut intent = Intent::empty();
    intent.state_read = vec![vec![0u8; MAX_STATE_READ_SIZE_IN_BYTES + 1]];
    validate_intent(&intent).unwrap();
}

#[test]
#[should_panic(expected = "Too many constraints")]
fn test_fail_too_many_constraints() {
    let mut intent = Intent::empty();
    intent.constraints = (0..MAX_CONSTRAINTS + 1).map(|_| vec![]).collect();
    validate_intent(&intent).unwrap();
}

#[test]
#[should_panic(expected = "Constraint too large")]
fn test_fail_constraint_too_large() {
    let mut intent = Intent::empty();
    intent.constraints = vec![vec![0u8; MAX_CONSTRAINT_SIZE_IN_BYTES + 1]];
    validate_intent(&intent).unwrap();
}
