//! Unit tests for [`crate::datadog`].
use super::*;
use crate::sink::{AuditEvent, SiemSink, SinkError};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ev(seq: i64) -> AuditEvent {
    AuditEvent {
        seq,
        id: format!("id-{seq}"),
        session_id: Some("s".to_string()),
        kind: "security_event".to_string(),
        payload: serde_json::json!({"alert": true}),
        created_at: 1_700_000_000,
    }
}

fn sink_for(server: &MockServer) -> DatadogLogsSink {
    DatadogLogsSink::new(
        format!("{}/api/v2/logs", server.uri()),
        "ddapikey".into(),
        None,
        None,
    )
    .unwrap()
}

#[tokio::test]
async fn ships_to_logs_v2_with_dd_api_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/logs"))
        .and(header("dd-api-key", "ddapikey"))
        .respond_with(ResponseTemplate::new(202))
        .expect(1)
        .mount(&server)
        .await;
    let sink = sink_for(&server);
    sink.ship(vec![ev(1), ev(2)]).await.unwrap();
    server.verify().await;
}

#[tokio::test]
async fn maps_5xx_to_transient() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let sink = sink_for(&server);
    match sink.ship(vec![ev(1)]).await {
        Err(SinkError::Transient(_)) => {}
        other => panic!("expected Transient, got {other:?}"),
    }
}

#[test]
fn debug_redacts_api_key() {
    let sink = DatadogLogsSink::new(
        "https://http-intake.logs.datadoghq.com/api/v2/logs".into(),
        "DEADBEEFKEY".into(),
        None,
        None,
    )
    .unwrap();
    let dbg = format!("{sink:?}");
    assert!(!dbg.contains("DEADBEEFKEY"));
}

#[test]
fn encode_emits_json_array_with_canonical_fields() {
    let sink = DatadogLogsSink::new("https://x".into(), "k".into(), None, None).unwrap();
    let body = sink.encode_batch(&[ev(1)]);
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
    let arr = parsed.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["ddsource"], "argo");
    assert_eq!(arr[0]["service"], "argo-gateway");
    assert_eq!(arr[0]["seq"], 1);
}

