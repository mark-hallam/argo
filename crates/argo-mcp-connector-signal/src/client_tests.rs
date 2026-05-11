//! Wiremock-backed unit tests for `SignalConnector`.
//!
//! Five scenarios required by the brief:
//!  1. `send_message` round-trips a direct send and decodes the timestamp.
//!  2. `send_group_message` POSTs `groupId` instead of `recipient`.
//!  3. `receive` returns an empty list when the daemon queue is drained.
//!  4. `receive` decodes attachment metadata on a real envelope.
//!  5. Daemon-unreachable surfaces a typed `ArgoError::Provider` whose message
//!     hints at running signal-cli in daemon mode.

use super::*;
use crate::types::SignalIncomingMessage;
use wiremock::matchers::{body_json_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ACCOUNT: &str = "+15551234567";

fn connector(server: &MockServer) -> SignalConnector {
    SignalConnector::new(server.uri(), ACCOUNT.into())
}

#[tokio::test]
async fn send_message_posts_direct_payload_and_decodes_timestamp() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/send"))
        .and(header("content-type", "application/json"))
        .and(body_json_string(
            r#"{"number":"+15551234567","recipient":"+15557654321","message":"hello"}"#,
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "timestamp": 1_700_000_000_123_i64 })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let receipt = connector(&server)
        .send_message("+15557654321", "hello")
        .await
        .unwrap();
    assert_eq!(receipt.timestamp, 1_700_000_000_123);
}

#[tokio::test]
async fn send_group_message_uses_group_id_field() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/send"))
        .and(body_json_string(
            r#"{"number":"+15551234567","groupId":"grp-base64-id","message":"ship it"}"#,
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "timestamp": 42_i64 })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let receipt = connector(&server)
        .send_group_message("grp-base64-id", "ship it")
        .await
        .unwrap();
    assert_eq!(receipt.timestamp, 42);
}

#[tokio::test]
async fn receive_returns_empty_list_when_queue_drained() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/receive/+15551234567"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let inbox = connector(&server).receive().await.unwrap();
    assert!(inbox.is_empty(), "drained queue must surface as empty Vec");
}

#[tokio::test]
async fn receive_decodes_attachment_meta_and_optional_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/receive/+15551234567"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "envelope": {
                    "timestamp": 1_700_000_000_001_i64,
                    "sourceNumber": "+15557654321",
                    "dataMessage": {
                        "message": "see attachment",
                        "groupInfo": { "groupId": "grp-xyz" },
                        "attachments": [
                            {
                                "id": "att-1",
                                "contentType": "image/jpeg",
                                "size": 2048
                            }
                        ]
                    }
                }
            },
            {
                "envelope": {
                    "timestamp": 1_700_000_000_002_i64,
                    "sourceNumber": "+15557654321"
                }
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let inbox: Vec<SignalIncomingMessage> = connector(&server).receive().await.unwrap();
    assert_eq!(inbox.len(), 2);
    let first = &inbox[0];
    assert_eq!(first.envelope_timestamp, 1_700_000_000_001);
    assert_eq!(first.source_number, "+15557654321");
    assert_eq!(first.body.as_deref(), Some("see attachment"));
    assert_eq!(first.group_id.as_deref(), Some("grp-xyz"));
    assert_eq!(first.attachments.len(), 1);
    assert_eq!(first.attachments[0].id, "att-1");
    assert_eq!(first.attachments[0].content_type, "image/jpeg");
    assert_eq!(first.attachments[0].size, 2048);

    let second = &inbox[1];
    assert!(
        second.body.is_none(),
        "receipt-style envelope must surface body=None"
    );
    assert!(second.group_id.is_none());
    assert!(second.attachments.is_empty());
}

#[tokio::test]
async fn daemon_unreachable_surfaces_typed_provider_error() {
    // Bind to a port and immediately drop the listener so the connect attempt
    // fails fast with ECONNREFUSED. The daemon URL is well-formed but no
    // process is listening — this is the realistic "daemon not running" mode.
    let unreachable = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        format!("http://127.0.0.1:{port}")
    };
    let signal = SignalConnector::new(unreachable.clone(), ACCOUNT.into());
    let err = signal
        .send_message("+15557654321", "hi")
        .await
        .expect_err("unreachable daemon must surface an error");
    match err {
        ArgoError::Provider(msg) => {
            assert!(
                msg.contains("ensure signal-cli daemon mode is running"),
                "error must hint at daemon mode, got {msg:?}"
            );
            assert!(
                msg.contains(&unreachable),
                "error must echo daemon URL, got {msg:?}"
            );
        }
        other => panic!("expected ArgoError::Provider, got {other:?}"),
    }
}

#[test]
fn validate_account_contract() {
    assert!(super::validate_account("+15551234567").is_ok());
    assert!(super::validate_account("+1").is_ok());
    let no_plus = super::validate_account("15551234567").expect_err("missing + must fail");
    match no_plus {
        ArgoError::Schema(msg) => assert!(msg.contains("E.164"), "got {msg:?}"),
        other => panic!("expected ArgoError::Schema, got {other:?}"),
    }
    let bad_chars = super::validate_account("+1-555").expect_err("non-digits must fail");
    match bad_chars {
        ArgoError::Schema(msg) => assert!(msg.contains("digits"), "got {msg:?}"),
        other => panic!("expected ArgoError::Schema, got {other:?}"),
    }
    let empty = super::validate_account("+").expect_err("empty digits must fail");
    assert!(matches!(empty, ArgoError::Schema(_)));
}
