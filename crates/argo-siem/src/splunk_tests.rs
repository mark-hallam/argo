//! Unit tests for [`crate::splunk`].
use super::*;
use crate::sink::{AuditEvent, SiemSink, SinkError};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ev(seq: i64) -> AuditEvent {
    AuditEvent {
        seq,
        id: format!("id-{seq}"),
        session_id: Some("sess".to_string()),
        kind: "tool_call".to_string(),
        payload: serde_json::json!({"a": 1}),
        created_at: 1_700_000_000 + seq,
    }
}

#[tokio::test]
async fn ships_batch_to_hec_endpoint_with_auth_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/services/collector"))
        .and(header("authorization", "Splunk supersecret"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let sink = SplunkHecSink::new(server.uri(), "supersecret".into(), None).unwrap();
    sink.ship(vec![ev(1), ev(2)]).await.unwrap();
    server.verify().await;
}

#[tokio::test]
async fn maps_5xx_to_transient_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let sink = SplunkHecSink::new(server.uri(), "t".into(), None).unwrap();
    match sink.ship(vec![ev(1)]).await {
        Err(SinkError::Transient(_)) => {}
        other => panic!("expected Transient, got {other:?}"),
    }
}

#[test]
fn debug_redacts_token() {
    let sink = SplunkHecSink::new("https://splunk.example.com".into(), "topsecret".into(), None)
        .unwrap();
    let dbg = format!("{sink:?}");
    assert!(!dbg.contains("topsecret"), "Debug must not leak token");
    assert!(dbg.contains("<redacted>"));
}

#[test]
fn encode_batch_emits_one_envelope_per_line() {
    let sink = SplunkHecSink::new("https://splunk.example.com".into(), "t".into(), None).unwrap();
    let body = sink.encode_batch(&[ev(1), ev(2)]);
    let lines: Vec<&str> = body.trim_end().split('\n').collect();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(v.get("event").is_some());
        assert_eq!(v["sourcetype"], "argo:audit");
    }
}

