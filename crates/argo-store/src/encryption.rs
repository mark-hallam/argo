//! At-rest encryption key resolution (Stream
//! `mission-encryption-at-rest-sqlcipher`).
//!
//! Defines the [`KeySource`] enum that backs the
//! `[security.encryption] key_source` config field, plus the
//! [`apply_encryption_pragma`] helper that issues `PRAGMA key` as the
//! first statement on a fresh connection pool.
//!
//! # SQLCipher integration
//!
//! `argo-store` ships with the pure-Rust `sqlx` SQLite driver, which
//! is not linked against SQLCipher. Stock builds therefore silently
//! ignore `PRAGMA key` and write plaintext pages — the helper is the
//! `wire` (per the brief title) that becomes load-bearing once an
//! operator swaps in a SQLCipher-linked SQLite driver. Enabling the
//! `sqlcipher` Cargo feature is the operator-facing toggle that
//! downstream builds (FIPS / SQLCipher Commercial) flip; it does not
//! by itself change the linked driver.

use crate::StoreResult as Result;
use argo_types::ArgoError;
use sqlx::{Pool, Sqlite};
use std::env;
use std::fmt;
use std::fs;
use std::path::PathBuf;

/// The configured source for the at-rest encryption key. Default is
/// [`KeySource::None`] — no `PRAGMA key` is issued and the database
/// is plaintext (current behaviour, backward compatible).
#[derive(Clone, Default, PartialEq, Eq)]
pub enum KeySource {
    /// Encryption disabled (plaintext store).
    #[default]
    None,
    /// Read raw hex key from the named environment variable.
    Env(String),
    /// Read raw hex key from the named file (operator-managed).
    File(PathBuf),
}

impl KeySource {
    /// Parse the `[security.encryption] key_source` config string.
    ///
    /// Accepted forms:
    /// - `"none"` (or empty) → [`KeySource::None`]
    /// - `"env:VAR_NAME"` → [`KeySource::Env`]
    /// - `"file:/path/to/key"` → [`KeySource::File`]
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] when the prefix is unknown or
    /// the value after the prefix is empty.
    pub fn parse(spec: &str) -> Result<Self> {
        let trimmed = spec.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
            return Ok(Self::None);
        }
        if let Some(var) = trimmed.strip_prefix("env:") {
            let var = var.trim();
            if var.is_empty() {
                return Err(ArgoError::Provider(
                    "key_source 'env:' prefix needs a variable name".into(),
                ));
            }
            return Ok(Self::Env(var.to_string()));
        }
        if let Some(path) = trimmed.strip_prefix("file:") {
            let path = path.trim();
            if path.is_empty() {
                return Err(ArgoError::Provider(
                    "key_source 'file:' prefix needs a path".into(),
                ));
            }
            return Ok(Self::File(PathBuf::from(path)));
        }
        Err(ArgoError::Provider(format!(
            "invalid key_source '{trimmed}': expected 'none', 'env:VAR', or 'file:/path'"
        )))
    }

    /// Resolve the key bytes (raw hex, lowercased, validated).
    ///
    /// Returns `Ok(None)` for [`KeySource::None`]. For env / file
    /// sources, reads the value, strips whitespace, validates that it
    /// is 64 hex characters (= 32 bytes = AES-256), and returns the
    /// canonical lowercased hex string.
    ///
    /// # Errors
    /// - env var missing or unreadable
    /// - file missing, unreadable, or zero-length
    /// - value is not 64 hex characters
    pub fn resolve(&self) -> Result<Option<String>> {
        match self {
            Self::None => Ok(None),
            Self::Env(var) => {
                let raw = env::var(var).map_err(|e| {
                    ArgoError::Provider(format!("encryption key env '{var}': {e}"))
                })?;
                Ok(Some(validate_hex_key(raw.trim())?))
            }
            Self::File(path) => {
                let raw = fs::read_to_string(path).map_err(|e| {
                    ArgoError::Provider(format!(
                        "encryption key file '{}': {e}",
                        path.display()
                    ))
                })?;
                Ok(Some(validate_hex_key(raw.trim())?))
            }
        }
    }
}

/// Redacted Debug — never leaks the variable name or key value
/// into tracing spans or panic messages.
impl fmt::Debug for KeySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "KeySource::None"),
            Self::Env(_) => write!(f, "KeySource::Env(<redacted>)"),
            Self::File(_) => write!(f, "KeySource::File(<redacted>)"),
        }
    }
}

fn validate_hex_key(s: &str) -> Result<String> {
    if s.len() != 64 {
        return Err(ArgoError::Provider(format!(
            "encryption key must be 64 hex characters (got {})",
            s.len()
        )));
    }
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ArgoError::Provider(
            "encryption key contains non-hex characters".into(),
        ));
    }
    Ok(s.to_ascii_lowercase())
}

/// Issue `PRAGMA key = "x'…'"` against every connection in `pool`.
/// Idempotent and safe to call on a stock (non-SQLCipher) driver —
/// the PRAGMA is silently ignored, so encryption is a no-op until the
/// driver is swapped. With a SQLCipher-linked driver this is the
/// required first-statement-after-open.
///
/// Pass an already-validated hex key (e.g. via
/// [`KeySource::resolve`]).
///
/// # Errors
/// Returns [`ArgoError::Provider`] if SQLite rejects the PRAGMA.
pub async fn apply_encryption_pragma(pool: &Pool<Sqlite>, hex_key: &str) -> Result<()> {
    let stmt = format!("PRAGMA key = \"x'{hex_key}'\"");
    sqlx::query(&stmt)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("apply PRAGMA key: {e}")))?;
    Ok(())
}

/// Rotate the encryption key in place via `PRAGMA rekey`. Caller is
/// responsible for having already applied the old key (via
/// [`apply_encryption_pragma`]) before invoking this.
///
/// # Errors
/// Returns [`ArgoError::Provider`] if SQLite rejects the rekey.
pub async fn apply_rekey_pragma(pool: &Pool<Sqlite>, new_hex_key: &str) -> Result<()> {
    let stmt = format!("PRAGMA rekey = \"x'{new_hex_key}'\"");
    sqlx::query(&stmt)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("apply PRAGMA rekey: {e}")))?;
    Ok(())
}

#[cfg(test)]
#[path = "encryption_tests.rs"]
mod tests;
