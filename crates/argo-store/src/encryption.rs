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
///
/// Cloud-KMS variants store the *pointer to* the wrapped DEK +
/// the KMS key reference. Resolving these variants performs a
/// network call to the KMS to unwrap the DEK; the raw key bytes are
/// only ever held briefly in memory.
#[derive(Clone, Default, PartialEq, Eq)]
pub enum KeySource {
    /// Encryption disabled (plaintext store).
    #[default]
    None,
    /// Read raw hex key from the named environment variable.
    Env(String),
    /// Read raw hex key from the named file (operator-managed).
    File(PathBuf),
    /// AWS KMS — unwrap DEK via `kms:Decrypt` (BYOK).
    AwsKms(KmsKeyRef),
    /// GCP Cloud KMS — unwrap DEK via Cloud KMS `:decrypt` (BYOK).
    GcpCmek(KmsKeyRef),
    /// Azure Key Vault — unwrap DEK via `/unwrapkey` (BYOK).
    AzureKeyVault(KmsKeyRef),
}

/// Cloud KMS reference shared by the three BYOK variants. The fields
/// are stored as opaque strings to keep the enum-shape uniform across
/// providers — each provider client interprets the fields as it sees
/// fit. See [`crate::kms`] for per-provider semantics.
#[derive(Clone, PartialEq, Eq)]
pub struct KmsKeyRef {
    /// Provider-specific key identifier (AWS ARN, GCP key path
    /// `project/location/ring/key`, Azure `<vault_url>#<key>`).
    pub key_id: String,
    /// Provider-specific region / location.
    pub region: String,
    /// Where the wrapped DEK is stored.
    pub wrapped_dek: WrappedDekSource,
    /// Optional bearer-token source.
    pub bearer: Option<WrappedDekSource>,
    /// Endpoint override — empty ⇒ provider default; tests set this
    /// to the wiremock URL.
    pub endpoint_override: String,
}

impl fmt::Debug for KmsKeyRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KmsKeyRef")
            .field("key_id", &"<redacted>")
            .field("region", &self.region)
            .field("wrapped_dek", &self.wrapped_dek)
            .field("bearer", &self.bearer.as_ref().map(|_| "<redacted>"))
            .field("endpoint_override", &self.endpoint_override)
            .finish()
    }
}

/// Where bytes (wrapped DEK / bearer token) come from. Mirrors the
/// local-key vocabulary so operators get one consistent mental model.
#[derive(Clone, PartialEq, Eq)]
pub enum WrappedDekSource {
    /// Read from the named environment variable.
    Env(String),
    /// Read from the named file path.
    File(PathBuf),
}

impl fmt::Debug for WrappedDekSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Env(_) => write!(f, "WrappedDekSource::Env(<redacted>)"),
            Self::File(_) => write!(f, "WrappedDekSource::File(<redacted>)"),
        }
    }
}

impl WrappedDekSource {
    /// Parse `env:VAR` or `file:/path`.
    ///
    /// # Errors
    /// Returns [`ArgoError::Schema`] if the prefix is unknown or
    /// the value is empty.
    pub fn parse(spec: &str) -> Result<Self> {
        let trimmed = spec.trim();
        if let Some(var) = trimmed.strip_prefix("env:") {
            let var = var.trim();
            if var.is_empty() {
                return Err(ArgoError::Schema(
                    "wrapped-dek/bearer 'env:' prefix needs a variable name".into(),
                ));
            }
            return Ok(Self::Env(var.to_string()));
        }
        if let Some(path) = trimmed.strip_prefix("file:") {
            let path = path.trim();
            if path.is_empty() {
                return Err(ArgoError::Schema(
                    "wrapped-dek/bearer 'file:' prefix needs a path".into(),
                ));
            }
            return Ok(Self::File(PathBuf::from(path)));
        }
        Err(ArgoError::Schema(format!(
            "invalid wrapped-dek/bearer source '{trimmed}': expected 'env:VAR' or 'file:/path'"
        )))
    }

    /// Resolve the bytes (binary).
    ///
    /// # Errors
    /// - env var missing or unreadable
    /// - file missing or unreadable
    pub fn resolve_bytes(&self) -> Result<Vec<u8>> {
        match self {
            Self::Env(var) => {
                let raw = env::var(var).map_err(|e| {
                    ArgoError::Provider(format!("wrapped-dek env '{var}': {e}"))
                })?;
                Ok(raw.trim().as_bytes().to_vec())
            }
            Self::File(path) => fs::read(path)
                .map_err(|e| {
                    ArgoError::Provider(format!(
                        "wrapped-dek file '{}': {e}",
                        path.display()
                    ))
                })
                .map(trim_trailing_newlines),
        }
    }

    /// Resolve as a UTF-8 string (used for bearer tokens).
    ///
    /// # Errors
    /// As [`resolve_bytes`] plus non-UTF-8 contents.
    pub fn resolve_string(&self) -> Result<String> {
        let bytes = self.resolve_bytes()?;
        String::from_utf8(bytes)
            .map_err(|e| ArgoError::Provider(format!("bearer source: non-UTF8 ({e})")))
    }
}

fn trim_trailing_newlines(mut v: Vec<u8>) -> Vec<u8> {
    while matches!(v.last(), Some(b'\n' | b'\r')) {
        v.pop();
    }
    v
}

impl KeySource {
    /// Parse the `[security.encryption] key_source` config string.
    ///
    /// Accepted forms:
    /// - `"none"` (or empty) → [`KeySource::None`]
    /// - `"env:VAR_NAME"` → [`KeySource::Env`]
    /// - `"file:/path/to/key"` → [`KeySource::File`]
    /// - `"aws-kms:<arn>?<params>"` → [`KeySource::AwsKms`]
    /// - `"gcp-cmek:<proj>/<loc>/<ring>/<key>?<params>"` → [`KeySource::GcpCmek`]
    /// - `"azure-kv:<vault>#<key>?<params>"` → [`KeySource::AzureKeyVault`]
    ///
    /// `<params>` is `key=value&key=value...`. Recognized keys:
    /// `region`, `wrapped` (required for KMS variants), `bearer`,
    /// `endpoint`. Both `wrapped` and `bearer` accept the
    /// `env:VAR` / `file:/path` vocabulary.
    ///
    /// # Errors
    /// Returns [`ArgoError::Schema`] when the prefix is unknown or
    /// the value after the prefix is empty.
    pub fn parse(spec: &str) -> Result<Self> {
        let trimmed = spec.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
            return Ok(Self::None);
        }
        if let Some(var) = trimmed.strip_prefix("env:") {
            let var = var.trim();
            if var.is_empty() {
                return Err(ArgoError::Schema(
                    "key_source 'env:' prefix needs a variable name".into(),
                ));
            }
            return Ok(Self::Env(var.to_string()));
        }
        if let Some(path) = trimmed.strip_prefix("file:") {
            let path = path.trim();
            if path.is_empty() {
                return Err(ArgoError::Schema(
                    "key_source 'file:' prefix needs a path".into(),
                ));
            }
            return Ok(Self::File(PathBuf::from(path)));
        }
        if let Some(body) = trimmed.strip_prefix("aws-kms:") {
            return crate::encryption_kms::parse_kms_spec(body, "aws-kms").map(Self::AwsKms);
        }
        if let Some(body) = trimmed.strip_prefix("gcp-cmek:") {
            return crate::encryption_kms::parse_kms_spec(body, "gcp-cmek").map(Self::GcpCmek);
        }
        if let Some(body) = trimmed.strip_prefix("azure-kv:") {
            return crate::encryption_kms::parse_kms_spec(body, "azure-kv")
                .map(Self::AzureKeyVault);
        }
        Err(ArgoError::Schema(format!(
            "invalid key_source '{trimmed}': expected 'none', 'env:VAR', 'file:/path', \
             or 'aws-kms:'/'gcp-cmek:'/'azure-kv:' prefix"
        )))
    }

    /// Resolve the key bytes (raw hex, lowercased, validated).
    ///
    /// Returns `Ok(None)` for [`KeySource::None`]. For env / file
    /// sources, reads the value, strips whitespace, validates that it
    /// is 64 hex characters (= 32 bytes = AES-256), and returns the
    /// canonical lowercased hex string.
    ///
    /// Cloud-KMS variants always error here — they need a network
    /// round-trip and the sync surface intentionally rejects them so
    /// existing callers that pre-date BYOK don't accidentally block
    /// on an async KMS call. Call [`resolve_async`] instead.
    ///
    /// # Errors
    /// - env var missing or unreadable
    /// - file missing, unreadable, or zero-length
    /// - value is not 64 hex characters
    /// - cloud-KMS variant (use [`resolve_async`])
    ///
    /// [`resolve_async`]: KeySource::resolve_async
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
            Self::AwsKms(_) | Self::GcpCmek(_) | Self::AzureKeyVault(_) => {
                Err(ArgoError::Provider(
                    "KMS key sources require async resolution — \
                     call `KeySource::resolve_async()` instead. \
                     The end-to-end store-open wiring lands in the \
                     BYOK follow-up brief; the v1 primitive surface \
                     is `resolve_async()` + `crate::kms::rewrap_dek()`."
                        .into(),
                ))
            }
        }
    }

    /// Async variant of [`resolve`] that handles the cloud-KMS
    /// variants by calling out to the configured KMS to unwrap the
    /// DEK. For [`KeySource::None`], [`KeySource::Env`], and
    /// [`KeySource::File`] this delegates to [`resolve`] so callers
    /// get one consistent API.
    ///
    /// # Errors
    /// As [`resolve`] for local variants; plus any KMS HTTP /
    /// IAM-denied error for cloud variants.
    ///
    /// [`resolve`]: KeySource::resolve
    #[cfg(feature = "kms")]
    pub async fn resolve_async(&self) -> Result<Option<String>> {
        match self {
            Self::None | Self::Env(_) | Self::File(_) => self.resolve(),
            Self::AwsKms(r) | Self::GcpCmek(r) | Self::AzureKeyVault(r) => {
                crate::encryption_kms::resolve_kms(self, r).await
            }
        }
    }
}

/// Redacted Debug — never leaks the variable name, key value, KMS
/// key id, or bearer token into tracing spans or panic messages.
impl fmt::Debug for KeySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "KeySource::None"),
            Self::Env(_) => write!(f, "KeySource::Env(<redacted>)"),
            Self::File(_) => write!(f, "KeySource::File(<redacted>)"),
            Self::AwsKms(_) => write!(f, "KeySource::AwsKms(<redacted>)"),
            Self::GcpCmek(_) => write!(f, "KeySource::GcpCmek(<redacted>)"),
            Self::AzureKeyVault(_) => write!(f, "KeySource::AzureKeyVault(<redacted>)"),
        }
    }
}

fn validate_hex_key(s: &str) -> Result<String> {
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
