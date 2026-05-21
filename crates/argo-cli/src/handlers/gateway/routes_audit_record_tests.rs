//! Tests for `routes_audit_record` — POST /api/v1/audit.

#![allow(deprecated)]

use super::record_response;
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.expect("open mem db");
    s.migrate().await.expect("migrate");
    Arc::new(s)
}

async fn seed_action_id(store: &Arc<SessionStore>) -> String {
    AuditSink::new(Arc::clone(store))
        .write(None, AuditKind::ToolCall, json!({"tool": "x"}), None)
        .await
        .unwrap();
    let id: Vec<u8> = sqlx::query_scalar(
        "SELECT id FROM audit_record WHERE kind = 'tool_call' ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    hex::encode(id)
}

fn post(body: Value) -> String {
    let s = body.to_string();
    format!(
        "POST /api/v1/audit HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{s}",
        s.len()
    )
}

fn parse(resp: &(&str, String)) -> Value {
    serde_json::from_str(&resp.1).unwrap()
}

#[tokio::test]
async fn record_minimal_body_inserts_chain_row() {
    let store = mem_store().await;
    let resp = record_response(
        &post(json!({"kind": "inspection", "body": "I checked seq 12 manually."})),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("201"), "status: {} body: {}", resp.0, resp.1);
    let body = parse(&resp);
    assert!(body["audit_id"].as_str().unwrap().len() == 32);
    assert_eq!(body["annotation_kind"], "inspection");
    assert_eq!(body["kind"], AuditKind::HttpRequest.label());
    // Annotation must be visible in the chain.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%audit.annotation%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn record_carries_related_action_id_into_payload() {
    let store = mem_store().await;
    let action = seed_action_id(&store).await;
    let resp = record_response(
        &post(json!({
            "kind": "cross_check",
            "body": "verified row matches Stripe ledger",
            "related_action_id": action,
        })),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("201"), "status: {}", resp.0);
    let body = parse(&resp);
    assert_eq!(body["related_action_id"], action);
    // Payload should embed the related_action_id.
    let payload_json: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE payload_json LIKE '%audit.annotation%' \
         ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(payload_json.contains(&action));
}

#[tokio::test]
async fn record_missing_kind_returns_400() {
    let store = mem_store().await;
    let resp = record_response(
        &post(json!({"body": "no kind field"})),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("400"), "status: {}", resp.0);
}

#[tokio::test]
async fn record_missing_body_returns_400() {
    let store = mem_store().await;
    let resp = record_response(
        &post(json!({"kind": "inspection"})),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("400"), "status: {}", resp.0);
}

#[tokio::test]
async fn record_empty_kind_returns_400() {
    let store = mem_store().await;
    let resp = record_response(
        &post(json!({"kind": "  ", "body": "ok"})),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("400"), "status: {}", resp.0);
}

#[tokio::test]
async fn record_oversize_body_returns_400() {
    let store = mem_store().await;
    let big = "a".repeat(8 * 1024 + 1);
    let resp = record_response(
        &post(json!({"kind": "x", "body": big})),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("400"), "status: {}", resp.0);
}

#[tokio::test]
async fn record_bad_related_action_id_returns_400() {
    let store = mem_store().await;
    let resp = record_response(
        &post(json!({
            "kind": "x",
            "body": "ok",
            "related_action_id": "not-hex",
        })),
        Some(&store),
    )
    .await;
    assert!(resp.0.starts_with("400"), "status: {}", resp.0);
}

#[tokio::test]
async fn record_appears_in_audit_feed() {
    let store = mem_store().await;
    record_response(
        &post(json!({"kind": "inspection", "body": "feed visibility check"})),
        Some(&store),
    )
    .await;
    // Use the same export path the /audit feed uses to confirm the
    // annotation row landed and is readable.
    let rows = argo_audit::tail_rows(&store, 10).await.unwrap();
    let ann = rows
        .iter()
        .find(|r| r.payload.get("endpoint").and_then(|v| v.as_str()) == Some("audit.annotation"))
        .expect("annotation row visible in feed");
    assert_eq!(ann.kind, AuditKind::HttpRequest.label());
    assert_eq!(
        ann.payload.get("annotation_kind").and_then(|v| v.as_str()),
        Some("inspection")
    );
}
