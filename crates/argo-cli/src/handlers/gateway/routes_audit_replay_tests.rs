//! Tests for `routes_audit_replay` — POST /api/v1/audit/<id>/replay.

#![allow(deprecated)]

use super::replay_response;
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use serde_json::Value;
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.expect("open mem db");
    s.migrate().await.expect("migrate");
    Arc::new(s)
}

async fn seed_one(store: &Arc<SessionStore>) -> String {
    AuditSink::new(Arc::clone(store))
        .write(
            None,
            AuditKind::ToolCall,
            serde_json::json!({"tool": "echo", "args": {"text": "hello"}}),
            Some("test-principal".to_string()),
        )
        .await
        .expect("write audit");
    let id: Vec<u8> = sqlx::query_scalar(
        "SELECT id FROM audit_record WHERE kind = 'tool_call' ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .expect("read id");
    hex::encode(id)
}

fn post(path: &str) -> String {
    format!("POST {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

fn parse(resp: &(&str, String)) -> Value {
    serde_json::from_str(&resp.1).unwrap()
}

#[tokio::test]
async fn replay_intact_row_returns_same_verdict() {
    let store = mem_store().await;
    let id = seed_one(&store).await;
    let resp = replay_response(&id, &post(&format!("/api/v1/audit/{id}/replay")), Some(&store)).await;
    assert!(resp.0.starts_with("200"), "status: {}", resp.0);
    let body = parse(&resp);
    assert_eq!(body["verdict"], "same");
    assert_eq!(body["kind"], "tool_call");
    assert_eq!(body["audit_id"], id);
    assert_eq!(
        body["expected_self_hash"], body["actual_self_hash"],
        "hashes must match on intact rows"
    );
    assert_eq!(body["kind_label_known"], true);
    assert_eq!(body["diff"], serde_json::json!({}));
    assert!(body["replayed_at"].is_string());
}

#[tokio::test]
async fn replay_tampered_payload_returns_differ_verdict() {
    let store = mem_store().await;
    let id = seed_one(&store).await;
    let id_bytes = hex::decode(&id).unwrap();
    // Mutate payload_json after the fact — chain hash will no
    // longer recompute to the stored self_hash.
    sqlx::query("UPDATE audit_record SET payload_json = ? WHERE id = ?")
        .bind(r#"{"tampered":true}"#)
        .bind(&id_bytes)
        .execute(store.pool())
        .await
        .unwrap();
    let resp = replay_response(&id, &post(&format!("/api/v1/audit/{id}/replay")), Some(&store)).await;
    assert!(resp.0.starts_with("200"), "status: {}", resp.0);
    let body = parse(&resp);
    assert_eq!(body["verdict"], "differ");
    assert_ne!(body["expected_self_hash"], body["actual_self_hash"]);
    assert!(body["diff"]["self_hash"].is_string());
}

#[tokio::test]
async fn replay_unknown_id_returns_404() {
    let store = mem_store().await;
    let bogus = "00112233445566778899aabbccddeeff";
    let resp = replay_response(bogus, &post(&format!("/api/v1/audit/{bogus}/replay")), Some(&store)).await;
    assert!(resp.0.starts_with("404"), "status: {}", resp.0);
    let body = parse(&resp);
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn replay_malformed_id_returns_400() {
    let store = mem_store().await;
    let resp = replay_response("not-hex", &post("/api/v1/audit/not-hex/replay"), Some(&store)).await;
    assert!(resp.0.starts_with("400"), "status: {}", resp.0);
}

#[tokio::test]
async fn replay_is_deterministic_same_input_same_verdict() {
    let store = mem_store().await;
    let id = seed_one(&store).await;
    let req = post(&format!("/api/v1/audit/{id}/replay"));
    let first = replay_response(&id, &req, Some(&store)).await;
    let second = replay_response(&id, &req, Some(&store)).await;
    let a: Value = parse(&first);
    let b: Value = parse(&second);
    // Verdicts and hashes must be identical between runs even
    // though `replayed_at` differs.
    assert_eq!(a["verdict"], b["verdict"]);
    assert_eq!(a["expected_self_hash"], b["expected_self_hash"]);
    assert_eq!(a["actual_self_hash"], b["actual_self_hash"]);
}

#[tokio::test]
async fn replay_unknown_kind_returns_error_verdict() {
    let store = mem_store().await;
    let id = seed_one(&store).await;
    let id_bytes = hex::decode(&id).unwrap();
    sqlx::query("UPDATE audit_record SET kind = 'fictional.kind' WHERE id = ?")
        .bind(&id_bytes)
        .execute(store.pool())
        .await
        .unwrap();
    let resp = replay_response(&id, &post(&format!("/api/v1/audit/{id}/replay")), Some(&store)).await;
    let body = parse(&resp);
    assert_eq!(body["verdict"], "error");
    assert_eq!(body["kind_label_known"], false);
    assert!(body["diff"]["kind"].is_string());
}
