//! Tests for `routes_worker_outputs` —
//! `GET /api/v1/workers/<id>/runs/<job_id>/output`.
//!
//! Two load-bearing guarantees are pinned here:
//!
//! 1. **Happy path** — a run's `assistant_output.md` + parsed
//!    `result.json` + `actions.json` are returned for the owning tenant,
//!    read from the bundle dir recorded in the `completed_jobs` memory
//!    row (no disk path comes from the request).
//! 2. **Tenant scoping** — a worker that belongs to another tenant, and
//!    a `job_id` with no completed-run row in this tenant's scope, both
//!    surface a uniform `404` (existence is not leaked).
//!
//! `ARGO_HOME` + `ARGO_VERTICALS_DIR` are pinned per test under the
//! process-wide env lock so the cwd-derived workspace tenant is
//! deterministic and does not collide with parallel tests.

#![allow(clippy::expect_used)]

use super::output_response;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

const VERTICAL: &str = "customer-support";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn set_test_env(tmp: &Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// The cwd-derived workspace + its tenant — what an unauthenticated
/// request resolves to.
async fn workspace_tenant(store: &Arc<SessionStore>) -> (TenantId, WorkspaceId) {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .expect("ensure workspace")
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId) -> Worker {
    let worker = Worker::draft(
        tenant,
        VERTICAL,
        Persona::named("Maya", "human:mark"),
        Role::new("Resolve customer questions."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

/// Materialise a minimal artifact bundle on disk + the `completed_jobs`
/// memory row that points at it. Returns the `job_id` (the
/// `pending_job_id` the route looks up by).
async fn seed_completed_run(
    store: &Arc<SessionStore>,
    workspace: WorkspaceId,
    bundle_dir: &Path,
) -> Uuid {
    let job_id = Uuid::now_v7();
    std::fs::create_dir_all(bundle_dir).expect("bundle dir");
    std::fs::write(
        bundle_dir.join("assistant_output.md"),
        "# Draft listing\n\nHere is the work I produced.",
    )
    .expect("assistant_output");
    std::fs::write(
        bundle_dir.join("result.json"),
        json!({ "stop_reason": "approval_required", "not_ready_to_post": false }).to_string(),
    )
    .expect("result.json");
    std::fs::write(
        bundle_dir.join("actions.json"),
        json!({ "counts": { "total": 2, "pending_approval": 1 }, "actions": [] }).to_string(),
    )
    .expect("actions.json");

    let body = json!({
        "schema_key": "completed_jobs",
        "pending_job_id": job_id.to_string(),
        "vertical_id": VERTICAL,
        "session_id": Uuid::now_v7().to_string(),
        "assistant_text": "Here is the work I produced.",
        "completed_at": "2026-06-11T00:00:00Z",
        "steps_completed": 2,
        "stop_reason": "approval_required",
        "stop_detail": "",
        "output_dir": bundle_dir.to_string_lossy(),
        "actions_emitted": 2,
    });
    MemoryStore::new(Arc::clone(store))
        .append_for_vertical(workspace, MemoryFile::Memory, body.to_string(), None, VERTICAL)
        .await
        .expect("append completed_jobs");
    job_id
}

fn req_for(worker_id: &str, job_id: &str) -> String {
    format!(
        "GET /api/v1/workers/{worker_id}/runs/{job_id}/output HTTP/1.1\r\nHost: x\r\n\r\n"
    )
}

#[tokio::test]
async fn happy_path_returns_assistant_output_and_actions() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (tenant, workspace) = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;
    let job_id = seed_completed_run(&store, workspace, &tmp.path().join("bundle")).await;

    let req = req_for(&worker.id.to_string(), &job_id.to_string());
    let (status, body) = output_response(
        &worker.id.to_string(),
        &job_id.to_string(),
        &req,
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["job_id"], job_id.to_string());
    assert_eq!(v["worker_id"], worker.id.to_string());
    assert_eq!(v["worker_name"], "Maya");
    assert_eq!(v["stop_reason"], "approval_required");
    assert_eq!(v["steps_completed"], 2);
    assert_eq!(v["actions_emitted"], 2);
    assert!(
        v["assistant_output"]
            .as_str()
            .unwrap_or_default()
            .contains("work I produced"),
        "assistant_output missing: {v}"
    );
    assert_eq!(v["result"]["not_ready_to_post"], false);
    assert_eq!(v["actions"]["counts"]["total"], 2);
}

#[tokio::test]
async fn unknown_job_id_is_404() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (tenant, _workspace) = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;

    let job_id = Uuid::now_v7().to_string();
    let req = req_for(&worker.id.to_string(), &job_id);
    let (status, _body) =
        output_response(&worker.id.to_string(), &job_id, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn cross_tenant_worker_is_404() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    // The request resolves to the cwd workspace tenant; the worker (and
    // its seeded run) live under a DIFFERENT tenant + workspace, so the
    // read must 404 — a caller cannot reach another tenant's bundles.
    let (_caller_tenant, _caller_ws) = workspace_tenant(&store).await;
    // `ensure_workspace` derives the tenant from the root path, so a
    // distinct root yields a distinct tenant the worker + run live under.
    let (other_tenant, other_ws) = store
        .ensure_workspace("other", "/tmp/other-tenant-root")
        .await
        .expect("other workspace");
    let worker = add_worker(&store, other_tenant).await;
    let job_id = seed_completed_run(&store, other_ws, &tmp.path().join("other-bundle")).await;

    let req = req_for(&worker.id.to_string(), &job_id.to_string());
    let (status, _body) = output_response(
        &worker.id.to_string(),
        &job_id.to_string(),
        &req,
        Some(&store),
    )
    .await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn invalid_job_id_is_400() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (tenant, _ws) = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;

    let req = req_for(&worker.id.to_string(), "not-a-uuid");
    let (status, _body) =
        output_response(&worker.id.to_string(), "not-a-uuid", &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}
