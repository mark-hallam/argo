//! Wiremock-based unit tests for the cloud KMS providers (Stream
//! `mission-byok-enterprise-tenants`).
//!
//! These tests stand up an in-process wiremock server, point each
//! provider client's endpoint override at it, and verify the wire
//! contract:
//!
//! - Decrypt happy path returns the plaintext DEK.
//! - Encrypt happy path returns a wrapped DEK that can round-trip
//!   through [`crate::kms::rewrap_dek`].
//! - Non-2xx responses surface as `ArgoError::Provider` with the
//!   provider name + HTTP status — the operator-actionable
//!   contract from the brief's acceptance criteria.
//! - End-to-end: [`KeySource::parse`] + [`KeySource::resolve_async`]
//!   integrate over a wiremock-stubbed endpoint.
//!
//! No real-cloud calls happen in CI.

#![cfg(feature = "kms")]

use base64::Engine;
use serde_json::json;
use wiremock::matchers::{header, header_exists, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::encryption::KeySource;
use crate::kms::{rewrap_dek, KmsProvider};

const SAMPLE_DEK: [u8; 32] = [
    0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
    0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef,
];
const SAMPLE_DEK_HEX: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SAMPLE_WRAPPED: &[u8] = b"FAKE-WRAPPED-DEK-CIPHERTEXT-BLOB";

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn b64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

// ----------------------------------------------------------------
// AWS KMS
// ----------------------------------------------------------------

#[cfg(feature = "kms-aws")]
mod aws_tests {
    use super::*;
    use crate::kms::aws::AwsKmsClient;

    fn client(endpoint: String, bearer: Option<String>) -> AwsKmsClient {
        AwsKmsClient::with_endpoint(
            "arn:aws:kms:us-east-1:111122223333:key/abcd-1234".into(),
            "us-east-1".into(),
            bearer,
            endpoint,
        )
        .expect("aws-kms client build")
    }

    #[tokio::test]
    async fn unwrap_dek_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .and(header("X-Amz-Target", "TrentService.Decrypt"))
            .and(header("Authorization", "test-sigv4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "KeyId": "arn:aws:kms:us-east-1:111122223333:key/abcd-1234",
                "Plaintext": b64(&SAMPLE_DEK),
            })))
            .expect(1)
            .mount(&server)
            .await;

        let c = client(server.uri(), Some("test-sigv4".into()));
        let plaintext = c.unwrap_dek(SAMPLE_WRAPPED).await.expect("decrypt");
        assert_eq!(plaintext, SAMPLE_DEK);
    }

    #[tokio::test]
    async fn wrap_dek_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("X-Amz-Target", "TrentService.Encrypt"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "CiphertextBlob": b64(SAMPLE_WRAPPED),
                "KeyId": "arn:aws:kms:us-east-1:111122223333:key/abcd-1234",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let c = client(server.uri(), None);
        let wrapped = c.wrap_dek(&SAMPLE_DEK).await.expect("encrypt");
        assert_eq!(wrapped, SAMPLE_WRAPPED);
    }

    #[tokio::test]
    async fn iam_denied_surfaces_provider_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("X-Amz-Target", "TrentService.Decrypt"))
            .respond_with(ResponseTemplate::new(400).set_body_string(
                r#"{"__type":"AccessDeniedException","message":"User is not authorized to perform kms:Decrypt"}"#,
            ))
            .expect(1)
            .mount(&server)
            .await;

        let c = client(server.uri(), None);
        let err = c.unwrap_dek(SAMPLE_WRAPPED).await.expect_err("must fail");
        let s = format!("{err}");
        assert!(s.contains("aws"), "missing provider tag: {s}");
        assert!(s.contains("400"), "missing status: {s}");
        assert!(
            s.contains("AccessDenied") || s.contains("not authorized"),
            "missing IAM hint: {s}"
        );
    }

    #[tokio::test]
    async fn rejects_short_dek_from_kms() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "KeyId": "arn:aws:kms:us-east-1:111122223333:key/abcd-1234",
                "Plaintext": b64(&[0u8; 16]),  // 16 bytes, not 32
            })))
            .mount(&server)
            .await;

        let c = client(server.uri(), None);
        let err = c.unwrap_dek(SAMPLE_WRAPPED).await.expect_err("must fail");
        assert!(format!("{err}").contains("32 bytes"));
    }
}

// ----------------------------------------------------------------
// GCP Cloud KMS
// ----------------------------------------------------------------

#[cfg(feature = "kms-gcp")]
mod gcp_tests {
    use super::*;
    use crate::kms::gcp::GcpCmekClient;

    const KEY_PATH: &str =
        "/v1/projects/acme/locations/global/keyRings/skycot/cryptoKeys/db";

    fn client(endpoint: String, bearer: Option<String>) -> GcpCmekClient {
        GcpCmekClient::with_endpoint(
            "acme".into(),
            "skycot".into(),
            "db".into(),
            "global".into(),
            bearer,
            endpoint,
        )
        .expect("gcp-cmek client build")
    }

    #[tokio::test]
    async fn unwrap_dek_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("{KEY_PATH}:decrypt")))
            .and(header("Authorization", "Bearer oauth-token-abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "plaintext": b64(&SAMPLE_DEK),
            })))
            .expect(1)
            .mount(&server)
            .await;

        let c = client(server.uri(), Some("oauth-token-abc".into()));
        let plaintext = c.unwrap_dek(SAMPLE_WRAPPED).await.expect("decrypt");
        assert_eq!(plaintext, SAMPLE_DEK);
    }

    #[tokio::test]
    async fn wrap_dek_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("{KEY_PATH}:encrypt")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ciphertext": b64(SAMPLE_WRAPPED),
                "name": "projects/acme/locations/global/keyRings/skycot/cryptoKeys/db/cryptoKeyVersions/1",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let c = client(server.uri(), None);
        let wrapped = c.wrap_dek(&SAMPLE_DEK).await.expect("encrypt");
        assert_eq!(wrapped, SAMPLE_WRAPPED);
    }

    #[tokio::test]
    async fn key_not_found_surfaces_provider_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("{KEY_PATH}:decrypt")))
            .respond_with(ResponseTemplate::new(404).set_body_string(
                r#"{"error":{"code":404,"message":"CryptoKey not found"}}"#,
            ))
            .mount(&server)
            .await;

        let c = client(server.uri(), None);
        let err = c.unwrap_dek(SAMPLE_WRAPPED).await.expect_err("must fail");
        let s = format!("{err}");
        assert!(s.contains("gcp"), "missing provider tag: {s}");
        assert!(s.contains("404"), "missing status: {s}");
    }
}

// ----------------------------------------------------------------
// Azure Key Vault
// ----------------------------------------------------------------

#[cfg(feature = "kms-azure")]
mod azure_tests {
    use super::*;
    use crate::kms::azure::AzureKeyVaultClient;

    fn client(endpoint: String, bearer: Option<String>) -> AzureKeyVaultClient {
        AzureKeyVaultClient::with_endpoint(
            "https://my-vault.vault.azure.net".into(),
            "db-key".into(),
            bearer,
            endpoint,
        )
        .expect("azure-kv client build")
    }

    #[tokio::test]
    async fn unwrap_dek_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/keys/db-key/unwrapkey"))
            .and(query_param("api-version", "7.4"))
            .and(header("Authorization", "Bearer aad-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "kid": "https://my-vault.vault.azure.net/keys/db-key/abc",
                "value": b64url(&SAMPLE_DEK),
            })))
            .expect(1)
            .mount(&server)
            .await;

        let c = client(server.uri(), Some("aad-token".into()));
        let plaintext = c.unwrap_dek(SAMPLE_WRAPPED).await.expect("unwrap");
        assert_eq!(plaintext, SAMPLE_DEK);
    }

    #[tokio::test]
    async fn wrap_dek_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/keys/db-key/wrapkey"))
            .and(query_param("api-version", "7.4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "kid": "https://my-vault.vault.azure.net/keys/db-key/abc",
                "value": b64url(SAMPLE_WRAPPED),
            })))
            .expect(1)
            .mount(&server)
            .await;

        let c = client(server.uri(), None);
        let wrapped = c.wrap_dek(&SAMPLE_DEK).await.expect("wrap");
        assert_eq!(wrapped, SAMPLE_WRAPPED);
    }

    #[tokio::test]
    async fn forbidden_surfaces_provider_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/keys/db-key/unwrapkey"))
            .respond_with(ResponseTemplate::new(403).set_body_string(
                r#"{"error":{"code":"Forbidden","message":"Caller is not authorized"}}"#,
            ))
            .mount(&server)
            .await;

        let c = client(server.uri(), None);
        let err = c.unwrap_dek(SAMPLE_WRAPPED).await.expect_err("must fail");
        let s = format!("{err}");
        assert!(s.contains("azure"), "missing provider tag: {s}");
        assert!(s.contains("403"), "missing status: {s}");
    }
}

// ----------------------------------------------------------------
// Cross-provider primitives + end-to-end KeySource integration
// ----------------------------------------------------------------

#[cfg(all(feature = "kms-aws", feature = "kms-gcp"))]
#[tokio::test]
async fn rewrap_dek_aws_to_gcp_keeps_dek_constant() {
    use crate::kms::aws::AwsKmsClient;
    use crate::kms::gcp::GcpCmekClient;

    let aws_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("X-Amz-Target", "TrentService.Decrypt"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "KeyId": "arn:aws:kms:us-east-1:111122223333:key/old",
            "Plaintext": b64(&SAMPLE_DEK),
        })))
        .expect(1)
        .mount(&aws_server)
        .await;

    let gcp_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/projects/acme/locations/global/keyRings/skycot/cryptoKeys/db:encrypt",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ciphertext": b64(b"NEW-WRAPPED-DEK"),
        })))
        .expect(1)
        .mount(&gcp_server)
        .await;

    let old = AwsKmsClient::with_endpoint(
        "arn:aws:kms:us-east-1:111122223333:key/old".into(),
        "us-east-1".into(),
        None,
        aws_server.uri(),
    )
    .unwrap();
    let new = GcpCmekClient::with_endpoint(
        "acme".into(),
        "skycot".into(),
        "db".into(),
        "global".into(),
        None,
        gcp_server.uri(),
    )
    .unwrap();

    let new_wrapped = rewrap_dek(&old, &new, SAMPLE_WRAPPED)
        .await
        .expect("rewrap");
    assert_eq!(new_wrapped, b"NEW-WRAPPED-DEK");
}

#[cfg(feature = "kms-aws")]
#[tokio::test]
async fn key_source_resolve_async_aws_kms_end_to_end() {
    use std::io::Write;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("X-Amz-Target", "TrentService.Decrypt"))
        .and(header_exists("Authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "KeyId": "arn:aws:kms:us-east-1:111122223333:key/integration",
            "Plaintext": b64(&SAMPLE_DEK),
        })))
        .expect(1)
        .mount(&server)
        .await;

    let mut wrapped_file = tempfile::NamedTempFile::new().unwrap();
    wrapped_file.write_all(SAMPLE_WRAPPED).unwrap();
    let mut bearer_file = tempfile::NamedTempFile::new().unwrap();
    bearer_file.write_all(b"AWS4-HMAC-SHA256 Credential=...").unwrap();

    let spec = format!(
        "aws-kms:arn:aws:kms:us-east-1:111122223333:key/integration?region=us-east-1\
         &wrapped=file:{wrapped}&bearer=file:{bearer}&endpoint={endpoint}",
        wrapped = wrapped_file.path().to_string_lossy(),
        bearer = bearer_file.path().to_string_lossy(),
        endpoint = server.uri(),
    );
    let ks = KeySource::parse(&spec).expect("parse");
    let resolved = ks.resolve_async().await.expect("resolve_async");
    assert_eq!(resolved.as_deref(), Some(SAMPLE_DEK_HEX));
}

#[tokio::test]
async fn key_source_sync_resolve_rejects_kms_variants() {
    let ks = KeySource::parse(
        "aws-kms:arn:aws:kms:us-east-1:111122223333:key/x?region=us-east-1\
         &wrapped=env:DOES_NOT_MATTER",
    )
    .unwrap();
    let err = ks.resolve().expect_err("sync must reject KMS");
    assert!(
        format!("{err}").contains("resolve_async"),
        "error message must redirect to async path"
    );
}

#[test]
fn key_source_parse_rejects_kms_missing_wrapped() {
    let err =
        KeySource::parse("aws-kms:arn:aws:kms:us-east-1:111122223333:key/x?region=us-east-1")
            .expect_err("must fail");
    assert!(format!("{err}").contains("wrapped"));
}

#[test]
fn key_source_parse_rejects_unknown_param() {
    let err = KeySource::parse(
        "aws-kms:arn:aws:kms:us-east-1:111122223333:key/x?\
         region=us-east-1&wrapped=env:X&bogus=1",
    )
    .expect_err("must fail");
    assert!(format!("{err}").contains("unknown param"));
}

#[test]
fn key_source_kms_debug_is_redacted() {
    let ks = KeySource::parse(
        "aws-kms:arn:aws:kms:us-east-1:111122223333:key/secret-id?\
         region=us-east-1&wrapped=env:SECRET_VAR&bearer=env:SECRET_BEARER",
    )
    .unwrap();
    let s = format!("{ks:?}");
    assert!(!s.contains("secret-id"), "Debug leaked key id: {s}");
    assert!(!s.contains("SECRET_VAR"), "Debug leaked wrapped env: {s}");
    assert!(!s.contains("SECRET_BEARER"), "Debug leaked bearer env: {s}");
    assert!(s.contains("AwsKms"));
}
