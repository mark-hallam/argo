//! Tests for the `/settings/<facet>` PATCH path aliases and the
//! permissive body-shape parsing added by
//! `web-gateway-endpoints-worker-settings-patch`.
//!
//! Background. Wave-3 audit found the web settings form calls
//! `PATCH /api/v1/workers/<id>/settings/<facet>` but the dispatcher
//! only matched `PATCH /api/v1/workers/<id>/<facet>`, so every save
//! 404'd and the API client silently fell back to its mock. The fix
//! is a path alias on the dispatcher plus permissive body parsers
//! that accept either the gateway-test field names (`channels`,
//! `tz`, `daily_usd`, `monthly_usd`) or the web-client field names
//! (`enabled`, `timezone`, `dailyCapUsd`, `monthlyCapUsd`).
//!
//! Held in a sibling test module so the original
//! `routes_worker_settings_tests.rs` stays under the 500-LOC ceiling.

#![allow(clippy::too_many_lines)]

use super::{get_settings, patch_budget, patch_channels, patch_quiet_hours};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
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

async fn setup() -> Env {
    let guard = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    Env {
        _guard: guard,
        _tmp: tmp,
        store,
        tenant,
    }
}

async fn setup_with_worker(name: &str) -> (Env, Worker) {
    let env = setup().await;
    let worker = add_worker(&env.store, env.tenant, name).await;
    (env, worker)
}

fn get_req(id: &WorkerId, role: Option<&str>) -> String {
    let mut req = format!("GET /api/v1/workers/{id}/settings HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    req.push_str("\r\n");
    req
}

fn patch_facet_req(id: &WorkerId, facet: &str, body: &Value) -> String {
    let body_s = serde_json::to_string(body).unwrap();
    format!(
        "PATCH /api/v1/workers/{id}/{facet} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body_s}",
        body_s.len()
    )
}

fn patch_settings_req(id: &WorkerId, facet: &str, body: &Value) -> String {
    let body_s = serde_json::to_string(body).unwrap();
    format!(
        "PATCH /api/v1/workers/{id}/settings/{facet} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body_s}",
        body_s.len()
    )
}

fn parse(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|_| panic!("invalid JSON: {body}"))
}

#[tokio::test]
async fn patch_channels_accepts_web_client_enabled_field() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "enabled": ["slack", "email"], "primary": "slack" });
    let req = patch_facet_req(&worker.id, "channels", &body);
    let (status, _) = patch_channels(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK");
    let (_, gv) = get_settings(
        &worker.id.to_string(),
        &get_req(&worker.id, Some("operator")),
        Some(&env.store),
    )
    .await;
    let g = parse(&gv);
    let all: Vec<&str> = g["channels"]["all"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(all.contains(&"slack") && all.contains(&"email"));
}

#[tokio::test]
async fn patch_quiet_hours_accepts_timezone_alias() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "start": "22:00", "end": "07:00", "timezone": "Australia/Melbourne" });
    let req = patch_facet_req(&worker.id, "quiet-hours", &body);
    let (status, _) = patch_quiet_hours(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK");
    let (_, b) = get_settings(
        &worker.id.to_string(),
        &get_req(&worker.id, None),
        Some(&env.store),
    )
    .await;
    let g = parse(&b);
    assert_eq!(g["quiet_hours"]["tz"], "Australia/Melbourne");
    assert_eq!(g["quiet_hours"]["start"], "22:00");
}

#[tokio::test]
async fn patch_budget_accepts_camel_case_caps() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "dailyCapUsd": 9.0, "monthlyCapUsd": 250 });
    let req = patch_facet_req(&worker.id, "budget", &body);
    let (status, _) = patch_budget(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK");
    let (_, b) = get_settings(
        &worker.id.to_string(),
        &get_req(&worker.id, None),
        Some(&env.store),
    )
    .await;
    let g = parse(&b);
    assert_eq!(g["budget"]["daily_usd"], 9.0);
    assert_eq!(g["budget"]["monthly_usd"], 250.0);
}

#[tokio::test]
async fn patch_budget_rejects_missing_caps_with_either_naming() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "dailyCapUsd": 5.0 });
    let req = patch_facet_req(&worker.id, "budget", &body);
    let (status, _) = patch_budget(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn dispatch_patch_routes_settings_channels_alias() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "enabled": ["slack"], "primary": "slack" });
    let req = patch_settings_req(&worker.id, "channels", &body);
    let resp =
        super::super::routes_dispatch_patch::dispatch_patch(&req, Some(&env.store)).await;
    let (status, _body) = resp.expect("dispatcher must route /settings/channels");
    assert_eq!(status, "200 OK");
}

#[tokio::test]
async fn dispatch_patch_routes_settings_quiet_hours_alias() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "start": "22:00", "end": "06:00", "timezone": "UTC" });
    let req = patch_settings_req(&worker.id, "quiet-hours", &body);
    let resp =
        super::super::routes_dispatch_patch::dispatch_patch(&req, Some(&env.store)).await;
    let (status, _) = resp.expect("dispatcher must route /settings/quiet-hours");
    assert_eq!(status, "200 OK");
}

#[tokio::test]
async fn dispatch_patch_routes_settings_budget_alias() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "dailyCapUsd": 5.0, "monthlyCapUsd": 100.0 });
    let req = patch_settings_req(&worker.id, "budget", &body);
    let resp =
        super::super::routes_dispatch_patch::dispatch_patch(&req, Some(&env.store)).await;
    let (status, _) = resp.expect("dispatcher must route /settings/budget");
    assert_eq!(status, "200 OK");
}

#[tokio::test]
async fn dispatch_patch_routes_settings_autonomy_alias() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "level": "balanced" });
    let req = patch_settings_req(&worker.id, "autonomy", &body);
    let resp =
        super::super::routes_dispatch_patch::dispatch_patch(&req, Some(&env.store)).await;
    let (status, _) = resp.expect("dispatcher must route /settings/autonomy");
    assert_eq!(status, "200 OK");
}

#[tokio::test]
async fn dispatch_patch_settings_alias_cross_tenant_returns_403() {
    let env = setup().await;
    let other = TenantId::new();
    let alien = add_worker(&env.store, other, "Alien").await;
    let body = json!({ "level": "balanced" });
    let req = patch_settings_req(&alien.id, "autonomy", &body);
    let resp =
        super::super::routes_dispatch_patch::dispatch_patch(&req, Some(&env.store)).await;
    let (status, _) = resp.expect("dispatcher matched");
    // Cross-tenant returns 403 (not 404). PR #1511 chose explicit
    // "wrong tenant" over existence-hiding; we follow the same
    // convention used by GET /settings — see
    // `routes_worker_settings_helpers::load_worker_by_id`.
    assert_eq!(status, "403 Forbidden");
}

#[tokio::test]
async fn dispatch_patch_unknown_facet_returns_none() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({});
    let req = patch_settings_req(&worker.id, "nope", &body);
    let resp =
        super::super::routes_dispatch_patch::dispatch_patch(&req, Some(&env.store)).await;
    assert!(
        resp.is_none(),
        "unknown facet under /settings must fall through to 404"
    );
}
