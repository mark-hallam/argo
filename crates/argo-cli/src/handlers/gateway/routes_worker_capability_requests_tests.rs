//! Tests for `routes_worker_capability_requests` —
//! `GET /api/v1/workers/<id>/capability-requests`.
//!
//! Pins the two load-bearing guarantees (mirroring
//! `routes_worker_outputs_tests`):
//!
//! 1. **Happy path** — a pending request recorded for the worker is
//!    returned for the owning tenant.
//! 2. **Tenant scoping** — a worker that belongs to another tenant, and
//!    an unknown worker, both surface a uniform `404` (existence is not
//!    leaked); an unparseable worker id is a `400`.

#![allow(clippy::expect_used)]

use super::list_response;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

const VERTICAL: &str = "customer-support";

fn set_test_env(tmp: &Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// The cwd-derived workspace tenant — what an unauthenticated request
/// resolves to.
async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let (tenant, _ws) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .expect("ensure workspace");
    tenant
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId) -> Worker {
    let worker = Worker::draft(
        tenant,
        VERTICAL,
        Persona::named("Nora", "human:mark"),
        Role::new("Resolve customer questions."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

async fn seed_request(store: &Arc<SessionStore>, worker: &Worker, tool: &str, access: &str) {
    argo_store::capability_request::record_pending(
        store.pool(),
        &worker.tenant_id.to_string(),
        &worker.id.to_string(),
        tool,
        access,
        Some("run-1"),
    )
    .await
    .expect("record capability_request");
}

fn req_for(worker_id: &str) -> String {
    format!("GET /api/v1/workers/{worker_id}/capability-requests HTTP/1.1\r\nHost: x\r\n\r\n")
}

#[tokio::test]
async fn happy_path_returns_pending_requests() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;
    seed_request(&store, &worker, "salesforce", "write").await;

    let req = req_for(&worker.id.to_string());
    let (status, body) = list_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["worker_id"], worker.id.to_string());
    assert_eq!(v["worker_name"], "Nora");
    let reqs = v["capability_requests"].as_array().expect("array");
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0]["tool_or_connector"], "salesforce");
    assert_eq!(reqs[0]["requested_access"], "write");
    assert_eq!(reqs[0]["status"], "pending");
}

#[tokio::test]
async fn no_requests_returns_empty_list() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;

    let req = req_for(&worker.id.to_string());
    let (status, body) = list_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert!(v["capability_requests"].as_array().expect("array").is_empty());
}

#[tokio::test]
async fn unknown_worker_is_404() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _tenant = workspace_tenant(&store).await;

    let ghost = Uuid::now_v7().to_string();
    let req = req_for(&ghost);
    let (status, _body) = list_response(&ghost, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn cross_tenant_worker_is_404() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    // The request resolves to the cwd workspace tenant; the worker lives
    // under a DIFFERENT tenant, so the read must 404 — a caller cannot
    // reach another tenant's worker requests.
    let _caller = workspace_tenant(&store).await;
    let (other_tenant, _other_ws) = store
        .ensure_workspace("other", "/tmp/other-tenant-root-capreq")
        .await
        .expect("other workspace");
    let worker = add_worker(&store, other_tenant).await;
    seed_request(&store, &worker, "salesforce", "write").await;

    let req = req_for(&worker.id.to_string());
    let (status, _body) = list_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn invalid_worker_id_is_400() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _tenant = workspace_tenant(&store).await;

    let req = req_for("not-a-uuid");
    let (status, _body) = list_response("not-a-uuid", &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}
