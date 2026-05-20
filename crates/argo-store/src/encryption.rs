//! At-rest encryption key resolution (Stream
//! `mission-encryption-at-rest-sqlcipher`, extended by
//! `mission-byok-enterprise-tenants`).
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
//!
//! # BYOK cloud-KMS variants (mission-byok-enterprise-tenants)
//!
//! Enterprise tenants want the encryption key to live in their own
//! KMS. The cloud-KMS variants ([`KeySource::AwsKms`],
//! [`KeySource::GcpCmek`], [`KeySource::AzureKeyVault`]) implement
//! the *envelope-encryption* pattern: SKYCOT stores a wrapped DEK
//! locally (on disk or in an env var) and asks the cloud KMS to
//! unwrap it at startup. KMS sees only the wrapped DEK, never the
//! plaintext database pages. See [`crate::kms`] for the threat
//! model + rotation story and [`crate::encryption_kms`] for the
//! parse + client-build helpers.
//!
//! Each variant is parsed from the `key_source` config string with
//! its own prefix:
//!
//! - `aws-kms:<arn>?region=<r>&wrapped=<spec>[&bearer=<spec>][&endpoint=<url>]`
//! - `gcp-cmek:<project>/<location>/<ring>/<key>?wrapped=<spec>[&bearer=<spec>][&endpoint=<url>]`
//! - `azure-kv:<vault_url>#<key_name>?wrapped=<spec>[&bearer=<spec>][&endpoint=<url>]`
//!
//! `<spec>` follows the same `env:VAR` / `file:/path` syntax used by
//! the local key sources, so secret material is never inlined into
//! `argo.toml`. The wrapped-DEK pointer is mandatory; the bearer
//! pointer is optional (omit for tests against wiremock).
//!
//! # Module layout
//!
//! Per `codex-loc-budget-followup-wave3` (HARD RULE 6 — parent must
//! stay under 500 LOC):
//! - [`KeySource`] + parse/resolve live in `encryption_key_source.rs`
//! - [`KmsKeyRef`] / [`WrappedDekSource`] live in
//!   `encryption_sources.rs`
//! - This parent file owns the pragma + hex/base64 helpers and
//!   re-exports the moved types so the external path
//!   `argo_store::encryption::*` is unchanged.

use crate::StoreResult as Result;
use argo_types::ArgoError;
use sqlx::{Pool, Sqlite};

#[path = "encryption_key_source.rs"]
mod encryption_key_source;
#[path = "encryption_sources.rs"]
mod encryption_sources;

pub use encryption_key_source::KeySource;
pub use encryption_sources::{KmsKeyRef, WrappedDekSource};

/// Decode a base64-encoded 32-byte secret into the 64-character
/// lowercase hex form that SQLCipher's `PRAGMA key = "x'…'"` literal
/// expects.
///
/// Same canonical wire format as
/// `argo_config::cred::db_key::DB_KEY_ENV` and
/// `argo_config::cred::db_key::get_or_create_db_key` — the gateway
/// boot path resolves a per-tenant `SecretString` from the keyring,
/// exposes the inner base64 string, and feeds it through this helper
/// before calling [`crate::SessionStore::open_with_hex_key`].
///
/// # Errors
/// Returns [`ArgoError::Schema`] when the input is not valid base64,
/// decodes to a length other than 32 bytes, or contains non-hex
/// characters after re-encoding.
///
/// ```
/// use argo_store::encryption::base64_to_hex;
/// // 32 zero bytes → 64 zero hex chars.
/// let zero_b64 = base64::Engine::encode(
///     &base64::engine::general_purpose::STANDARD, [0u8; 32],
/// );
/// let hex = base64_to_hex(&zero_b64).unwrap();
/// assert_eq!(hex, "0".repeat(64));
/// ```
pub fn base64_to_hex(base64_secret: &str) -> Result<String> {
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    const DB_KEY_RAW_LEN: usize = 32;
    let bytes = B64
        .decode(base64_secret.trim())
        .map_err(|e| ArgoError::Schema(format!("db key not valid base64: {e}")))?;
    if bytes.len() != DB_KEY_RAW_LEN {
        return Err(ArgoError::Schema(format!(
            "db key must decode to {DB_KEY_RAW_LEN} bytes (got {})",
            bytes.len()
        )));
    }
    let mut out = String::with_capacity(DB_KEY_RAW_LEN * 2);
    for b in &bytes {
        out.push_str(&format!("{b:02x}"));
    }
    Ok(out)
}

/// Validate that `s` is exactly 64 ASCII hex characters and return
/// the lowercase canonical form. Used as the gatekeeper before any
/// `PRAGMA key = "x'…'"` interpolation so a caller cannot smuggle a
/// non-hex byte through into the literal.
///
/// # Errors
/// Returns [`ArgoError::Schema`] when the length is wrong or any
/// character is outside `[0-9a-fA-F]`.
pub fn validate_hex_key(s: &str) -> Result<String> {
    if s.len() != 64 {
        return Err(ArgoError::Schema(format!(
            "encryption key must be 64 hex characters (got {})",
            s.len()
        )));
    }
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ArgoError::Schema(
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
    let hex_key = validate_hex_key(hex_key)?;
    // audit-allow: validated SQLCipher PRAGMA literal
    let stmt = format!("PRAGMA key = \"x'{hex_key}'\"");
    sqlx::query(&stmt)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("apply PRAGMA key: {e}")))?;
    Ok(())
}

/// Whether this binary was compiled with the `sqlcipher` Cargo
/// feature — the operator-facing toggle that signals SQLCipher's
/// `PRAGMA key` is load-bearing on the linked SQLite driver. Stock
/// builds return `false`; enterprise builds (`--features
/// bundled-sqlcipher`, which implies `sqlcipher`) return `true`.
///
/// Single source of truth for the compile-time flag — callers asking
/// "is this a SQLCipher-linked binary?" go through this helper rather
/// than re-spelling `cfg!(feature = "sqlcipher")`.
///
/// # Examples
///
/// ```
/// use argo_store::encryption::sqlcipher_linked;
/// assert_eq!(sqlcipher_linked(), cfg!(feature = "sqlcipher"));
/// ```
#[must_use]
pub const fn sqlcipher_linked() -> bool {
    cfg!(feature = "sqlcipher")
}

/// Rotate the encryption key in place via `PRAGMA rekey`. Caller is
/// responsible for having already applied the old key (via
/// [`apply_encryption_pragma`]) before invoking this.
///
/// # Errors
/// Returns [`ArgoError::Provider`] if SQLite rejects the rekey.
pub async fn apply_rekey_pragma(pool: &Pool<Sqlite>, new_hex_key: &str) -> Result<()> {
    let new_hex_key = validate_hex_key(new_hex_key)?;
    // audit-allow: validated SQLCipher PRAGMA literal
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
