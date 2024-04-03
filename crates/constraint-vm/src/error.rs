//! The types of errors that might occur throughout constraint checking.

use crate::{
    asm::{self, Word},
    Stack,
};
use core::fmt;
use thiserror::Error;

/// Intent checking error.
#[derive(Debug, Error)]
pub enum CheckError {
    #[error("errors occurred while executing one or more constraints: {0}")]
    ConstraintErrors(#[from] ConstraintErrors),
    #[error("one or more constraints were unsatisfied: {0}")]
    ConstraintsUnsatisfied(#[from] ConstraintsUnsatisfied),
}

/// The index of each failed constraint alongside the error it produced.
#[derive(Debug, Error)]
pub struct ConstraintErrors(pub Vec<(usize, ConstraintError)>);

/// The index of each constraint that was not satisfied.
#[derive(Debug, Error)]
pub struct ConstraintsUnsatisfied(pub Vec<usize>);

/// Shorthand for a `Result` where the error type is a `ConstraintError`.
pub type ConstraintResult<T> = Result<T, ConstraintError>;

/// Constraint checking error.
#[derive(Debug, Error)]
pub enum ConstraintError {
    #[error(
        "invalid constraint evaluation result\n  \
        expected: [0] (false) or [1] (true)\n  \
        found:    {0:?}"
    )]
    InvalidEvaluation(Stack),
    #[error("operation at index {0} failed: {1}")]
    Op(usize, OpError),
}

/// Shorthand for a `Result` where the error type is an `OpError`.
pub type OpResult<T> = Result<T, OpError>;

/// An individual operation failed during constraint checking error.
#[derive(Debug, Error)]
pub enum OpError {
    #[error("access operation error: {0}")]
    Access(#[from] AccessError),
    #[error("ALU operation error: {0}")]
    Alu(#[from] AluError),
    #[error("crypto operation error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("stack operation error: {0}")]
    Stack(#[from] StackError),
    #[error("bytecode error: {0}")]
    FromBytes(#[from] asm::FromBytesError),
}

/// Access operation error.
#[derive(Debug, Error)]
pub enum AccessError {
    #[error("decision variable slot out of bounds")]
    DecisionSlotOutOfBounds,
    #[error("solution data index out of bounds")]
    SolutionDataOutOfBounds,
    #[error("a cycle was detected between transient decision variables")]
    TransientDecisionVariableCycle,
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

impl fmt::Display for ConstraintErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("the constraints at the following indices failed: \n")?;
        for (ix, err) in &self.0 {
            f.write_str(&format!("  {ix}: {err}\n"))?;
        }
        Ok(())
    }
}

impl fmt::Display for ConstraintsUnsatisfied {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("the constraints at the following indices returned false: \n")?;
        for ix in &self.0 {
            f.write_str(&format!("  {ix}\n"))?;
        }
        Ok(())
    }
}
