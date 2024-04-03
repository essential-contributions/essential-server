//! Crypto operation implementations.

use crate::{asm::Word, error::CryptoError, OpResult, Stack};
use essential_types::convert::{
    bytes_from_word, u8_32_from_word_4, u8_64_from_word_8, word_4_from_u8_32,
};

/// `Crypto::Sha256` implementation.
pub(crate) fn sha256(stack: &mut Stack) -> OpResult<()> {
    use sha2::Digest;
    let data: Vec<_> =
        stack.pop_len_words(|words| Ok(bytes_from_words(words.iter().copied()).collect()))?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&data);
    let hash_bytes: [u8; 32] = hasher.finalize().into();
    let hash_words = word_4_from_u8_32(hash_bytes);
    stack.extend(hash_words);
    Ok(())
}

/// `Crypto::VerifyEd25519` implementation.
pub(crate) fn verify_ed25519(stack: &mut Stack) -> OpResult<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let pubkey_words = stack.pop4()?;
    let signature_words = stack.pop8()?;
    let data: Vec<_> =
        stack.pop_len_words(|words| Ok(bytes_from_words(words.iter().copied()).collect()))?;
    let pubkey_bytes = u8_32_from_word_4(pubkey_words);
    let pubkey = VerifyingKey::from_bytes(&pubkey_bytes).map_err(CryptoError::Ed25519)?;
    let signature_bytes = u8_64_from_word_8(signature_words);
    let signature = Signature::from_bytes(&signature_bytes);
    let valid = pubkey.verify(&data, &signature).is_ok();
    let word = Word::from(valid);
    stack.push(word);
    Ok(())
}

fn bytes_from_words(words: impl IntoIterator<Item = Word>) -> impl Iterator<Item = u8> {
    words.into_iter().flat_map(bytes_from_word)
}

#[cfg(test)]
mod tests {
    use crate::{
        asm::{Crypto, Op, Stack},
        exec_ops,
        test_util::*,
        types::{convert::bytes_from_word, Hash},
    };

    fn exec_ops_sha256(ops: &[Op]) -> Hash {
        let stack = exec_ops(ops.iter().copied(), TEST_ACCESS).unwrap();
        assert_eq!(stack.len(), 4);
        let bytes: Vec<u8> = stack.iter().copied().flat_map(bytes_from_word).collect();
        bytes.try_into().unwrap()
    }

    #[test]
    #[rustfmt::skip]
    fn sha256_1_word() {
        let ops = &[
            Stack::Push(0x0000000000000000).into(), // Data
            Stack::Push(1).into(), // Data Length
            Crypto::Sha256.into(),
        ];
        let hash = exec_ops_sha256(ops);
        // Value retrieved externally using command:
        // $ echo "0000000000000000" | xxd -r -p | sha256sum
        let expected = [
            0xaf, 0x55, 0x70, 0xf5, 0xa1, 0x81, 0x0b, 0x7a,
            0xf7, 0x8c, 0xaf, 0x4b, 0xc7, 0x0a, 0x66, 0x0f,
            0x0d, 0xf5, 0x1e, 0x42, 0xba, 0xf9, 0x1d, 0x4d,
            0xe5, 0xb2, 0x32, 0x8d, 0xe0, 0xe8, 0x3d, 0xfc,
        ];
        assert_eq!(&hash[..], &expected);
    }

    #[test]
    #[rustfmt::skip]
    fn sha256_3_words() {
        let ops = &[
            Stack::Push(0x00000000000000FF).into(), // Data
            Stack::Push(0x00000000000000FF).into(), // Data
            Stack::Push(0x00000000000000FF).into(), // Data
            Stack::Push(3).into(), // Data Length
            Crypto::Sha256.into(),
        ];
        let hash = exec_ops_sha256(ops);
        // Value retrieved externally using command:
        // $ echo "00000000000000FF00000000000000FF00000000000000FF" | xxd -r -p | sha256sum
        let expected = [
            0x58, 0x2d, 0xc8, 0xbd, 0xf8, 0xed, 0x36, 0x46,
            0x65, 0xa2, 0xd4, 0x59, 0x13, 0xc4, 0x79, 0x9f,
            0x38, 0x6e, 0xe0, 0xc2, 0x51, 0x96, 0x80, 0x81,
            0x00, 0xe2, 0xfc, 0x2d, 0xae, 0x75, 0x00, 0xd6,
        ];
        assert_eq!(&hash[..], &expected);
    }
}
