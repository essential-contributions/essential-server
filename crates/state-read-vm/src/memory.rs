//! Memory operation implementations and related items.

use crate::asm::Word;

/// The maximum number of words stored in memory.
pub const SIZE_LIMIT: usize = 4096;

/// A type representing the VM's memory.
#[derive(Debug, Default)]
pub struct Memory(Vec<Option<Word>>);
