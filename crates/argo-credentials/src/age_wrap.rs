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

/// Logical key version reported by [`decrypt_value_with_fallback`] so
/// the caller knows whether the blob is on the current encryption key
/// or whether it still needs the lazy-migration re-encrypt.
///
/// The variants are 1-indexed to match operator-facing documentation
/// (`key_version: 1` = pre-2026-05-23 process-global key; `key_version: 2`
/// = per-tenant HKDF-derived key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum KeyVersion {
    /// Decrypted under the **legacy** process-global master key —
    /// caller must re-encrypt with the per-tenant key and write back.
    Legacy = 1,
    /// Decrypted under the current per-tenant master key — no action.
    PerTenant = 2,
}

/// Decrypted plaintext plus the key version that produced it.
#[derive(Debug, Clone)]
pub struct DecryptedValue {
    /// The plaintext value.
    pub plaintext: String,
    /// `Legacy` when the fallback key was needed → caller should
    /// re-encrypt with the primary (per-tenant) key. `PerTenant` is
    /// the steady state.
    pub key_version: KeyVersion,
}

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

/// Decrypt a blob with the primary (per-tenant) key, falling back to
/// the legacy global key on AEAD failure. The lazy-migration entry
/// point: if [`KeyVersion::Legacy`] comes back, the caller is expected
/// to re-encrypt with `primary` and write the new blob back to storage.
///
/// Bech32 / base64 / non-UTF-8 errors are NOT retried with the
/// fallback — those are structural, not key-mismatch, failures.
///
/// # Errors
/// Returns [`ArgoError::Schema`] for malformed input, or
/// [`ArgoError::Provider`] when *both* keys fail to decrypt. The
/// returned error message names the primary failure so operators can
/// tell "wrong key" from "no fallback configured."
pub fn decrypt_value_with_fallback(
    blob_b64: &str,
    primary: &MasterKey,
    legacy: Option<&MasterKey>,
) -> ArgoResult<DecryptedValue> {
    match decrypt_value(blob_b64, primary) {
        Ok(plaintext) => Ok(DecryptedValue {
            plaintext,
            key_version: KeyVersion::PerTenant,
        }),
        Err(ArgoError::Schema(msg)) => Err(ArgoError::Schema(msg)),
        Err(primary_err) => match legacy {
            Some(fallback) => match decrypt_value(blob_b64, fallback) {
                Ok(plaintext) => Ok(DecryptedValue {
                    plaintext,
                    key_version: KeyVersion::Legacy,
                }),
                Err(fallback_err) => Err(ArgoError::Provider(format!(
                    "age decrypt: per-tenant key failed ({primary_err}); \
                     legacy fallback also failed ({fallback_err})"
                ))),
            },
            None => Err(primary_err),
        },
    }
}
