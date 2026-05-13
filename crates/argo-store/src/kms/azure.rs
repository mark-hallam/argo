//! Azure Key Vault client for BYOK envelope encryption (Stream
//! `mission-byok-enterprise-tenants`).
//!
//! Speaks Key Vault REST API 7.4 (`/keys/{name}/{version}/encrypt`
//! and `/decrypt`). We pick REST over the `azure_security_keyvault`
//! crate for the same reason as the AWS/GCP variants — minimal
//! transitive footprint and wiremock-friendly test loop.
//!
//! # Auth
//!
//! v1 takes a pre-issued Azure Active Directory access token as a
//! bearer. Operators typically mint these via Workload Identity or
//! Managed Identity sidecars; on-the-fly AAD token acquisition is
//! Phase 2.
//!
//! # Wrap algorithm
//!
//! The brief leaves the wrap algorithm operator-choice. v1 defaults
//! to `RSA-OAEP-256` (asymmetric vault keys); operators may swap
//! via [`AzureKeyVaultClient::with_algorithm`]. RSA-OAEP-256 is the
//! Azure-recommended default for wrap operations.

use std::time::Duration;

use base64::Engine;
use serde::Deserialize;
use serde_json::json;

use crate::kms::{http_error, transport_err, validate_dek_len, KmsProvider};
use crate::StoreResult as Result;
use argo_types::ArgoError;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_ALGORITHM: &str = "RSA-OAEP-256";
const API_VERSION: &str = "7.4";

/// Azure Key Vault client. The wrap/unwrap key is identified by
/// the vault URL + key name; the latest enabled version is used
/// implicitly when no version is provided.
#[derive(Clone)]
pub struct AzureKeyVaultClient {
    /// Vault URL (e.g. `https://my-vault.vault.azure.net`).
    pub vault_url: String,
    /// Key name inside the vault.
    pub key_name: String,
    /// Optional key version. Empty string ⇒ "latest enabled".
    pub key_version: String,
    /// Wrap algorithm; defaults to `RSA-OAEP-256`.
    pub algorithm: String,
    /// AAD bearer token.
    pub bearer: Option<String>,
    /// Endpoint override — defaults to `vault_url`. Tests point
    /// this at wiremock.
    endpoint: String,
    http: reqwest::Client,
}

impl std::fmt::Debug for AzureKeyVaultClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzureKeyVaultClient")
            .field("vault_url", &"<redacted>")
            .field("key_name", &"<redacted>")
            .field("key_version", &self.key_version)
            .field("algorithm", &self.algorithm)
            .field("bearer", &self.bearer.as_ref().map(|_| "<redacted>"))
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

impl AzureKeyVaultClient {
    /// Build a client targeting the live vault URL.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] if the reqwest client cannot
    /// be constructed.
    pub fn new(vault_url: String, key_name: String, bearer: Option<String>) -> Result<Self> {
        let endpoint = vault_url.clone();
        Self::build(
            vault_url,
            key_name,
            String::new(),
            DEFAULT_ALGORITHM.into(),
            bearer,
            endpoint,
        )
    }

    /// Build a client with an explicit endpoint (test override).
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] if the reqwest client cannot
    /// be constructed.
    pub fn with_endpoint(
        vault_url: String,
        key_name: String,
        bearer: Option<String>,
        endpoint: String,
    ) -> Result<Self> {
        Self::build(
            vault_url,
            key_name,
            String::new(),
            DEFAULT_ALGORITHM.into(),
            bearer,
            endpoint,
        )
    }

    /// Set a non-default wrap algorithm (e.g. `RSA1_5`,
    /// `RSA-OAEP`).
    #[must_use]
    pub fn with_algorithm(mut self, algorithm: String) -> Self {
        self.algorithm = algorithm;
        self
    }

    fn build(
        vault_url: String,
        key_name: String,
        key_version: String,
        algorithm: String,
        bearer: Option<String>,
        endpoint: String,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .map_err(|e| ArgoError::Provider(format!("azure-kv client build: {e}")))?;
        Ok(Self {
            vault_url,
            key_name,
            key_version,
            algorithm,
            bearer,
            endpoint,
            http,
        })
    }

    async fn post(&self, suffix: &str, body: serde_json::Value) -> Result<reqwest::Response> {
        let version_seg = if self.key_version.is_empty() {
            String::new()
        } else {
            format!("/{}", self.key_version)
        };
        let url = format!(
            "{}/keys/{}{}/{}?api-version={}",
            self.endpoint, self.key_name, version_seg, suffix, API_VERSION
        );
        let mut req = self.http.post(&url);
        if let Some(bearer) = &self.bearer {
            req = req.header("Authorization", format!("Bearer {bearer}"));
        }
        req.json(&body)
            .send()
            .await
            .map_err(|e| transport_err("azure", e))
    }
}

#[async_trait::async_trait]
impl KmsProvider for AzureKeyVaultClient {
    async fn unwrap_dek(&self, wrapped: &[u8]) -> Result<Vec<u8>> {
        let body = json!({
            "alg": self.algorithm,
            // Azure Key Vault expects base64url-no-pad for `value`.
            "value": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(wrapped),
        });
        let resp = self.post("unwrapkey", body).await?;
        if !resp.status().is_success() {
            return Err(http_error("azure", resp).await);
        }
        let parsed: KeyOpResponse = resp
            .json()
            .await
            .map_err(|e| ArgoError::Provider(format!("azure-kv unwrap parse: {e}")))?;
        let plaintext = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parsed.value.as_bytes())
            .map_err(|e| ArgoError::Provider(format!("azure-kv unwrap base64: {e}")))?;
        validate_dek_len(&plaintext)?;
        Ok(plaintext)
    }

    async fn wrap_dek(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        validate_dek_len(plaintext)?;
        let body = json!({
            "alg": self.algorithm,
            "value": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(plaintext),
        });
        let resp = self.post("wrapkey", body).await?;
        if !resp.status().is_success() {
            return Err(http_error("azure", resp).await);
        }
        let parsed: KeyOpResponse = resp
            .json()
            .await
            .map_err(|e| ArgoError::Provider(format!("azure-kv wrap parse: {e}")))?;
        let blob = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parsed.value.as_bytes())
            .map_err(|e| ArgoError::Provider(format!("azure-kv wrap base64: {e}")))?;
        Ok(blob)
    }
}

#[derive(Deserialize)]
struct KeyOpResponse {
    value: String,
}
