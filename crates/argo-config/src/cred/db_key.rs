//! Per-tenant database encryption-key resolution backed by the OS keyring.
//!
//! Supplies the 32-byte symmetric key used to derive SQLCipher's
//! `PRAGMA key` in PR 4 of the `mission-sqlcipher-*` sequence. Only the
//! API lands here; the wire-in point at [`crate::store::SessionStore`]
//! is intentionally untouched (see brief PR 4).
//!
//! ## Resolution precedence
//!
//! 1. `SKYCOT_DB_KEY_BASE64` env var (Fly ephemeral / CI). Value MUST
//!    decode to exactly 32 bytes; malformed input is a hard error, not
//!    a silent fallback, so misconfigured deploys fail closed.
//! 2. OS keyring under service `skycot.db.encryption`, account
//!    `tenant-<uuid>`. macOS routes through Keychain; Linux through the
//!    Secret Service D-Bus API; Windows is out of scope for v1 per the
//!    brief.
//! 3. Generate-and-persist: a fresh 32-byte secret from `OsRng`, base64-
//!    encoded, written back to the keyring so the next call retrieves
//!    the same key.
//!
//! ## Per-tenant isolation
//!
//! Each tenant gets a distinct keyring entry. Multi-tenant deployments
//! never share an encryption key across tenants — that would be a
//! cross-tenant data-confidentiality regression.
//!
//! ## Test backends
//!
//! Real-Keychain access prompts for the user password on macOS, which
//! would break CI. Tests install a process-wide in-memory credential
//! builder via `keyring::set_default_credential_builder` (the same
//! pattern used by [`crate::cred_keyring`] tests) so unit tests never
//! touch the platform store.

use argo_types::{ArgoError, ArgoResult, TenantId};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rand_core::{OsRng, RngCore};
use secrecy::SecretString;

/// Env var that overrides the keyring lookup. Value is a base64-encoded
/// 32-byte secret. Intended for headless deploys (Fly) and CI runners
/// where no OS keyring is available.
pub const DB_KEY_ENV: &str = "SKYCOT_DB_KEY_BASE64";

/// Keyring service name under which per-tenant DB encryption keys are
/// stored. Stable wire identifier — operators can inspect entries via
/// `security find-generic-password -s skycot.db.encryption` on macOS.
pub const KEYRING_SERVICE: &str = "skycot.db.encryption";

/// Required size of the raw DB encryption key in bytes (32 bytes →
/// 256-bit, matching SQLCipher's default key size).
pub const DB_KEY_LEN: usize = 32;

/// Compute the keyring account name for a tenant. Stable wire format;
/// the operator's `security find-generic-password -a tenant-<uuid>`
/// invocation depends on it.
#[must_use]
pub fn account_for(tenant_id: TenantId) -> String {
    format!("tenant-{tenant_id}")
}

/// Retrieve the per-tenant SQLCipher encryption key, creating and
/// persisting a fresh 32-byte secret on first call.
///
/// Returned [`SecretString`] holds the *base64-encoded* key text — that
/// is the canonical wire format both for env-var input and for keyring
/// storage. Callers that need raw bytes call [`decode_secret`].
///
/// The function is marked `async` for forward-compatibility with PR 4's
/// wire-in at `SessionStore::open` (which is itself async). The current
/// implementation does synchronous keyring I/O; PR 4 may switch to
/// `tokio::task::spawn_blocking` if the D-Bus call becomes a measurable
/// blocker on Linux.
///
/// # Errors
///
/// - [`ArgoError::Schema`] when `SKYCOT_DB_KEY_BASE64` is set but not a
///   valid base64-encoded 32-byte payload.
/// - [`ArgoError::Provider`] when the keyring backend rejects the read
///   or write, or when a previously-stored entry has been tampered with
///   (wrong length, non-base64 payload).
///
/// ```no_run
/// use argo_config::cred::db_key::get_or_create_db_key;
/// use argo_types::TenantId;
/// # async fn run() -> argo_types::ArgoResult<()> {
/// let tenant = TenantId::new();
/// let _key = get_or_create_db_key(tenant).await?;
/// # Ok(()) }
/// ```
pub async fn get_or_create_db_key(tenant_id: TenantId) -> ArgoResult<SecretString> {
    get_or_create_db_key_sync(tenant_id)
}

/// Synchronous core of [`get_or_create_db_key`]. Tests exercise this
/// directly to avoid pulling tokio into `argo-config`'s dev-deps.
pub(crate) fn get_or_create_db_key_sync(tenant_id: TenantId) -> ArgoResult<SecretString> {
    if let Some(secret) = env_override()? {
        return Ok(secret);
    }
    let account = account_for(tenant_id);
    if let Some(existing) = keyring_load(&account)? {
        return Ok(existing);
    }
    keyring_generate_and_store(&account)
}

/// Decode the base64 payload from a [`SecretString`] returned by
/// [`get_or_create_db_key`] into the underlying 32 raw bytes. The
/// caller is responsible for zeroing the buffer once consumed.
///
/// # Errors
///
/// Returns [`ArgoError::Schema`] when the inner string is not valid
/// base64 or decodes to a length other than [`DB_KEY_LEN`].
pub fn decode_secret(secret: &SecretString) -> ArgoResult<[u8; DB_KEY_LEN]> {
    use secrecy::ExposeSecret;
    decode_base64_key(secret.expose_secret())
}

fn env_override() -> ArgoResult<Option<SecretString>> {
    let raw = match std::env::var(DB_KEY_ENV) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(ArgoError::Schema(format!(
                "{DB_KEY_ENV} is set but not valid UTF-8"
            )));
        }
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let _ = decode_base64_key(trimmed)?;
    Ok(Some(SecretString::new(Box::from(trimmed))))
}

fn keyring_load(account: &str) -> ArgoResult<Option<SecretString>> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, account)
        .map_err(|e| ArgoError::Provider(format!("keyring entry construct: {e}")))?;
    match entry.get_password() {
        Ok(value) => {
            let _ = decode_base64_key(&value).map_err(|e| {
                ArgoError::Provider(format!(
                    "keyring entry for {account} is corrupted: {e}; \
                     operator must delete and re-init"
                ))
            })?;
            Ok(Some(SecretString::new(Box::from(value.as_str()))))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(ArgoError::Provider(format!(
            "keyring read {account}: {e}"
        ))),
    }
}

fn keyring_generate_and_store(account: &str) -> ArgoResult<SecretString> {
    let mut bytes = [0u8; DB_KEY_LEN];
    OsRng.fill_bytes(&mut bytes);
    let encoded = B64.encode(bytes);
    let entry = keyring::Entry::new(KEYRING_SERVICE, account)
        .map_err(|e| ArgoError::Provider(format!("keyring entry construct: {e}")))?;
    entry
        .set_password(&encoded)
        .map_err(|e| ArgoError::Provider(format!("keyring write {account}: {e}")))?;
    Ok(SecretString::new(Box::from(encoded.as_str())))
}

fn decode_base64_key(value: &str) -> ArgoResult<[u8; DB_KEY_LEN]> {
    let raw = B64
        .decode(value.trim())
        .map_err(|e| ArgoError::Schema(format!("db key not valid base64: {e}")))?;
    let len = raw.len();
    if len != DB_KEY_LEN {
        return Err(ArgoError::Schema(format!(
            "db key wrong length: got {len} bytes, want {DB_KEY_LEN}"
        )));
    }
    let mut out = [0u8; DB_KEY_LEN];
    out.copy_from_slice(&raw);
    Ok(out)
}

#[cfg(test)]
#[path = "db_key_tests.rs"]
mod db_key_tests;
