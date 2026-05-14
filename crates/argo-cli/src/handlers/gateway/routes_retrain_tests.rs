//! Tests for `routes_retrain` — Stream J12. Mirrors the
//! `routes_worker_memory_tests` env-lock + tempdir-pinned
//! `ARGO_HOME` pattern (cwd-derived workspace tenant is
//! deterministic that way).

#![allow(clippy::too_many_lines)]

use super::{
    classify_response, commit_response, dispatch, reverse_test_response, undo_response,
};
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

#[path = "routes_retrain_tests_classify.rs"]
mod classify;

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store
        .ensure_workspace(name, &root)
        .await
        .expect("ensure workspace");
    tenant
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named(name, "human:mark"),
        Role::new("Resolve customer questions."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

fn req_with(verb: &str, path: &str, body: &str) -> String {
    format!(
        "{verb} {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

async fn json_of(status: &str, body: &str) -> Value {
    assert!(status.starts_with('2'), "expected 2xx, got {status}: {body}");
    serde_json::from_str(body).expect("json body")
}

// Helper: classify + commit a memory_fact and return retrain_id.
async fn commit_memory_fact(store: &Arc<SessionStore>, worker_id: &str, text: &str) -> String {
    let cl_req = req_with(
        "POST",
        &format!("/api/v1/workers/{worker_id}/retrain/classify"),
        &json!({ "input": text }).to_string(),
    );
    let (cs, cb) = classify_response(worker_id, &cl_req, Some(store)).await;
    assert_eq!(cs, "200 OK", "classify: {cb}");
    let classify_body: Value = serde_json::from_str(&cb).unwrap();
    let commit_body = json!({
        "classification": classify_body["classification"],
        "structural_change": classify_body["structural_change"],
    });
    let co_req = req_with(
        "POST",
        &format!("/api/v1/workers/{worker_id}/retrain/commit"),
        &commit_body.to_string(),
    );
    let (status, body) = commit_response(worker_id, &co_req, Some(store)).await;
    assert_eq!(status, "201 Created", "commit: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    v["retrain_id"].as_str().unwrap().to_string()
}

// =========================================================================

#[tokio::test]
async fn commit_returns_201_with_retrain_id_and_persists_memory_fact() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let retrain_id = commit_memory_fact(&store, &wid, "Acme prefers email").await;
    assert!(Uuid::parse_str(&retrain_id).is_ok());
    // Memory fact was actually persisted to the namespace.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM memory_entry WHERE body = ? AND superseded_by IS NULL",
    )
    .bind("Acme prefers email")
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(count, 1, "memory_fact must persist to memory_entry");
}

#[tokio::test]
async fn commit_unknown_classification_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let body = json!({ "classification": "bogus", "structural_change": { "text": "x" } });
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/commit"),
        &body.to_string(),
    );
    let (status, _) = commit_response(&wid, &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn undo_returns_200_and_tombstones_memory_fact() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let retrain_id = commit_memory_fact(&store, &wid, "Acme wants weekly reports").await;
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/undo"),
        &json!({ "retrain_id": retrain_id }).to_string(),
    );
    let (status, body) = undo_response(&wid, &req, Some(&store)).await;
    let v = json_of(status, &body).await;
    assert_eq!(v["ok"], true);
    assert!(v["undone_at"].is_string());
    // Original fact must now be tombstoned (superseded_by set).
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM memory_entry WHERE body = ? AND superseded_by IS NULL",
    )
    .bind("Acme wants weekly reports")
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(active, 0, "fact must be tombstoned after undo");
}

#[tokio::test]
async fn undo_twice_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let retrain_id = commit_memory_fact(&store, &wid, "double-undo target").await;
    let undo_req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/undo"),
        &json!({ "retrain_id": retrain_id }).to_string(),
    );
    let (s1, _) = undo_response(&wid, &undo_req, Some(&store)).await;
    assert_eq!(s1, "200 OK");
    let (s2, _) = undo_response(&wid, &undo_req, Some(&store)).await;
    assert_eq!(s2, "400 Bad Request", "second undo must be rejected");
}

#[tokio::test]
async fn undo_unknown_retrain_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let ghost = Uuid::now_v7().to_string();
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/undo"),
        &json!({ "retrain_id": ghost }).to_string(),
    );
    let (status, _) = undo_response(&wid, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn reverse_test_returns_default_3_samples_when_omitted() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let retrain_id = commit_memory_fact(&store, &wid, "reverse-test target fact").await;
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/reverse-test"),
        &json!({ "retrain_id": retrain_id }).to_string(),
    );
    let (status, body) = reverse_test_response(&wid, &req, Some(&store)).await;
    let v = json_of(status, &body).await;
    assert!(v["samples"].is_array(), "samples must be array");
    // With no tool_call rows the array is empty but the shape is preserved.
    assert!(v["samples"].as_array().unwrap().len() <= 3);
}

#[tokio::test]
async fn reverse_test_sample_size_out_of_range_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let retrain_id = commit_memory_fact(&store, &wid, "size oob").await;
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/reverse-test"),
        &json!({ "retrain_id": retrain_id, "sample_size": 99 }).to_string(),
    );
    let (status, _) = reverse_test_response(&wid, &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn dispatch_routes_each_subpath_to_the_right_handler() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    // classify via dispatch.
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/classify"),
        &json!({ "input": "Acme is in San Francisco" }).to_string(),
    );
    let Some((status, _)) = dispatch(&req, Some(&store)).await else {
        panic!("dispatch returned None for classify");
    };
    assert_eq!(status, "200 OK");
    // Unknown subpath falls through to None.
    let bad = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/bogus"),
        "",
    );
    assert!(dispatch(&bad, Some(&store)).await.is_none());
    // Non-retrain path is None.
    let off = req_with("POST", &format!("/api/v1/workers/{wid}/other/thing"), "");
    assert!(dispatch(&off, Some(&store)).await.is_none());
}

#[tokio::test]
async fn handlers_return_503_when_store_missing() {
    let _g = env_lock().await;
    let id = Uuid::now_v7().to_string();
    let req = req_with("POST", &format!("/api/v1/workers/{id}/retrain/classify"), r#"{"input":"x"}"#);
    let (s1, _) = classify_response(&id, &req, None).await;
    assert_eq!(s1, "503 Service Unavailable");
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{id}/retrain/commit"),
        r#"{"classification":"memory_fact","structural_change":{"text":"x"}}"#,
    );
    let (s2, _) = commit_response(&id, &req, None).await;
    assert_eq!(s2, "503 Service Unavailable");
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{id}/retrain/undo"),
        &format!(r#"{{"retrain_id":"{}"}}"#, Uuid::now_v7()),
    );
    let (s3, _) = undo_response(&id, &req, None).await;
    assert_eq!(s3, "503 Service Unavailable");
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{id}/retrain/reverse-test"),
        &format!(r#"{{"retrain_id":"{}"}}"#, Uuid::now_v7()),
    );
    let (s4, _) = reverse_test_response(&id, &req, None).await;
    assert_eq!(s4, "503 Service Unavailable");
}

#[tokio::test]
async fn audit_row_emitted_for_commit() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let _ = commit_memory_fact(&store, &wid, "audit-trail").await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE kind = 'http.request' \
           AND json_extract(payload_json, '$.endpoint') = 'worker.retrain.committed'",
    )
    .fetch_one(store.pool()).await.unwrap();
    assert!(count >= 1, "commit must emit a retrain audit row");
}
