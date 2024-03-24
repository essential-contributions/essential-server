use crate::{hash, serialize, sign, verify};
use essential_state_asm::{
    constraint_asm::{Access, Pred},
    ControlFlow, Memory, Op, State, StateReadOp,
};
use essential_types::{
    intent::{Directive, Intent},
    slots::{Slots, StateSlot},
};
use placeholder::Signature;
use secp256k1::{PublicKey, Secp256k1, SecretKey};

fn keypair(data: [u8; 32]) -> (SecretKey, PublicKey) {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&data).unwrap();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    (secret_key, public_key)
}

fn empty_intent() -> Intent {
    Intent {
        slots: Slots {
            decision_variables: 0,
            state: Default::default(),
        },
        state_read: Default::default(),
        constraints: Default::default(),
        directive: Directive::Satisfy,
    }
}

fn intent() -> Intent {
    // state foo: int = state.extern.get(extern_address, key, 1);
    let state_read: Vec<StateReadOp> = vec![
        // allocate memory
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Memory(Memory::Alloc),
        // // extern_addres
        // StateReadOp::Constraint(Op::Push(deployed_address[0])),
        // StateReadOp::Constraint(Op::Push(deployed_address[1])),
        // StateReadOp::Constraint(Op::Push(deployed_address[2])),
        // StateReadOp::Constraint(Op::Push(deployed_address[3])),
        // key
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::Constraint(Op::Push(1)),
        // amount
        StateReadOp::Constraint(Op::Push(1)),
        StateReadOp::State(State::StateReadWordRangeExtern),
        // end of program
        StateReadOp::ControlFlow(ControlFlow::Halt),
    ];
    let state_read = serialize(&state_read);
    let state_read = vec![state_read];

    let mut constraints = vec![];
    let constraint: Vec<Op> = vec![
        // constraint foo == 7;
        Op::Push(0),
        Op::Push(0),
        Op::Access(Access::State),
        Op::Push(7),
        Op::Pred(Pred::Eq),
        // var bar: int = 11;
        // constraint bar == 11;
        Op::Push(0),
        Op::Access(Access::DecisionVar),
        Op::Push(11),
        Op::Pred(Pred::Eq),
        Op::Pred(Pred::And),
    ];

    let constraint = serialize(&constraint);

    constraints.push(constraint);

    Intent {
        slots: Slots {
            decision_variables: 1,
            state: vec![StateSlot {
                index: 0,
                amount: 1,
                program_index: 0,
            }],
        },
        state_read,
        constraints,
        directive: Directive::Satisfy,
    }
}

#[test]
fn test_serialize_empty_intent() {
    let serialization = serialize(&empty_intent());
    let expected_output = vec![0, 0, 0, 0, 0];
    assert_eq!(expected_output, serialization);
}

#[test]
fn test_hash_empty_intent() {
    let hash = hash(&empty_intent());
    let expected_output = [
        136, 85, 80, 138, 173, 225, 110, 197, 115, 210, 30, 106, 72, 93, 253, 10, 118, 36, 8, 92,
        26, 20, 181, 236, 221, 100, 133, 222, 12, 104, 57, 164,
    ];

    assert_eq!(expected_output, hash);
}

#[test]
fn test_sign_empty_intent() {
    let (sk, _pk) = keypair([0xcd; 32]);
    let signed = sign(empty_intent(), sk);
    let expected_signature = [
        146, 133, 75, 124, 21, 95, 239, 201, 25, 157, 10, 217, 52, 174, 37, 34, 225, 19, 164, 163,
        176, 243, 4, 9, 117, 106, 222, 55, 230, 234, 40, 14, 117, 183, 227, 208, 128, 95, 254, 244,
        249, 120, 91, 254, 163, 186, 232, 125, 6, 71, 74, 180, 107, 30, 135, 138, 85, 182, 185, 19,
        224, 51, 60, 197,
    ];
    assert_eq!(expected_signature, signed.signature.bytes);
}

#[test]
fn test_verify_signature_empty_intent() {
    let (sk, pk) = keypair([0xcd; 32]);
    let signed = sign(empty_intent(), sk);

    assert!(verify(empty_intent(), signed.signature, pk));
}

#[test]
fn test_serialize() {
    let serialization = serialize(&intent());
    let expected_output = vec![
        1, 1, 0, 1, 0, 1, 25, 9, 0, 0, 2, 3, 0, 0, 0, 2, 0, 0, 2, 0, 0, 2, 0, 0, 2, 0, 0, 2, 1, 1,
        2, 0, 1, 21, 10, 0, 0, 0, 0, 7, 2, 0, 14, 5, 0, 0, 0, 7, 0, 0, 22, 5, 0, 5, 6, 0,
    ];
    assert_eq!(expected_output, serialization);
}

#[test]
fn test_hash() {
    let hash = hash(&intent());
    let expected_output = [
        35, 186, 20, 183, 53, 135, 108, 224, 97, 2, 236, 166, 129, 33, 125, 135, 58, 164, 140, 32,
        84, 45, 167, 84, 197, 242, 200, 255, 26, 153, 40, 219,
    ];

    assert_eq!(expected_output, hash);
}

#[test]
fn test_sign() {
    let (sk, _pk) = keypair([0xcd; 32]);
    let signed = sign(intent(), sk);
    let expected_signature = [
        132, 32, 165, 121, 23, 75, 22, 169, 255, 144, 9, 164, 46, 163, 125, 20, 36, 209, 120, 185,
        81, 210, 19, 202, 93, 15, 159, 246, 212, 19, 208, 73, 45, 182, 91, 17, 47, 84, 101, 103,
        242, 102, 43, 83, 199, 202, 196, 250, 83, 97, 98, 90, 193, 199, 218, 74, 161, 59, 169, 71,
        167, 205, 16, 199,
    ];
    assert_eq!(expected_signature, signed.signature.bytes);
}

#[test]
fn test_verify_signature() {
    let (sk, pk) = keypair([0xcd; 32]);
    let signed = sign(intent(), sk);

    let (sk2, pk2) = keypair([0xef; 32]);

    assert!(verify(intent(), signed.signature.clone(), pk));
    assert!(!verify(intent(), Signature { bytes: [0u8; 64] }, pk));
    assert!(!verify(intent(), signed.signature, pk2));
}
