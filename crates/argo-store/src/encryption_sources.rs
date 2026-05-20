//! Cloud-KMS source pointers used by the BYOK variants of
//! [`crate::encryption::KeySource`]. Extracted from
//! `encryption.rs` so the parent stays under the 500-LOC ceiling
//! (HARD RULE 6, brief: `codex-loc-budget-followup-wave3`).
//!
//! These types are re-exported from `crate::encryption` so the
//! external path stays stable.

use crate::StoreResult as Result;
use argo_types::ArgoError;
use std::env;
use std::fmt;
use std::fs;
use std::path::PathBuf;

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
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_store::encryption::WrappedDekSource;
    /// let env = WrappedDekSource::parse("env:ARGO_WRAPPED_DEK").unwrap();
    /// assert!(matches!(env, WrappedDekSource::Env(_)));
    /// // Unknown prefix is rejected.
    /// assert!(WrappedDekSource::parse("http://example.com/dek").is_err());
    /// ```
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
                let raw = env::var(var)
                    .map_err(|e| ArgoError::Provider(format!("wrapped-dek env '{var}': {e}")))?;
                Ok(raw.trim().as_bytes().to_vec())
            }
            Self::File(path) => fs::read(path)
                .map_err(|e| {
                    ArgoError::Provider(format!("wrapped-dek file '{}': {e}", path.display()))
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
