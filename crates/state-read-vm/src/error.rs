#[doc(inline)]
pub use constraint_vm::error::{StackError, StackResult};
use thiserror::Error;

pub type StateReadResult<T, E> = Result<T, StateReadError<E>>;

#[derive(Debug)]
pub enum StateReadError<E> {
    Op(usize, OpError<E>),
}

pub type OpResult<T, E> = Result<T, OpError<E>>;

#[derive(Debug, Error)]
pub enum OpError<E> {
    #[error("")]
    Constraint(#[from] constraint_vm::error::OpError),
    #[error("")]
    ControlFlow(#[from] ControlFlowError),
    #[error("")]
    Memory(#[from] MemoryError),
    #[error("")]
    Stack(#[from] StackError),
    #[error("")]
    StateRead(#[from] E),
}

#[derive(Debug, Error)]
pub enum ControlFlowError {}

#[derive(Debug, Error)]
pub enum MemoryError {}
