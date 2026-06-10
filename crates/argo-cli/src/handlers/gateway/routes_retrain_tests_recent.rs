//! Recent-retrains read-panel tests, split out of
//! `routes_retrain_tests.rs` to keep that file under the 500-LOC
//! budget. Loaded as a child mod via `#[path]` from the parent test
//! file. Tests are reachable as `routes_retrain::tests::recent::*`.

use super::{
    add_worker, json_of, mem_store, recent_response, req_with, set_test_env, workspace_tenant,
    UNDO_KEY,
};
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

// Writes a `worker.retrain.committed` audit row directly + flushes,
// mirroring what `commit_inner` persists. Deterministic for read tests
// (each `emit_audit` call uses its own batched sink we can't flush).
#[allow(clippy::too_many_arguments)]
async fn seed_commit_row(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: &str,
    retrain_id: &str,
    classification: &str,
    structural_change: Value,
    applied_at: &str,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    sink.write(
        None,
        AuditKind::HttpRequest,
        json!({
            "endpoint": "worker.retrain.committed",
            "retrain_id": retrain_id,
            "tenant_id": tenant.to_string(),
            "worker_id": worker_id,
            "classification": classification,
            "structural_change": structural_change,
            "applied_at": applied_at,
        }),
        None,
    )
    .await
    .unwrap();
    sink.flush_pending().await.unwrap();
}

#[tokio::test]
async fn recent_lists_committed_retrains_newest_first_in_spa_shape() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();

    let rid1 = Uuid::now_v7().to_string();
    seed_commit_row(
        &store,
        tenant,
        &wid,
        &rid1,
        "memory_fact",
        json!({ "text": "Acme prefers email" }),
        "2026-05-20T10:00:00+00:00",
    )
    .await;
    let rid2 = Uuid::now_v7().to_string();
    seed_commit_row(
        &store,
        tenant,
        &wid,
        &rid2,
        "vocabulary_patch",
        json!({ "from": "client", "to": "customer" }),
        "2026-05-22T10:00:00+00:00",
    )
    .await;

    let req = req_with("GET", &format!("/api/v1/workers/{wid}/retrain/recent"), "");
    let (status, body) = recent_response(&wid, &req, Some(&store)).await;
    let v = json_of(status, &body).await;
    let arr = v.as_array().expect("array body");
    assert_eq!(arr.len(), 2, "{body}");
    // Newest-first by insert order: the vocabulary patch was written last.
    assert_eq!(arr[0]["classification"], "vocabulary");
    assert!(arr[0]["oneLineSummary"]
        .as_str()
        .unwrap()
        .contains("customer"));
    assert_eq!(arr[0]["retrainId"], rid2);
    assert!(arr[0]["committedAt"]
        .as_str()
        .unwrap()
        .starts_with("2026-05-22"));
    assert!(arr[0]["undoExpiresAt"].is_string());
    assert_eq!(arr[1]["classification"], "memory");
    assert!(arr[1]["oneLineSummary"].as_str().unwrap().contains("Acme"));
}

#[tokio::test]
async fn recent_marks_undone_and_handles_missing_store_and_unknown_worker() {
    // No store → 503 (no env needed).
    let ghost = Uuid::now_v7().to_string();
    let r0 = req_with("GET", &format!("/api/v1/workers/{ghost}/retrain/recent"), "");
    let (s0, _) = recent_response(&ghost, &r0, None).await;
    assert_eq!(s0, "503 Service Unavailable");

    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;

    // Unknown worker → 404.
    let (s404, _) = recent_response(&ghost, &r0, Some(&store)).await;
    assert_eq!(s404, "404 Not Found");

    // Committed then undone → record carries `undoneAt`.
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let rid = Uuid::now_v7().to_string();
    seed_commit_row(
        &store,
        tenant,
        &wid,
        &rid,
        "memory_fact",
        json!({ "text": "fact to undo" }),
        "2026-05-20T10:00:00+00:00",
    )
    .await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
    sink.write(
        None,
        AuditKind::HttpRequest,
        json!({
            "endpoint": UNDO_KEY,
            "retrain_id": rid,
            "tenant_id": tenant.to_string(),
            "worker_id": wid,
            "undone_at": "2026-05-21T10:00:00+00:00",
        }),
        None,
    )
    .await
    .unwrap();
    sink.flush_pending().await.unwrap();

    let req = req_with("GET", &format!("/api/v1/workers/{wid}/retrain/recent"), "");
    let (status, body) = recent_response(&wid, &req, Some(&store)).await;
    let v = json_of(status, &body).await;
    let arr = v.as_array().expect("array body");
    assert_eq!(arr.len(), 1, "{body}");
    assert_eq!(arr[0]["undoneAt"], "2026-05-21T10:00:00+00:00");
}
