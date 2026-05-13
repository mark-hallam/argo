//! AWS KMS client for BYOK envelope encryption (Stream
//! `mission-byok-enterprise-tenants`).
//!
//! Speaks the AWS KMS JSON-1.1 wire protocol directly via reqwest.
//! The choice avoids pulling the full `aws-sdk-kms` (which would
//! add ~40 transitive crates and ~3 min to cold compile) while
//! keeping the surface wiremock-testable: the production code path
//! is the same HTTP POST the SDK would issue, just without the
//! SDK's endpoint resolver in the way.
//!
//! # Auth
//!
//! v1 takes a pre-signed *bearer token* from the operator. In a
//! production SKYCOT deployment that token is an STS session
//! `Authorization: AWS4-HMAC-SHA256 ...` header value produced by a
//! sidecar (e.g. AWS Vault, granted_assume_role, or an IRSA
//! webhook). SigV4 bundling and STS `AssumeRole` are deliberately
//! Phase 2 — the v1 wire is the load-bearing piece; signing is a
//! sidecar concern.
//!
//! # Endpoint
//!
//! Defaults to `https://kms.<region>.amazonaws.com`. Tests override
//! via [`AwsKmsClient::with_endpoint`] to point at wiremock.

use std::time::Duration;

use base64::Engine;
use serde::Deserialize;
use serde_json::json;

use crate::kms::{http_error, transport_err, validate_dek_len, KmsProvider};
use crate::StoreResult as Result;
use argo_types::ArgoError;

const KMS_TARGET_DECRYPT: &str = "TrentService.Decrypt";
const KMS_TARGET_ENCRYPT: &str = "TrentService.Encrypt";
const CONTENT_TYPE: &str = "application/x-amz-json-1.1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// AWS KMS client. Configured once per [`KeySource`] resolution.
///
/// [`KeySource`]: crate::encryption::KeySource
#[derive(Clone)]
pub struct AwsKmsClient {
    /// Full KMS key ARN (e.g.
    /// `arn:aws:kms:us-east-1:111122223333:key/<uuid>`).
    pub key_arn: String,
    /// AWS region the key lives in. Drives the default endpoint.
    pub region: String,
    /// Operator-supplied `Authorization:` header value (e.g. a
    /// pre-signed SigV4 string or an STS session token). Optional
    /// for wiremock tests; required in production.
    pub bearer: Option<String>,
    /// Endpoint override — defaults to the AWS regional endpoint.
    /// Tests override to point at a wiremock localhost stub.
    endpoint: String,
    /// Shared reqwest client (kept on the struct so connection
    /// pooling carries across decrypt/encrypt cycles).
    http: reqwest::Client,
}

impl std::fmt::Debug for AwsKmsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Redact bearer + ARN suffix; the ARN account id + key id
        // are mildly sensitive (capability tokens).
        f.debug_struct("AwsKmsClient")
            .field("key_arn", &"<redacted>")
            .field("region", &self.region)
            .field("bearer", &self.bearer.as_ref().map(|_| "<redacted>"))
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

impl AwsKmsClient {
    /// Build a client with the AWS regional default endpoint.
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] if the reqwest client cannot
    /// be constructed (e.g. TLS root store unavailable).
    pub fn new(key_arn: String, region: String, bearer: Option<String>) -> Result<Self> {
        let endpoint = format!("https://kms.{region}.amazonaws.com");
        Self::build(key_arn, region, bearer, endpoint)
    }

    /// Build a client with an explicit endpoint (test override).
    ///
    /// # Errors
    /// Returns [`ArgoError::Provider`] if the reqwest client cannot
    /// be constructed.
    pub fn with_endpoint(
        key_arn: String,
        region: String,
        bearer: Option<String>,
        endpoint: String,
    ) -> Result<Self> {
        Self::build(key_arn, region, bearer, endpoint)
    }

    fn build(
        key_arn: String,
        region: String,
        bearer: Option<String>,
        endpoint: String,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .map_err(|e| ArgoError::Provider(format!("aws-kms client build: {e}")))?;
        Ok(Self {
            key_arn,
            region,
            bearer,
            endpoint,
            http,
        })
    }

    async fn post(&self, target: &str, body: serde_json::Value) -> Result<reqwest::Response> {
        let mut req = self
            .http
            .post(&self.endpoint)
            .header("Content-Type", CONTENT_TYPE)
            .header("X-Amz-Target", target);
        if let Some(bearer) = &self.bearer {
            req = req.header("Authorization", bearer);
        }
        req.json(&body)
            .send()
            .await
            .map_err(|e| transport_err("aws", e))
    }
}

#[async_trait::async_trait]
impl KmsProvider for AwsKmsClient {
    async fn unwrap_dek(&self, wrapped: &[u8]) -> Result<Vec<u8>> {
        let body = json!({
            "CiphertextBlob": base64::engine::general_purpose::STANDARD.encode(wrapped),
            "KeyId": self.key_arn,
        });
        let resp = self.post(KMS_TARGET_DECRYPT, body).await?;
        if !resp.status().is_success() {
            return Err(http_error("aws", resp).await);
        }
        let parsed: DecryptResponse = resp
            .json()
            .await
            .map_err(|e| ArgoError::Provider(format!("aws-kms decrypt parse: {e}")))?;
        let plaintext = base64::engine::general_purpose::STANDARD
            .decode(parsed.plaintext.as_bytes())
            .map_err(|e| ArgoError::Provider(format!("aws-kms decrypt base64: {e}")))?;
        validate_dek_len(&plaintext)?;
        Ok(plaintext)
    }

    async fn wrap_dek(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        validate_dek_len(plaintext)?;
        let body = json!({
            "Plaintext": base64::engine::general_purpose::STANDARD.encode(plaintext),
            "KeyId": self.key_arn,
        });
        let resp = self.post(KMS_TARGET_ENCRYPT, body).await?;
        if !resp.status().is_success() {
            return Err(http_error("aws", resp).await);
        }
        let parsed: EncryptResponse = resp
            .json()
            .await
            .map_err(|e| ArgoError::Provider(format!("aws-kms encrypt parse: {e}")))?;
        let blob = base64::engine::general_purpose::STANDARD
            .decode(parsed.ciphertext_blob.as_bytes())
            .map_err(|e| ArgoError::Provider(format!("aws-kms encrypt base64: {e}")))?;
        Ok(blob)
    }
}

#[derive(Deserialize)]
struct DecryptResponse {
    #[serde(rename = "Plaintext")]
    plaintext: String,
}

#[derive(Deserialize)]
struct EncryptResponse {
    #[serde(rename = "CiphertextBlob")]
    ciphertext_blob: String,
}
