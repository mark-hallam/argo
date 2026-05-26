//! Test entry point + shared helpers for `routes_worker_overview` — split
//! into scenario groups (status / data / pure-fn) so each file stays under
//! the 500-LOC budget.
//!
//! Scenario modules:
//! - [`status`](routes_worker_overview_tests_status) — error / status-code
//!   response tests (400, 403, 404, 410, 503, query-string handling).
//! - [`data`](routes_worker_overview_tests_data) — DB-driven KPI, duties,
//!   recent-activity, audit, and per-role permission tests.
//! - [`pure`](routes_worker_overview_tests_pure) — pure-function tests on
//!   `classify_source`, `slugify`, `trend_for`, `next_fire_for`, etc.

#![allow(clippy::too_many_lines)]
#![allow(deprecated)]

use super::super::routes_worker_detail::PrincipalRole;
use super::{
    classify_source, label_from_template, next_fire_for, parse_role_header, show_response, slugify,
    trend_for, SourceKind,
};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::{SessionCreate, SessionStore};
use argo_team_headline::bullets_for_vertical;
use argo_types::{SessionId, TenantId, WorkspaceId};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Mutex, MutexGuard};

const VERT: &str = "customer-support";

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
    std::env::remove_var(super::super::demo_mode::DEMO_MODE_ENV);
}

struct Env {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    store: Arc<SessionStore>,
    tenant: TenantId,
    ws: WorkspaceId,
}

async fn prep() -> Env {
    let guard = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, ws) = store
        .ensure_workspace(name, &root)
        .await
        .expect("ensure workspace");
    Env {
        _guard: guard,
        _tmp: tmp,
        store,
        tenant,
        ws,
    }
}

fn req_for(id: &str, role: Option<&str>, principal: Option<&str>) -> String {
    let mut req = format!("GET /api/v1/workers/{id}/overview HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    if let Some(p) = principal {
        req.push_str(&format!("X-Argo-Principal: {p}\r\n"));
    }
    req.push_str("\r\n");
    req
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, builder: &str) -> Worker {
    let mut worker = Worker::draft(
        tenant,
        VERT,
        Persona::named("Maya", builder),
        Role::new("Resolve customer questions."),
    );
    worker.charter = Some("Be the most helpful triage agent on the floor.".into());
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

async fn seed_session(env: &Env, worker: WorkerId) -> SessionId {
    env.store
        .create_session_with_worker(SessionCreate {
            tenant_id: env.tenant,
            workspace_id: env.ws,
            model: "test-model",
            provider: "test-provider",
            title: Some("seed"),
            vertical_id: Some(VERT),
            worker_id: Some(argo_store::WorkerId(worker.0)),
        })
        .await
        .expect("create session")
}

async fn write_audit(sink: &AuditSink, session: SessionId, kind: AuditKind, payload: Value) {
    sink.write(Some(session), kind, payload, None)
        .await
        .expect("audit write");
    sink.flush_pending().await.expect("flush audit");
}

async fn parse_ok(req: &str, id: &str, store: &Arc<SessionStore>) -> Value {
    let (status, body) = show_response(id, req, Some(store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    serde_json::from_str(&body).expect("json body")
}

#[path = "routes_worker_overview_tests_status.rs"]
mod status;

#[path = "routes_worker_overview_tests_data.rs"]
mod data;

#[path = "routes_worker_overview_tests_pure.rs"]
mod pure;
