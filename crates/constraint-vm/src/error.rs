//! The types of errors that might occur throughout constraint checking.

use crate::asm::{self, Word};
use thiserror::Error;

/// The top-level error type for constraint checking.
#[derive(Debug, Error)]
pub enum CheckError {
    #[error("access error: {0}")]
    Access(#[from] AccessError),
    #[error("ALU operation error: {0}")]
    Alu(#[from] AluError),
    #[error("crypto operation error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("stack operation error: {0}")]
    Stack(#[from] StackError),
    #[error("bytecode error: {0}")]
    FromBytes(#[from] asm::FromBytesError),
    #[error("invalid constraint evaluation result {0}, exepcted `0` (false) or `1` (true)")]
    InvalidConstraintValue(Word),
}

/// Access operation error.
#[derive(Debug, Error)]
pub enum AccessError {
    #[error("decision variable slot out of bounds")]
    DecisionSlotOutOfBounds,
    #[error("state slot out of bounds")]
    StateSlotOutOfBounds,
    #[error("invalid state slot delta: expected `0` or `1`, found {0}")]
    InvalidStateSlotDelta(Word),
    #[error("attempted to access a state slot that has no value")]
    StateSlotWasNone,
}

/// ALU operation error.
#[derive(Debug, Error)]
pub enum AluError {
    #[error("word overflow")]
    Overflow,
    #[error("word underflow")]
    Underflow,
    #[error("attempted to divide by zero")]
    DivideByZero,
}

/// Crypto operation error.
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("failed to verify ed25519 signature: {0}")]
    Ed25519(#[from] ed25519_dalek::ed25519::Error),
}

/// Stack operation error.
#[derive(Debug, Error)]
pub enum StackError {
    #[error("attempted to pop an empty stack")]
    Empty,
    #[error("indexed stack out of bounds")]
    IndexOutOfBounds,
}
