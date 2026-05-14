//! Unit tests for [`crate::elastic`].
use super::*;
use crate::sink::{AuditEvent, SiemSink, SinkError};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ev(seq: i64) -> AuditEvent {
    AuditEvent {
        seq,
        id: format!("id-{seq}"),
        session_id: None,
        kind: "gateway_request".to_string(),
        payload: serde_json::json!({"k": "v"}),
        created_at: 1_700_000_000,
    }
}

#[tokio::test]
async fn ships_bulk_to_endpoint_with_ndjson_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/_bulk"))
        .and(header("content-type", "application/x-ndjson"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let sink = ElasticSink::new(server.uri(), String::new(), None).unwrap();
    sink.ship(vec![ev(1), ev(2)]).await.unwrap();
    server.verify().await;
}

#[tokio::test]
async fn sends_authorization_when_token_non_empty() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "ApiKey base64stuff"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let sink = ElasticSink::new(server.uri(), "ApiKey base64stuff".into(), None).unwrap();
    sink.ship(vec![ev(1)]).await.unwrap();
    server.verify().await;
}

#[tokio::test]
async fn maps_5xx_to_transient() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(502))
        .mount(&server)
        .await;
    let sink = ElasticSink::new(server.uri(), String::new(), None).unwrap();
    match sink.ship(vec![ev(1)]).await {
        Err(SinkError::Transient(_)) => {}
        other => panic!("expected Transient, got {other:?}"),
    }
}

#[test]
fn debug_redacts_auth_header() {
    let sink = ElasticSink::new("https://es.example.com".into(), "Basic c2VjcmV0".into(), None)
        .unwrap();
    let dbg = format!("{sink:?}");
    assert!(!dbg.contains("c2VjcmV0"));
}

#[test]
fn encode_alternates_action_and_doc_lines() {
    let sink = ElasticSink::new("https://es".into(), String::new(), Some("my-idx".into())).unwrap();
    let body = sink.encode_batch(&[ev(1), ev(2)]);
    let lines: Vec<&str> = body.trim_end().split('\n').collect();
    assert_eq!(lines.len(), 4, "two events → 4 lines (2 action + 2 doc)");
    let action: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(action["index"]["_index"], "my-idx");
    let doc: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(doc["seq"], 1);
}

