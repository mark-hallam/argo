//! Tests for `routes_worker_run` (fix-worker-detail-run-now-action-and-endpoint).
//!
//! Drives the handler directly against a `:memory:` `SessionStore`,
//! seeded with one worker plus the capability-engine role defaults. The
//! upstream bearer-auth gate lives in `dispatch_post` and is not
//! re-exercised here.

#![allow(clippy::too_many_lines)]

use super::post_run;
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::MutexGuard;

/// Env vars this suite mutates — snapshotted + restored by the test
/// guards so a panic can't leak state into the next test.
const TOUCHED_ENV: &[&str] = &[
    "ARGO_HOME",
    "ARGO_VERTICALS_DIR",
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "ARGO_WORKER_RUN_PROVIDER",
];

async fn env_lock() -> MutexGuard<'static, ()> {
    // SHARED with the sibling `routes_worker_run_exec::tests` suite —
    // both mutate process-global env + cwd, so they serialize on ONE
    // mutex (a per-file mutex would let them stomp each other).
    super::TEST_ENV_LOCK.lock().await
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// Snapshot every env var in [`TOUCHED_ENV`] for later restore.
fn snapshot_env() -> Vec<(&'static str, Option<String>)> {
    TOUCHED_ENV
        .iter()
        .map(|&k| (k, std::env::var(k).ok()))
        .collect()
}

/// Restore env to a prior snapshot.
fn restore_env(saved: &[(&'static str, Option<String>)]) {
    for (k, v) in saved {
        match v {
            Some(val) => std::env::set_var(k, val),
            None => std::env::remove_var(k),
        }
    }
}

/// A stable, always-present directory to restore cwd to (NEVER a
/// possibly-deleted tempdir captured from `current_dir()`).
fn stable_cwd() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

/// A real, loadable vertical from the workspace `verticals/` dir, so a
/// worker created with it passes the run handler's
/// validate-before-claim vertical check. `"support"` (the old draft
/// default) does NOT exist on disk, so the runnable-path tests must use
/// this id.
const RUNNABLE_VERTICAL: &str = "customer-support";

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store.ensure_workspace(name, &root).await.expect("ws");
    tenant
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        RUNNABLE_VERTICAL,
        Persona::named(name, "human:mark"),
        Role::new("Resolve customer questions."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create");
    worker
}

/// Test environment for the run handler. Holds the shared env mutex +
/// tempdir, snapshots [`TOUCHED_ENV`], and on drop restores cwd to a
/// STABLE path + every touched env var to its pre-test value.
///
/// The handler now runs a REAL turn, so `setup_with_worker` arms the
/// preconditions hermetically: it `chdir`s into the tempdir FIRST (so
/// the cwd-tenant the handler resolves matches the tenant the worker is
/// created under, AND any `process_job` artifact writes land in the
/// tempdir), sets a present provider key, and pins the run to the
/// deterministic `mock` provider via the `ARGO_WORKER_RUN_PROVIDER`
/// seam — no network LLM call.
struct Env {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    store: Arc<SessionStore>,
    tenant: TenantId,
    saved_env: Vec<(&'static str, Option<String>)>,
}

impl Drop for Env {
    fn drop(&mut self) {
        // Restore cwd to a stable, always-present dir (never the
        // tempdir that is about to be removed), then restore env.
        let _ = std::env::set_current_dir(stable_cwd());
        restore_env(&self.saved_env);
    }
}

async fn setup_with_worker(name: &str) -> (Env, Worker) {
    let guard = env_lock().await;
    let saved_env = snapshot_env();
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    // Arm a runnable, hermetic environment (see `Env` docs).
    std::env::set_current_dir(tmp.path()).expect("chdir tmp");
    std::env::set_var("ANTHROPIC_API_KEY", "test-key-present");
    std::env::set_var("ARGO_WORKER_RUN_PROVIDER", "mock");
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, name).await;
    (
        Env {
            _guard: guard,
            _tmp: tmp,
            store,
            tenant,
            saved_env,
        },
        worker,
    )
}

fn run_req(id: &WorkerId, role: Option<&str>, body: &str) -> String {
    let mut req = format!("POST /api/v1/workers/{id}/run HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    req.push_str("X-Argo-Principal: human:alice@acme\r\n");
    req.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    req
}

fn parse(body: &str) -> serde_json::Value {
    serde_json::from_str(body).unwrap_or_else(|_| panic!("invalid JSON: {body}"))
}

#[tokio::test]
async fn run_with_capability_returns_200_with_action_id() {
    let (env, worker) = setup_with_worker("Mariah").await;
    let body = r#"{"note":"demo run","trigger_source":"manual_run_now"}"#;
    let req = run_req(&worker.id, Some("owner"), body);
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v = parse(&resp);
    // Backward-compatible fields the SPA toast deep-links on.
    let action_id = v["action_id"].as_str().expect("action_id present");
    assert_eq!(action_id.len(), 32, "action_id should be 32-char hex");
    assert!(action_id.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(v["audit_id"], v["action_id"], "audit_id mirrors action_id");
    assert!(v["trigger_id"].is_string());
    assert!(v["queued_at"].is_string());
    // New truthful fields: a REAL turn ran to completion.
    assert_eq!(v["status"], "completed", "a real turn completed");
    assert!(
        v["session_id"].as_str().is_some_and(|s| !s.is_empty()),
        "a real session id is reported"
    );
    assert!(
        v["chars"].as_u64().is_some_and(|c| c > 0),
        "the assistant reply has non-zero length"
    );
}

#[tokio::test]
async fn run_emits_audit_row_with_worker_run_endpoint() {
    let (env, worker) = setup_with_worker("Mariah").await;
    let req = run_req(&worker.id, Some("owner"), "{}");
    let (status, _) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK");
    let rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%worker.run%'",
    )
    .fetch_one(env.store.pool())
    .await
    .expect("count");
    assert_eq!(rows, 1, "expected exactly one worker.run audit row");
}

#[tokio::test]
async fn run_response_audit_id_resolves_to_real_row() {
    let (env, worker) = setup_with_worker("Mariah").await;
    let req = run_req(&worker.id, Some("owner"), "{}");
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v = parse(&resp);
    let audit_hex = v["audit_id"].as_str().expect("audit_id");
    let id_bytes = hex::decode(audit_hex).expect("hex");
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE id = ?")
            .bind(id_bytes)
            .fetch_one(env.store.pool())
            .await
            .expect("count");
    assert_eq!(count, 1, "audit_id must resolve to the inserted row");
}

#[tokio::test]
async fn run_without_capability_returns_403() {
    let (env, worker) = setup_with_worker("Mariah").await;
    let req = run_req(&worker.id, Some("user"), "{}");
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "403 Forbidden", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "forbidden");
}

#[tokio::test]
async fn run_unknown_worker_returns_404() {
    let _guard = env_lock().await;
    let saved = snapshot_env();
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _tenant = workspace_tenant(&store).await;
    let ghost = WorkerId::new();
    let req = run_req(&ghost, Some("owner"), "{}");
    let (status, resp) = post_run(&ghost.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found", "body: {resp}");
    restore_env(&saved);
}

#[tokio::test]
async fn run_archived_worker_returns_410() {
    let (env, worker) = setup_with_worker("Mariah").await;
    IdentityStore::new(&env.store)
        .archive_worker(env.tenant, worker.id)
        .await
        .expect("archive");
    let req = run_req(&worker.id, Some("owner"), "{}");
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "410 Gone", "body: {resp}");
}

#[tokio::test]
async fn run_invalid_body_returns_400() {
    let (env, worker) = setup_with_worker("Mariah").await;
    let req = run_req(&worker.id, Some("owner"), "{not json");
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "bad_request");
}

#[tokio::test]
async fn run_empty_body_treated_as_default_trigger_source() {
    // Empty body → the handler derives the default standing-task prompt
    // and still runs a real turn to completion.
    let (env, worker) = setup_with_worker("Mariah").await;
    let req = run_req(&worker.id, Some("owner"), "");
    let (status, resp) = post_run(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v = parse(&resp);
    assert!(v["action_id"].is_string());
    assert_eq!(v["status"], "completed");
}

#[tokio::test]
async fn run_no_store_returns_503() {
    let _g = env_lock().await;
    let saved = snapshot_env();
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let id = WorkerId::new();
    let req = run_req(&id, Some("owner"), "{}");
    let (status, body) = post_run(&id.to_string(), &req, None).await;
    assert_eq!(status, "503 Service Unavailable", "body: {body}");
    restore_env(&saved);
}
