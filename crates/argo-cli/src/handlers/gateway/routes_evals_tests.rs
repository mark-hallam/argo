//! Tests for `routes_evals` — the real, store-backed per-worker eval
//! dashboard (a worker's evals = the reeval history of its learned
//! skills).
//!
//! Mirrors the `routes_worker_skills_tests` harness: env-lock,
//! tempdir-pinned `ARGO_HOME`, cwd-derived workspace tenant so the
//! handler's `ensure_workspace` resolution is deterministic. Seeds the
//! lazily-applied `skill_reeval_history` table directly (it is not in
//! the migration registry; the handler ensures it before reading).

#![allow(clippy::too_many_lines)]

use super::list_response;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

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
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store
        .ensure_workspace(name, &root)
        .await
        .expect("ensure workspace");
    tenant
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    let mut worker = Worker::draft(
        tenant,
        "support",
        Persona::named(name, "human:mark"),
        Role::new("Resolve customer questions."),
    );
    worker.trust_grade = 3;
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

/// Ensure the lazily-applied table exists in the test store, then
/// insert one reeval row keyed by `skill_id` (the same TEXT UUID the
/// `worker_skill` overlay holds).
#[allow(clippy::too_many_arguments)]
async fn seed_reeval_row(
    store: &Arc<SessionStore>,
    skill_id: &str,
    name: &str,
    version: &str,
    run_at: i64,
    pass_rate: f64,
    previous: Option<f64>,
    regressed: bool,
) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_reeval_history (\
         eval_run_id TEXT PRIMARY KEY, skill_id TEXT NOT NULL, \
         skill_name TEXT NOT NULL, skill_version TEXT NOT NULL, \
         run_at INTEGER NOT NULL, pass_rate REAL, previous_pass_rate REAL, \
         regressed INTEGER NOT NULL DEFAULT 0)",
    )
    .execute(store.pool())
    .await
    .expect("create reeval table");
    sqlx::query(
        "INSERT INTO skill_reeval_history \
         (eval_run_id, skill_id, skill_name, skill_version, run_at, \
          pass_rate, previous_pass_rate, regressed) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::now_v7().to_string())
    .bind(skill_id)
    .bind(name)
    .bind(version)
    .bind(run_at)
    .bind(pass_rate)
    .bind(previous)
    .bind(if regressed { 1_i64 } else { 0_i64 })
    .execute(store.pool())
    .await
    .expect("insert reeval row");
}

fn get_req(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

// =========================================================================
// Worker WITH a learned skill that has reeval history → real shape
// =========================================================================

#[tokio::test]
async fn worker_with_seeded_skill_returns_real_runs() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;

    // Worker learns a skill (overlay row, TEXT skill_id) ...
    let skill_id = uuid::Uuid::now_v7().to_string();
    IdentityStore::new(&store)
        .add_worker_skill(worker.id, &skill_id, 3)
        .await
        .expect("add worker skill");
    // ... and that skill has two reeval rows (a stable baseline, then a
    // regression). run_at: older first arg < newer.
    seed_reeval_row(
        &store,
        &skill_id,
        "candidate-screening-followup",
        "v4.1.0",
        16_900,
        0.91,
        None,
        false,
    )
    .await;
    seed_reeval_row(
        &store,
        &skill_id,
        "candidate-screening-followup",
        "v4.2.0",
        18_400,
        0.32,
        Some(0.91),
        true,
    )
    .await;

    let path = format!("/api/v1/workers/{}/evals", worker.id);
    let (status, body) = list_response(&worker.id.to_string(), &get_req(&path), Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("parse json");

    assert_eq!(v["workerId"], worker.id.to_string());
    let history = v["history"].as_array().expect("history array");
    assert_eq!(history.len(), 2, "both reeval rows surface: {v}");

    // Newest-first: latest == the regression row (v4.2.0).
    let latest = &v["latest"];
    assert!(!latest.is_null(), "latest must be a real run: {v}");
    assert_eq!(latest["version"], "v4.2.0");
    assert_eq!(latest["decision"], "regressed");
    assert_eq!(latest["status"], "failed");
    assert_eq!(latest["skill"], "candidate-screening-followup");
    assert!(
        (latest["passRate"].as_f64().expect("passRate") - 0.32).abs() < 1e-9,
        "passRate maps from pass_rate: {v}"
    );
    // history[0] == latest (newest), history[1] == the stable baseline.
    assert_eq!(history[0]["version"], "v4.2.0");
    assert_eq!(history[1]["version"], "v4.1.0");
    assert_eq!(history[1]["decision"], "no_baseline");
    assert_eq!(history[1]["status"], "passed");
}

// =========================================================================
// Worker with NO learned skills → empty-but-valid (latest: null)
// =========================================================================

#[tokio::test]
async fn worker_without_skills_returns_empty_valid_shape() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Quiet").await;

    let path = format!("/api/v1/workers/{}/evals", worker.id);
    let (status, body) = list_response(&worker.id.to_string(), &get_req(&path), Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("parse json");
    assert_eq!(v["workerId"], worker.id.to_string());
    assert!(v["latest"].is_null(), "empty worker → latest null: {v}");
    assert_eq!(
        v["history"].as_array().expect("history array").len(),
        0,
        "empty worker → empty history (real empty, not mock): {v}"
    );
}

/// A learned skill that has *no* reeval rows yet is still an honest
/// empty — not a fabricated run.
#[tokio::test]
async fn worker_with_skill_but_no_history_is_empty() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "New").await;
    let skill_id = uuid::Uuid::now_v7().to_string();
    IdentityStore::new(&store)
        .add_worker_skill(worker.id, &skill_id, 3)
        .await
        .expect("add worker skill");

    let path = format!("/api/v1/workers/{}/evals", worker.id);
    let (status, body) = list_response(&worker.id.to_string(), &get_req(&path), Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("parse json");
    assert!(v["latest"].is_null(), "no history → latest null: {v}");
    assert_eq!(v["history"].as_array().expect("history").len(), 0);
}

// =========================================================================
// Tenant scoping + arg validation
// =========================================================================

/// A worker in another tenant must surface as 404 (no existence leak),
/// and its skills' reeval rows must never reach the response.
#[tokio::test]
async fn cross_tenant_worker_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _cwd_tenant = workspace_tenant(&store).await;
    let alien_tenant = TenantId::new();
    let alien = add_worker(&store, alien_tenant, "Alien").await;
    let skill_id = uuid::Uuid::now_v7().to_string();
    IdentityStore::new(&store)
        .add_worker_skill(alien.id, &skill_id, 3)
        .await
        .expect("add alien skill");
    seed_reeval_row(&store, &skill_id, "secret-skill", "v1", 1, 0.9, None, false).await;

    let path = format!("/api/v1/workers/{}/evals", alien.id);
    let (status, _) = list_response(&alien.id.to_string(), &get_req(&path), Some(&store)).await;
    assert_eq!(
        status, "404 Not Found",
        "cross-tenant worker must surface as 404"
    );
}

#[tokio::test]
async fn invalid_worker_id_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let (status, _) = list_response("not-a-uuid", "GET /\r\n\r\n", Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn missing_store_returns_503() {
    let _g = env_lock().await;
    let ghost = uuid::Uuid::now_v7();
    let path = format!("/api/v1/workers/{ghost}/evals");
    let (status, _) = list_response(&ghost.to_string(), &get_req(&path), None).await;
    // `store_unavailable()` returns a 503-class envelope.
    assert!(status.starts_with("503"), "no store → 503, got {status}");
}
