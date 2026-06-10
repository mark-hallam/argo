//! Tests for `routes_worker_executor_promote` (UF Phase-B · B3,
//! DEC-037).
//!
//! Drives the handler directly against a `:memory:` `SessionStore`,
//! seeded with one worker plus the capability-engine role defaults.
//! Mirrors the `routes_worker_freeze` harness. The upstream bearer-auth
//! gate lives in `dispatch_post` and is not re-exercised here.

#![allow(clippy::too_many_lines)]

use super::post_executor_promote;
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::executor_promotion;
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{Duration, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::OwnedMutexGuard;

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

fn worker_with_probation(tenant: TenantId, name: &str, probation: Option<Duration>) -> Worker {
    let mut worker = Worker::draft(
        tenant,
        "support",
        Persona::named(name, "human:mark"),
        Role::new("Resolve customer questions."),
    );
    if let Some(window) = probation {
        let until = Utc::now() + window;
        worker.persona.provenance_chain = vec![format!("probation_until={}", until.to_rfc3339())];
    }
    worker
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    add_worker_inner(store, worker_with_probation(tenant, name, None)).await
}

async fn add_worker_inner(store: &Arc<SessionStore>, worker: Worker) -> Worker {
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create");
    worker
}

struct Env {
    _guard: OwnedMutexGuard<()>,
    _tmp: TempDir,
    store: Arc<SessionStore>,
    tenant: TenantId,
}

async fn setup_with_worker(name: &str) -> (Env, Worker) {
    let (env, worker) = setup_inner(|tenant| worker_with_probation(tenant, name, None)).await;
    (env, worker)
}

async fn setup_inner(make: impl FnOnce(TenantId) -> Worker) -> (Env, Worker) {
    let guard = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker_inner(&store, make(tenant)).await;
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

fn promote_req(id: &WorkerId, action: &str, role: Option<&str>, body: &str) -> String {
    let mut req =
        format!("POST /api/v1/workers/{id}/actions/{action}/executor HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    req.push_str("X-Argo-Principal: human:alice@acme\r\n");
    req.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    req
}

const MCP_BODY: &str = r#"{"kind":"mcp","server":"echo","tool":"echo"}"#;

fn parse(body: &str) -> serde_json::Value {
    serde_json::from_str(body).unwrap_or_else(|_| panic!("invalid JSON: {body}"))
}

async fn promotion_count(env: &Env) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM executor_promotion")
        .fetch_one(env.store.pool())
        .await
        .expect("count")
}

#[tokio::test]
async fn promote_with_owner_returns_200_and_persists() {
    let (env, worker) = setup_with_worker("Maya").await;
    let req = promote_req(&worker.id, "send_email", Some("owner"), MCP_BODY);
    let (status, resp) =
        post_executor_promote(&worker.id.to_string(), "send_email", &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v = parse(&resp);
    assert_eq!(v["worker_id"], worker.id.to_string());
    assert_eq!(v["action_type"], "send_email");
    assert_eq!(v["executor"]["kind"], "mcp");
    assert_eq!(v["executor"]["server"], "echo");
    assert_eq!(v["executor"]["tool"], "echo");
    assert!(v["promoted_at"].is_string());

    let row = executor_promotion::get(
        env.store.pool(),
        &env.tenant.to_string(),
        &worker.id.to_string(),
        "send_email",
    )
    .await
    .expect("get")
    .expect("promotion persisted");
    assert_eq!(row.mcp_tool.as_deref(), Some("echo"));
}

#[tokio::test]
async fn promote_without_owner_returns_403_and_rbac_deny() {
    // Admin is below the owner floor for graduation.
    let (env, worker) = setup_with_worker("Maya").await;
    let req = promote_req(&worker.id, "send_email", Some("admin"), MCP_BODY);
    let (status, resp) =
        post_executor_promote(&worker.id.to_string(), "send_email", &req, Some(&env.store)).await;
    assert_eq!(status, "403 Forbidden", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "forbidden");

    let denies: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = 'rbac.deny'")
            .fetch_one(env.store.pool())
            .await
            .expect("count");
    assert_eq!(denies, 1, "expected one RbacDeny audit row");
    assert_eq!(promotion_count(&env).await, 0, "denied call writes no grant");
}

#[tokio::test]
async fn promote_during_probation_returns_409_and_writes_no_grant() {
    let (env, worker) =
        setup_inner(|tenant| worker_with_probation(tenant, "Probie", Some(Duration::hours(1))))
            .await;
    let req = promote_req(&worker.id, "send_email", Some("owner"), MCP_BODY);
    let (status, resp) =
        post_executor_promote(&worker.id.to_string(), "send_email", &req, Some(&env.store)).await;
    assert_eq!(status, "409 Conflict", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "probation_active");
    assert_eq!(
        promotion_count(&env).await,
        0,
        "a worker in probation cannot be graduated"
    );
}

#[tokio::test]
async fn promote_non_mcp_kind_returns_400() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = r#"{"kind":"computer_use","server":"x","tool":"y"}"#;
    let req = promote_req(&worker.id, "send_email", Some("owner"), body);
    let (status, resp) =
        post_executor_promote(&worker.id.to_string(), "send_email", &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "bad_request");
    assert_eq!(promotion_count(&env).await, 0);
}

#[tokio::test]
async fn promote_missing_binding_returns_400() {
    let (env, worker) = setup_with_worker("Maya").await;
    // kind=mcp but no server.
    let body = r#"{"kind":"mcp","tool":"echo"}"#;
    let req = promote_req(&worker.id, "send_email", Some("owner"), body);
    let (status, resp) =
        post_executor_promote(&worker.id.to_string(), "send_email", &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request", "body: {resp}");
    assert_eq!(promotion_count(&env).await, 0);
}

#[tokio::test]
async fn promote_nonexistent_worker_returns_404() {
    let (env, _worker) = setup_with_worker("Maya").await;
    let ghost = WorkerId::new();
    let req = promote_req(&ghost, "send_email", Some("owner"), MCP_BODY);
    let (status, resp) =
        post_executor_promote(&ghost.to_string(), "send_email", &req, Some(&env.store)).await;
    assert_eq!(status, "404 Not Found", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "not_found");
}

#[tokio::test]
async fn promote_cross_tenant_worker_returns_404() {
    // tenant_b cannot graduate tenant_a's worker — `get_worker(tenant,
    // id)` scopes the lookup so a cross-tenant id reads as not-found.
    let (env, _own) = setup_with_worker("Maya").await;
    let other_tenant = TenantId::new();
    let alien = add_worker(&env.store, other_tenant, "Alien").await;
    let req = promote_req(&alien.id, "send_email", Some("owner"), MCP_BODY);
    let (status, resp) =
        post_executor_promote(&alien.id.to_string(), "send_email", &req, Some(&env.store)).await;
    assert_eq!(status, "404 Not Found", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "not_found");
    assert_eq!(promotion_count(&env).await, 0);
}
