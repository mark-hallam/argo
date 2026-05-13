//! Cloud KMS wrap/unwrap for BYOK at-rest encryption (Stream
//! `mission-byok-enterprise-tenants`).
//!
//! # DEK envelope encryption (threat model)
//!
//! Enterprise tenants want their cloud-provider Key Management
//! Service (KMS) to own the root of trust for SKYCOT's at-rest
//! encryption, not a key the operator pastes into a config file.
//! The pattern is *envelope encryption*:
//!
//! 1. Operator generates a single 256-bit *data encryption key* (DEK)
//!    locally. This DEK is what SQLCipher's `PRAGMA key` consumes.
//! 2. Operator wraps the DEK with their KMS key (`kms:Encrypt`) and
//!    stores the *wrapped DEK* — a ciphertext blob — on disk or in
//!    an env var.
//! 3. At startup, SKYCOT reads the wrapped DEK and asks KMS to
//!    `Decrypt` it. KMS sees only the wrapped DEK, never the
//!    plaintext database pages.
//! 4. The unwrapped DEK feeds `PRAGMA key`. The plaintext DEK never
//!    leaves the SKYCOT process address space and is not logged.
//!
//! A stolen database file without KMS access is unreadable: the
//! attacker has SQLCipher-encrypted pages plus a KMS-encrypted DEK,
//! and neither helps without an IAM-authorized round-trip to the
//! cloud KMS. This is the BYOK procurement story.
//!
//! # Rotation
//!
//! Rotation re-wraps the *same* DEK under a new KMS key. The DEK
//! is unchanged, so SQLCipher data pages do not need re-encryption.
//! Rotation is therefore O(1) regardless of database size — the
//! "fast rotate" property enterprise procurement teams care about.
//! See [`rewrap_dek`] for the primitive.
//!
//! # v1 scope
//!
//! - 3 providers behind separate Cargo features (`kms-aws`,
//!   `kms-gcp`, `kms-azure`); default builds pull none.
//! - All three speak HTTP/JSON to their KMS endpoints; no heavy
//!   cloud SDKs are pulled.
//! - Auth is operator-supplied via a bearer token (AWS session
//!   credentials, GCP OAuth access token, Azure AAD token). SigV4
//!   bundling and OAuth token refresh are a Phase 2 hardening; the
//!   v1 wire is testable end-to-end with wiremock today.
//! - HSM-direct integration, per-row encryption, and HashiCorp
//!   Vault are out of scope per the brief.

#![cfg(feature = "kms")]

use argo_types::ArgoError;

use crate::StoreResult as Result;

#[cfg(feature = "kms-aws")]
pub mod aws;
#[cfg(feature = "kms-azure")]
pub mod azure;
#[cfg(feature = "kms-gcp")]
pub mod gcp;

/// One cloud KMS unwrap/wrap pair. Implementations live in
/// per-provider sibling modules; the trait gives [`KeySource`] a
/// single dispatch surface regardless of which cloud is configured.
///
/// [`KeySource`]: crate::encryption::KeySource
#[async_trait::async_trait]
pub trait KmsProvider: Send + Sync {
    /// Decrypt (unwrap) a wrapped DEK ciphertext blob and return the
    /// plaintext DEK bytes. The DEK is 32 bytes (AES-256).
    async fn unwrap_dek(&self, wrapped: &[u8]) -> Result<Vec<u8>>;

    /// Encrypt (wrap) a plaintext DEK under this KMS key and return
    /// the wrapped DEK ciphertext blob.
    async fn wrap_dek(&self, plaintext: &[u8]) -> Result<Vec<u8>>;
}

/// Helper used by every provider: turn a non-2xx response (or a
/// transport error) into an actionable [`ArgoError::Provider`].
/// Names the provider and the HTTP status so operators can map the
/// error to an IAM policy or network issue without source-diving.
pub(crate) fn provider_err(provider: &str, status: u16, body: &str) -> ArgoError {
    let snippet: String = body.chars().take(240).collect();
    ArgoError::Provider(format!(
        "{provider} KMS call failed (HTTP {status}): {snippet}"
    ))
}

/// Map a reqwest transport error to a typed [`ArgoError::Provider`].
pub(crate) fn transport_err(provider: &str, e: reqwest::Error) -> ArgoError {
    ArgoError::Provider(format!("{provider} KMS network error: {e}"))
}

/// Re-wrap the DEK from `old` to `new`. Decrypts under `old`,
/// re-encrypts under `new`, returns the new wrapped-DEK bytes.
/// The plaintext DEK never leaves this function's stack frame.
///
/// # Errors
/// Returns [`ArgoError::Provider`] if either provider's unwrap/wrap
/// call fails.
pub async fn rewrap_dek(
    old: &dyn KmsProvider,
    new: &dyn KmsProvider,
    wrapped: &[u8],
) -> Result<Vec<u8>> {
    let plaintext = old.unwrap_dek(wrapped).await?;
    new.wrap_dek(&plaintext).await
}

/// 32-byte DEK requirement check shared across providers.
pub(crate) fn validate_dek_len(plaintext: &[u8]) -> Result<()> {
    if plaintext.len() != 32 {
        return Err(ArgoError::Provider(format!(
            "KMS-unwrapped DEK must be 32 bytes (AES-256); got {}",
            plaintext.len()
        )));
    }
    Ok(())
}

/// Shared HTTP error helper: read body (best-effort) and wrap.
pub(crate) async fn http_error(provider: &str, resp: reqwest::Response) -> ArgoError {
    let status = resp.status().as_u16();
    let body = resp.text().await.unwrap_or_default();
    provider_err(provider, status, &body)
}
