//! Stack operation and related stack manipulation implementations.

use crate::{asm::Word, error::StackError, ConstraintResult};

/// The VM's `Stack`, i.e. a `Vec` of `Word`s updated during each step of execution.
///
/// A light wrapper around `Vec<Word>` providing helper methods specific to
/// essential VM execution.
#[derive(Clone, Debug, Default)]
pub struct Stack(Vec<Word>);

impl Stack {
    /// The DupFrom op implementation.
    pub(crate) fn dup_from(&mut self) -> ConstraintResult<()> {
        let rev_ix_w = self.pop1()?;
        let rev_ix = usize::try_from(rev_ix_w).map_err(|_| StackError::IndexOutOfBounds)?;
        let ix = self
            .len()
            .checked_sub(rev_ix)
            .and_then(|i| i.checked_sub(1))
            .ok_or(StackError::IndexOutOfBounds)?;
        let w = *self.get(ix).ok_or(StackError::IndexOutOfBounds)?;
        self.push(w);
        Ok(())
    }

    /// A wrapper around `Vec::pop`, producing an error in the case that the stack is empty.
    pub fn pop1(&mut self) -> ConstraintResult<Word> {
        Ok(self.pop().ok_or(StackError::Empty)?)
    }

    /// Pop the top 2 values from the stack.
    ///
    /// The last values popped appear first in the returned fixed-size array.
    pub fn pop2(&mut self) -> ConstraintResult<[Word; 2]> {
        let w1 = self.pop1()?;
        let w0 = self.pop1()?;
        Ok([w0, w1])
    }

    /// Pop the top 3 values from the stack.
    ///
    /// The last values popped appear first in the returned fixed-size array.
    pub fn pop3(&mut self) -> ConstraintResult<[Word; 3]> {
        let w2 = self.pop1()?;
        let [w0, w1] = self.pop2()?;
        Ok([w0, w1, w2])
    }

    /// Pop the top 4 values from the stack.
    ///
    /// The last values popped appear first in the returned fixed-size array.
    pub fn pop4(&mut self) -> ConstraintResult<[Word; 4]> {
        let w3 = self.pop1()?;
        let [w0, w1, w2] = self.pop3()?;
        Ok([w0, w1, w2, w3])
    }

    /// Pop the top 8 values from the stack.
    ///
    /// The last values popped appear first in the returned fixed-size array.
    pub fn pop8(&mut self) -> ConstraintResult<[Word; 8]> {
        let [w4, w5, w6, w7] = self.pop4()?;
        let [w0, w1, w2, w3] = self.pop4()?;
        Ok([w0, w1, w2, w3, w4, w5, w6, w7])
    }

    /// Pop 1 word from the stack, apply the given function and push the returned word.
    pub fn pop1_push1<F>(&mut self, f: F) -> ConstraintResult<()>
    where
        F: FnOnce(Word) -> ConstraintResult<Word>,
    {
        let w = self.pop1()?;
        let x = f(w)?;
        self.push(x);
        Ok(())
    }

    /// Pop 2 words from the stack, apply the given function and push the returned word.
    pub fn pop2_push1<F>(&mut self, f: F) -> ConstraintResult<()>
    where
        F: FnOnce(Word, Word) -> ConstraintResult<Word>,
    {
        let [w0, w1] = self.pop2()?;
        let x = f(w0, w1)?;
        self.push(x);
        Ok(())
    }

    /// Pop 8 words from the stack, apply the given function and push the returned word.
    pub fn pop8_push1<F>(&mut self, f: F) -> ConstraintResult<()>
    where
        F: FnOnce([Word; 8]) -> ConstraintResult<Word>,
    {
        let ws = self.pop8()?;
        let x = f(ws)?;
        self.push(x);
        Ok(())
    }

    /// Pop 1 word from the stack, apply the given function and push the 2 returned words.
    pub fn pop1_push2<F>(&mut self, f: F) -> ConstraintResult<()>
    where
        F: FnOnce(Word) -> ConstraintResult<[Word; 2]>,
    {
        let w = self.pop1()?;
        let xs = f(w)?;
        self.extend(xs);
        Ok(())
    }

    /// Pop 2 words from the stack, apply the given function and push the 2 returned words.
    pub fn pop2_push2<F>(&mut self, f: F) -> ConstraintResult<()>
    where
        F: FnOnce(Word, Word) -> ConstraintResult<[Word; 2]>,
    {
        let [w0, w1] = self.pop2()?;
        let xs = f(w0, w1)?;
        self.extend(xs);
        Ok(())
    }

    /// Pop 2 words from the stack, apply the given function and push the 4 returned words.
    pub fn pop2_push4<F>(&mut self, f: F) -> ConstraintResult<()>
    where
        F: FnOnce(Word, Word) -> ConstraintResult<[Word; 4]>,
    {
        let [w0, w1] = self.pop2()?;
        let xs = f(w0, w1)?;
        self.extend(xs);
        Ok(())
    }

    /// Pop a length value from the top of the stack and return it.
    pub fn pop_len(&mut self) -> ConstraintResult<usize> {
        let len_word = self.pop1()?;
        let len = usize::try_from(len_word).map_err(|_| StackError::IndexOutOfBounds)?;
        Ok(len)
    }

    /// Pop the length from the top of the stack, then pop and provide that many
    /// words to the given function.
    pub fn pop_len_words<F, O>(&mut self, f: F) -> ConstraintResult<O>
    where
        F: FnOnce(&[Word]) -> ConstraintResult<O>,
    {
        let len = self.pop_len()?;
        let ix = self
            .len()
            .checked_sub(len)
            .ok_or(StackError::IndexOutOfBounds)?;
        f(&self[ix..])
    }
}

impl From<Stack> for Vec<Word> {
    fn from(stack: Stack) -> Self {
        stack.0
    }
}

impl From<Vec<Word>> for Stack {
    fn from(vec: Vec<Word>) -> Self {
        Self(vec)
    }
}

impl core::ops::Deref for Stack {
    type Target = Vec<Word>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for Stack {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
