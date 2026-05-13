//! GCP Cloud KMS client for BYOK envelope encryption (Stream
//! `mission-byok-enterprise-tenants`).
//!
//! Speaks the Cloud KMS REST v1 protocol — `:encrypt` and
//! `:decrypt` on the CryptoKey resource. The brief mentioned gRPC
//! as one option; we picked REST because it is the same wire-level
//! contract, drops the prost/tonic transitive footprint, and keeps
//! the wiremock-test loop straightforward.
//!
//! # Auth
//!
//! v1 takes a pre-issued OAuth `Bearer` access token from the
//! operator (typically minted by an `Application Default
//! Credentials` sidecar or Workload Identity). Refreshing the token
//! before expiry is Phase 2; production deployments either use
//! short-lived tokens with sidecar rotation or proxy through a
//! workload-identity-aware ingress.

use std::time::Duration;

use base64::Engine;
use serde::Deserialize;
use serde_json::json;

use crate::kms::{http_error, transport_err, validate_dek_len, KmsProvider};
use crate::StoreResult as Result;
use argo_types::ArgoError;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// GCP Cloud KMS client. The CryptoKey resource is named by
/// `(project, key_ring, key_name)`; we resolve the latest enabled
/// CryptoKeyVersion implicitly (Cloud KMS chooses the primary).
#[derive(Clone)]
pub struct GcpCmekClient {
    /// GCP project id (e.g. `acme-prod-1234`).
    pub project: String,
    /// Cloud KMS key ring name.
    pub key_ring: String,
    /// CryptoKey name inside the key ring.
    pub key_name: String,
    /// KMS location (e.g. `global`, `us-east1`). Defaults to
    /// `global` if the caller leaves the field blank.
    pub location: String,
    /// OAuth access token. Optional for wiremock tests; required
    /// for production calls.
    pub bearer: Option<String>,
    /// Endpoint override — defaults to
    /// `https://cloudkms.googleapis.com`. Tests point this at
    /// wiremock.
    endpoint: String,
    http: reqwest::Client,
}

impl std::fmt::Debug for GcpCmekClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GcpCmekClient")
            .field("project", &"<redacted>")
            .field("key_ring", &"<redacted>")
            .field("key_name", &"<redacted>")
            .field("location", &self.location)
            .field("bearer", &self.bearer.as_ref().map(|_| "<redacted>"))
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

impl GcpCmekClient {
    /// Build a client with the public Cloud KMS endpoint.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] if the reqwest client cannot
    /// be constructed.
    pub fn new(
        project: String,
        key_ring: String,
        key_name: String,
        location: String,
        bearer: Option<String>,
    ) -> Result<Self> {
        Self::build(
            project,
            key_ring,
            key_name,
            location,
            bearer,
            "https://cloudkms.googleapis.com".into(),
        )
    }

    /// Build a client with an explicit endpoint (test override).
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] if the reqwest client cannot
    /// be constructed.
    pub fn with_endpoint(
        project: String,
        key_ring: String,
        key_name: String,
        location: String,
        bearer: Option<String>,
        endpoint: String,
    ) -> Result<Self> {
        Self::build(project, key_ring, key_name, location, bearer, endpoint)
    }

    fn build(
        project: String,
        key_ring: String,
        key_name: String,
        location: String,
        bearer: Option<String>,
        endpoint: String,
    ) -> Result<Self> {
        let location = if location.is_empty() {
            "global".to_string()
        } else {
            location
        };
        let http = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .map_err(|e| ArgoError::Provider(format!("gcp-cmek client build: {e}")))?;
        Ok(Self {
            project,
            key_ring,
            key_name,
            location,
            bearer,
            endpoint,
            http,
        })
    }

    fn key_path(&self) -> String {
        format!(
            "projects/{}/locations/{}/keyRings/{}/cryptoKeys/{}",
            self.project, self.location, self.key_ring, self.key_name
        )
    }

    async fn post(&self, suffix: &str, body: serde_json::Value) -> Result<reqwest::Response> {
        let url = format!("{}/v1/{}:{}", self.endpoint, self.key_path(), suffix);
        let mut req = self.http.post(&url);
        if let Some(bearer) = &self.bearer {
            req = req.header("Authorization", format!("Bearer {bearer}"));
        }
        req.json(&body)
            .send()
            .await
            .map_err(|e| transport_err("gcp", e))
    }
}

#[async_trait::async_trait]
impl KmsProvider for GcpCmekClient {
    async fn unwrap_dek(&self, wrapped: &[u8]) -> Result<Vec<u8>> {
        let body = json!({
            "ciphertext": base64::engine::general_purpose::STANDARD.encode(wrapped),
        });
        let resp = self.post("decrypt", body).await?;
        if !resp.status().is_success() {
            return Err(http_error("gcp", resp).await);
        }
        let parsed: DecryptResponse = resp
            .json()
            .await
            .map_err(|e| ArgoError::Provider(format!("gcp-cmek decrypt parse: {e}")))?;
        let plaintext = base64::engine::general_purpose::STANDARD
            .decode(parsed.plaintext.as_bytes())
            .map_err(|e| ArgoError::Provider(format!("gcp-cmek decrypt base64: {e}")))?;
        validate_dek_len(&plaintext)?;
        Ok(plaintext)
    }

    async fn wrap_dek(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        validate_dek_len(plaintext)?;
        let body = json!({
            "plaintext": base64::engine::general_purpose::STANDARD.encode(plaintext),
        });
        let resp = self.post("encrypt", body).await?;
        if !resp.status().is_success() {
            return Err(http_error("gcp", resp).await);
        }
        let parsed: EncryptResponse = resp
            .json()
            .await
            .map_err(|e| ArgoError::Provider(format!("gcp-cmek encrypt parse: {e}")))?;
        let blob = base64::engine::general_purpose::STANDARD
            .decode(parsed.ciphertext.as_bytes())
            .map_err(|e| ArgoError::Provider(format!("gcp-cmek encrypt base64: {e}")))?;
        Ok(blob)
    }
}

#[derive(Deserialize)]
struct DecryptResponse {
    plaintext: String,
}

#[derive(Deserialize)]
struct EncryptResponse {
    ciphertext: String,
}
