//! [`KeySource`] — the parsed `[security.encryption] key_source`
//! config value. Extracted from `encryption.rs` so the parent stays
//! under the 500-LOC ceiling (HARD RULE 6, brief:
//! `codex-loc-budget-followup-wave3`). Re-exported from
//! `crate::encryption` so the external path stays stable.

use crate::encryption::{validate_hex_key, KmsKeyRef};
use crate::StoreResult as Result;
use argo_types::ArgoError;
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
///
/// # Examples
///
/// ```
/// use argo_store::encryption::KeySource;
/// // Empty / "none" parses to KeySource::None (the default).
/// assert_eq!(KeySource::parse("").unwrap(), KeySource::None);
/// assert_eq!(KeySource::parse("none").unwrap(), KeySource::None);
/// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_store::encryption::KeySource;
    /// // The local-source variants round-trip the env / file pointer.
    /// let env = KeySource::parse("env:ARGO_DB_ENCRYPTION_KEY").unwrap();
    /// assert!(matches!(env, KeySource::Env(_)));
    /// // Empty `env:` / `file:` payload is rejected.
    /// assert!(KeySource::parse("env:").is_err());
    /// ```
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
                let raw = env::var(var)
                    .map_err(|e| ArgoError::Provider(format!("encryption key env '{var}': {e}")))?;
                Ok(Some(validate_hex_key(raw.trim())?))
            }
            Self::File(path) => {
                let raw = fs::read_to_string(path).map_err(|e| {
                    ArgoError::Provider(format!("encryption key file '{}': {e}", path.display()))
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
