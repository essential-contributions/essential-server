//! Crypto operation implementations.

use crate::{asm::Word, pop4, pop8, pop_len_words, CheckResult, Stack};
use essential_types::convert::{
    bytes_from_word, u8_32_from_word_4, u8_64_from_word_8, word_4_from_u8_32,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("failed to verify ed25519 signature: {0}")]
    Ed25519(#[from] ed25519_dalek::ed25519::Error),
}

pub fn sha256(stack: &mut Stack) -> CheckResult<()> {
    use sha2::Digest;
    let data: Vec<_> = pop_len_words(stack, |words| {
        Ok(bytes_from_words(words.iter().copied()).collect())
    })?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&data);
    let hash_bytes: [u8; 32] = hasher.finalize().into();
    let hash_words = word_4_from_u8_32(hash_bytes);
    stack.extend(hash_words);
    Ok(())
}

pub fn verify_ed25519(stack: &mut Stack) -> CheckResult<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let pubkey_words = pop4(stack)?;
    let signature_words = pop8(stack)?;
    let data: Vec<_> = pop_len_words(stack, |words| {
        Ok(bytes_from_words(words.iter().copied()).collect())
    })?;
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
