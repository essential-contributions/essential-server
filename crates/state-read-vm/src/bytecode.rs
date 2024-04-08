//! Items related to bytecode representation for the State Read VM.

use crate::asm::{FromBytesError, Op, Opcode};

/// A memory efficient representation of a sequence of operations parsed from bytecode.
///
/// State Read interpretation differs slightly from the Constraint
/// interpretation in that State Read operations can include arbitrary control flow.
///
/// This means that unlike constraint execution where we reliably parse one
/// operation at a time, we must store operations in case control flow would
/// require re-visiting a previous operation.
///
/// One simple solution might be to use a `Vec<Op>`, however it is important to
/// consider that the size of each element within a `Vec<Op>` will be equal to
/// the size of the discriminant plus the largest `Op` variant size (today, this
/// is `Push(Word)`, but this may change as new operations are added). This can
/// have memory requirement implications for programs with large numbers of ops.
///
/// To avoid this issue, we instead store the raw "packed" bytecode alongside
/// a list of indices into the bytecode representing the location of each
/// operation.
#[derive(Clone, Debug, Default)]
pub struct BytecodeMapped {
    /// The bytecode representation of a program's operations.
    bytecode: Vec<u8>,
    /// The index of each op within the bytecode slice.
    ///
    /// Indices are guaranteed to be valid by construction and point to a valid operation.
    map: Vec<usize>,
}

/// A slice into a [`BytecodeMapped`] instance.
#[derive(Clone, Copy, Debug)]
pub struct BytecodeMappedSlice<'a> {
    /// The full bytecode slice from the original `BytecodeMapped`.
    bytecode: &'a [u8],
    /// Some subslice into the `map` of the original `BytecodeMapped`.
    map: &'a [usize],
}

impl BytecodeMapped {
    /// Push a single operation onto the bytecode mapping.
    pub fn push_op(&mut self, op: Op) {
        self.map.push(self.bytecode.len());
        self.bytecode.extend(op.to_bytes());
    }

    /// The inner slice of bytecode that has been mapped.
    pub fn bytecode(&self) -> &[u8] {
        &self.bytecode
    }

    /// The slice of operation indices within the mapped bytecode.
    pub fn map(&self) -> &[usize] {
        &self.map
    }

    /// Slice the op indices from the given index.
    ///
    /// The returned slice represents the remainder of the program from the given op.
    ///
    /// Returns `None` if `start` is out of range of the `map` slice.
    pub fn ops_from(&self, start: usize) -> Option<BytecodeMappedSlice> {
        Some(BytecodeMappedSlice {
            bytecode: &self.bytecode,
            map: self.map.get(start..)?,
        })
    }

    /// The operation at the given index.
    pub fn op(&self, ix: usize) -> Option<Op> {
        let slice = self.ops_from(ix)?;
        slice.ops().next()
    }

    /// An iterator yielding all mapped operations.
    pub fn ops(&self) -> impl '_ + Iterator<Item = Op> {
        expect_ops_from_indices(&self.bytecode, self.map.iter().copied())
    }
}

impl<'a> BytecodeMappedSlice<'a> {
    /// The slice of operation indices within the mapped bytecode.
    pub fn map(self) -> &'a [usize] {
        self.map
    }

    /// An iterator yielding all mapped operations represented by this slice.
    pub fn ops(self) -> impl 'a + Iterator<Item = Op> {
        expect_ops_from_indices(self.bytecode, self.map.iter().copied())
    }
}

// Allow for collecting a `BytecodeMapped` from an iterator over `Op`s.
impl FromIterator<Op> for BytecodeMapped {
    fn from_iter<T: IntoIterator<Item = Op>>(iter: T) -> Self {
        let mut mapped = BytecodeMapped::default();
        iter.into_iter().for_each(|op| mapped.push_op(op));
        mapped
    }
}

/// Given a bytecode slice and an operation mapping that is assumed to have been
/// previously validated, produce an iterator yielding all associated operations.
fn expect_ops_from_indices<'a>(
    bytecode: &'a [u8],
    map: impl 'a + IntoIterator<Item = usize>,
) -> impl 'a + Iterator<Item = Op> {
    const EXPECT_MSG: &str = "validated upon construction";
    map.into_iter().map(|ix| {
        let mut bytes = bytecode[ix..].iter().copied();
        parse_op(&mut bytes).expect(EXPECT_MSG).expect(EXPECT_MSG)
    })
}

/// Parse an `Op` from the given byte iterator. Returns `None` if the iterator is empty.
fn parse_op(mut bytes: &mut impl Iterator<Item = u8>) -> Option<Result<Op, FromBytesError>> {
    let opcode_byte = bytes.next()?;
    let res = Opcode::try_from(opcode_byte)
        .map_err(From::from)
        .and_then(|opcode| opcode.parse_op(&mut bytes).map_err(From::from));
    Some(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm;

    // Ensure we can collect a `Result<BytecodeMapped, _>` from an iterator of `Result<Op, _>`.
    #[test]
    fn mapped_from_op_results() {
        let results: &[Result<Op, _>] = &[
            Ok(asm::Stack::Push(6).into()),
            Ok(asm::Stack::Push(7).into()),
            Ok(asm::Alu::Mul.into()),
            Ok(asm::Pred::Eq.into()),
        ];
        let mapped: Result<BytecodeMapped, ()> = results.iter().cloned().collect();
        mapped.unwrap();
    }
}
