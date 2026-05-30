//! Tests for the audit-detail render helpers that back the "Run now"
//! drill-in fix (decision chain + provenance for a worker-attributed
//! meta-event). Self-contained: seeds a `worker.run`-shaped audit row
//! through `AuditSink` and drives `detail_response` end-to-end so the
//! assertions cover the real composed JSON the SPA consumes.

// `AuditSink::new` is deprecated in favour of `new_with_tenant`; the
// test fixtures seed under the default tenant intentionally (same
// pattern as `routes_audit_detail_tests.rs`).
#![allow(deprecated)]

use super::super::routes_audit_detail::detail_response;
use argo_audit::{AuditKind, AuditSink, DEFAULT_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn seed_and_id(store: &Arc<SessionStore>, payload: Value) -> String {
    AuditSink::new(Arc::clone(store))
        .write(None, AuditKind::HttpRequest, payload, None)
        .await
        .expect("write audit row");
    let id: Vec<u8> = sqlx::query_scalar(
        "SELECT id FROM audit_record WHERE kind = 'http.request' ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .expect("read back audit id");
    hex::encode(id)
}

fn get(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {DEFAULT_AUDIT_TENANT_ID}\r\n\r\n")
}

fn parse_body(resp: &(&str, String)) -> Value {
    serde_json::from_str(&resp.1).unwrap_or_else(|_| panic!("invalid JSON: {}", resp.1))
}

#[tokio::test]
async fn detail_renders_explicit_manual_trigger_chain() {
    // A "Run now" `worker.run` row carries its own single-step
    // `decision_chain` (step=manual_trigger). The detail composer must
    // render that step with a non-empty body instead of the synthesised
    // 8-step agent trace (all blank → "Decision chain unavailable").
    let store = mem_store().await;
    let payload = json!({
        "endpoint": "worker.run",
        "worker_id": "w-run-1",
        "worker_name": "Mariah",
        "trigger_id": "trig-abc",
        "trigger_source": "manual_run_now",
        "note": "fire the re-pricing duty",
        "decision_chain": [
            {
                "step": "manual_trigger",
                "actor": "human:mark@acme",
                "trigger_source": "manual_run_now",
                "note": "fire the re-pricing duty",
                "ts": "2026-05-30T00:00:00+00:00",
            }
        ],
    });
    let id = seed_and_id(&store, payload).await;
    let resp = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    let body = parse_body(&resp);
    let chain = body["detail"]["decision_chain"]
        .as_array()
        .expect("array");
    assert_eq!(chain.len(), 1, "explicit chain renders one step, got {chain:?}");
    assert_eq!(chain[0]["n"].as_u64(), Some(1));
    assert_eq!(chain[0]["title"], "Manual trigger");
    let step_body = chain[0]["body"].as_str().expect("body string");
    assert!(
        !step_body.trim().is_empty(),
        "manual-trigger step must have a non-empty body, got '{step_body}'"
    );
    assert!(
        step_body.contains("human:mark@acme"),
        "body names the actor: {step_body}"
    );
    // The raw trigger fields are preserved under `details` so the SPA's
    // expandable block (and the runbook pass-cue) can still read them.
    assert_eq!(chain[0]["details"]["step"], "manual_trigger");
}

#[tokio::test]
async fn detail_provenance_anchor_present_for_meta_event() {
    // Even a meta-event row (no model/skill/policy) carries a real
    // hash-chain anchor from its `self_hash`; the SPA renders it as the
    // "Chain anchor" provenance row so the card is not "unavailable".
    let store = mem_store().await;
    let payload = json!({
        "endpoint": "worker.run",
        "worker_id": "w-run-2",
        "worker_name": "Mariah",
        "trigger_id": "trig-xyz",
    });
    let id = seed_and_id(&store, payload).await;
    let resp = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    let body = parse_body(&resp);
    let anchor = body["detail"]["provenance"]["hash_chain_anchor"]
        .as_str()
        .expect("anchor present");
    assert!(!anchor.is_empty(), "hash_chain_anchor must be non-empty");
}
