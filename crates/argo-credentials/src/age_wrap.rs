//! `age`-crate-backed encrypt/decrypt helpers for single credential
//! values.
//!
//! Each credential value is encrypted independently with the
//! workspace master key and stored as a base64-encoded age blob. This
//! keeps the wrapping orthogonal to TOML escaping rules — the
//! ciphertext is always ASCII regardless of the plaintext content —
//! and makes mixed plaintext/encrypted files trivial to parse: an
//! encrypted entry is a TOML table `{ value = "<blob>" }`, a legacy
//! plaintext entry is a bare string.

use crate::master_key::MasterKey;
use argo_types::{ArgoError, ArgoResult};
use base64::Engine;

const BLOB_ENGINE: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

/// Encrypt one credential value with the master key. Returns a
/// base64-encoded binary age blob.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when the underlying age library
/// fails to wrap the plaintext (e.g. recipient construction failure).
pub fn encrypt_value(plaintext: &str, key: &MasterKey) -> ArgoResult<String> {
    let recipient = key.recipient();
    let ciphertext = age::encrypt(&recipient, plaintext.as_bytes())
        .map_err(|e| ArgoError::Provider(format!("age encrypt: {e}")))?;
    Ok(BLOB_ENGINE.encode(ciphertext))
}

/// Decrypt one credential value previously produced by [`encrypt_value`].
///
/// # Errors
/// Returns [`ArgoError::Schema`] when the blob is not valid base64 or
/// age-formatted ciphertext, or [`ArgoError::Provider`] when the master
/// key cannot decrypt it (wrong key).
pub fn decrypt_value(blob_b64: &str, key: &MasterKey) -> ArgoResult<String> {
    let bytes = BLOB_ENGINE
        .decode(blob_b64.trim().as_bytes())
        .map_err(|e| ArgoError::Schema(format!("base64 decode: {e}")))?;
    let plaintext = age::decrypt(key.identity(), &bytes)
        .map_err(|e| ArgoError::Provider(format!("age decrypt: {e}")))?;
    String::from_utf8(plaintext)
        .map_err(|e| ArgoError::Schema(format!("decrypted value not UTF-8: {e}")))
}
