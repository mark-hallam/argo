//! Tests for `routes_worker_freeze` (mission-pm-kill-switch-4-gateway).
//!
//! Drives the two handlers directly against a `:memory:` `SessionStore`,
//! seeded with one worker plus the capability-engine role defaults. The
//! upstream bearer-auth gate lives in `dispatch_post` and is not
//! re-exercised here.

#![allow(clippy::too_many_lines)]

use super::{post_freeze, post_unfreeze};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::worker_freeze;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
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
    let (tenant, _) = store.ensure_workspace(name, &root).await.expect("ws");
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
        .expect("create");
    worker
}

struct Env {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    store: Arc<SessionStore>,
    tenant: TenantId,
}

async fn setup_with_worker(name: &str) -> (Env, Worker) {
    let guard = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, name).await;
    (
        Env {
            _guard: guard,
            _tmp: tmp,
            store,
            tenant,
        },
        worker,
    )
}

fn freeze_req(id: &WorkerId, role: Option<&str>, body: &str) -> String {
    let mut req = format!("POST /api/v1/workers/{id}/freeze HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    req.push_str("X-Argo-Principal: human:alice@acme\r\n");
    req.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    req
}

fn unfreeze_req(id: &WorkerId, role: Option<&str>) -> String {
    let mut req = format!("POST /api/v1/workers/{id}/unfreeze HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    req.push_str("X-Argo-Principal: human:alice@acme\r\nContent-Length: 0\r\n\r\n");
    req
}

fn parse(body: &str) -> serde_json::Value {
    serde_json::from_str(body).unwrap_or_else(|_| panic!("invalid JSON: {body}"))
}

#[tokio::test]
async fn freeze_with_capability_returns_200_and_persists() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = r#"{"reason":"incident response"}"#;
    let req = freeze_req(&worker.id, Some("owner"), body);
    let (status, resp) = post_freeze(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v = parse(&resp);
    assert_eq!(v["worker_id"], worker.id.to_string());
    assert_eq!(v["reason"], "incident response");
    assert!(v["frozen_at"].is_string());

    let active = worker_freeze::is_frozen(
        env.store.pool(),
        &env.tenant.to_string(),
        &worker.id.to_string(),
    )
    .await
    .expect("is_frozen");
    assert!(active.is_some(), "freeze row should be active");
}

#[tokio::test]
async fn freeze_without_capability_returns_403_and_emits_rbac_deny() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = r#"{"reason":"investigate"}"#;
    let req = freeze_req(&worker.id, Some("user"), body);
    let (status, resp) = post_freeze(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "403 Forbidden", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "forbidden");

    let denies: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = 'rbac.deny'")
            .fetch_one(env.store.pool())
            .await
            .expect("count");
    assert_eq!(denies, 1, "expected one RbacDeny audit row");

    // The store primitive must NOT have run.
    let active = worker_freeze::is_frozen(
        env.store.pool(),
        &env.tenant.to_string(),
        &worker.id.to_string(),
    )
    .await
    .expect("is_frozen");
    assert!(active.is_none(), "denied call must not freeze the worker");
}

#[tokio::test]
async fn unfreeze_reverses_freeze() {
    let (env, worker) = setup_with_worker("Maya").await;
    // Freeze first.
    let freeze_req_body = freeze_req(&worker.id, Some("owner"), r#"{"reason":"first"}"#);
    let (status, _) =
        post_freeze(&worker.id.to_string(), &freeze_req_body, Some(&env.store)).await;
    assert_eq!(status, "200 OK");

    let req = unfreeze_req(&worker.id, Some("owner"));
    let (status, resp) = post_unfreeze(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v = parse(&resp);
    assert_eq!(v["worker_id"], worker.id.to_string());
    assert_eq!(v["unfrozen"], true);

    let active = worker_freeze::is_frozen(
        env.store.pool(),
        &env.tenant.to_string(),
        &worker.id.to_string(),
    )
    .await
    .expect("is_frozen");
    assert!(active.is_none(), "row should be unfrozen");
}

#[tokio::test]
async fn freeze_with_empty_reason_returns_400() {
    let (env, worker) = setup_with_worker("Maya").await;
    // Empty `reason` value.
    let body = r#"{"reason":""}"#;
    let req = freeze_req(&worker.id, Some("owner"), body);
    let (status, resp) = post_freeze(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "bad_request");

    // Also: empty body.
    let req2 = freeze_req(&worker.id, Some("owner"), "");
    let (status2, resp2) = post_freeze(&worker.id.to_string(), &req2, Some(&env.store)).await;
    assert_eq!(status2, "400 Bad Request", "body: {resp2}");
}

#[tokio::test]
async fn freeze_for_nonexistent_worker_returns_404() {
    let (env, _worker) = setup_with_worker("Maya").await;
    let ghost = WorkerId::new();
    let body = r#"{"reason":"ghost"}"#;
    let req = freeze_req(&ghost, Some("owner"), body);
    let (status, resp) = post_freeze(&ghost.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "404 Not Found", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "not_found");
}
