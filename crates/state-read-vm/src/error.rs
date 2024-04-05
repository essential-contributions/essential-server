#[doc(inline)]
use crate::asm::{self, Word};
pub use constraint_vm::error::{StackError, StackResult};
use thiserror::Error;

/// Shorthand for a `Result` where the error type is a `StateReadError`.
pub type StateReadResult<T, E> = Result<T, StateReadError<E>>;

/// State read execution failure.
#[derive(Debug, Error)]
pub enum StateReadError<E> {
    /// The operation at the specified index failed.
    #[error("operation at index {0} failed: {1}")]
    Op(usize, OpError<E>),
}

/// Shorthand for a `Result` where the error type is an `OpError`.
pub type OpResult<T, E> = Result<T, OpError<E>>;

/// An individual operation failed during state read execution.
#[derive(Debug, Error)]
pub enum OpError<E> {
    /// An error occurred during a `Constraint` operation.
    #[error("constraint operation error: {0}")]
    Constraint(#[from] constraint_vm::error::OpError),
    /// An error occurred during a `ControlFlow` operation.
    #[error("control flow operation error: {0}")]
    ControlFlow(#[from] ControlFlowError),
    /// An error occurred during a `Memory` operation.
    #[error("memory operation error: {0}")]
    Memory(#[from] MemoryError),
    /// An error occurred during a `Stack` operation.
    #[error("stack operation error: {0}")]
    Stack(#[from] StackError),
    /// An error occurred during a `StateRead` operation.
    #[error("state read operation error: {0}")]
    StateRead(E),
    /// An error occurred while parsing an operation from bytes.
    #[error("bytecode error: {0}")]
    FromBytes(#[from] asm::FromBytesError),
}

/// Errors occuring during `ControlFlow` operation.
#[derive(Debug, Error)]
pub enum ControlFlowError {
    /// A `JumpIf` operation encountered an invalid condition.
    ///
    /// Condition values must be 0 (false) or 1 (true).
    #[error("invalid condition value {0}, expected 0 (false) or 1 (true)")]
    InvalidJumpIfCondition(Word),
}

/// Shorthand for a `Result` where the error type is an `MemoryError`.
pub type MemoryResult<T> = Result<T, MemoryError>;

/// Errors occuring during `Memory` operation.
#[derive(Debug, Error)]
pub enum MemoryError {
    /// Attempted to access a memory index that was out of bounds.
    #[error("index out of bounds")]
    IndexOutOfBounds,
    /// An operation would have caused memory to overflow.
    #[error("operation would cause memory to overflow")]
    Overflow,
}
