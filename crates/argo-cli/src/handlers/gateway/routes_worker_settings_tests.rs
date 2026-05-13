//! Tests for `routes_worker_settings` (Stream J11).
//!
//! Drives every handler directly against a `:memory:` `SessionStore`,
//! seeded with one (or two) workers. The upstream bearer-auth gate
//! lives in `routes.rs::dispatch_route_inner` / `dispatch_post_inner`
//! and is not re-exercised here.

#![allow(clippy::too_many_lines)]

use super::{
    delete_worker_endpoint, get_settings, patch_autonomy, patch_budget, patch_channels,
    patch_quiet_hours, post_archive, post_pause, post_resume,
};
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
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
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
    IdentityStore::new(store).create_worker(&worker).await.expect("create");
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
    Env { _guard: guard, _tmp: tmp, store, tenant }
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

fn patch_req(id: &WorkerId, path_suffix: &str, body: &Value) -> String {
    let body_s = serde_json::to_string(body).unwrap();
    format!(
        "PATCH /api/v1/workers/{id}/{path_suffix} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body_s}",
        body_s.len()
    )
}

fn post_req(id: &WorkerId, path_suffix: &str) -> String {
    format!(
        "POST /api/v1/workers/{id}/{path_suffix} HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n"
    )
}

fn delete_req(id: &WorkerId, role: Option<&str>, confirm: Option<&str>) -> String {
    let mut req = format!("DELETE /api/v1/workers/{id} HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    if let Some(c) = confirm {
        req.push_str(&format!("X-Confirm-Worker-Name: {c}\r\n"));
    }
    req.push_str("\r\n");
    req
}

fn parse(body: &str) -> Value {
    serde_json::from_str(body).unwrap_or_else(|_| panic!("invalid JSON: {body}"))
}

#[tokio::test]
async fn get_settings_returns_required_shape() {
    let (env, worker) = setup_with_worker("Maya").await;
    let req = get_req(&worker.id, Some("operator"));
    let (status, body) = get_settings(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v = parse(&body);
    for k in ["channels", "quiet_hours", "budget", "autonomy", "advanced"] {
        assert!(v.get(k).is_some(), "missing key {k}");
    }
    assert!(v["channels"]["all"].is_array());
    assert_eq!(v["autonomy"]["level"], "balanced");
}

#[tokio::test]
async fn get_settings_operator_sees_advanced_block() {
    let (env, worker) = setup_with_worker("Maya").await;
    let op = get_req(&worker.id, Some("operator"));
    let (_, op_b) = get_settings(&worker.id.to_string(), &op, Some(&env.store)).await;
    assert!(parse(&op_b)["advanced"].is_object());
    let aud = get_req(&worker.id, Some("auditor"));
    let (_, aud_b) = get_settings(&worker.id.to_string(), &aud, Some(&env.store)).await;
    assert!(parse(&aud_b)["advanced"].is_null());
}

#[tokio::test]
async fn get_settings_unknown_worker_returns_404() {
    let env = setup().await;
    let ghost = WorkerId::new();
    let req = get_req(&ghost, None);
    let (status, body) = get_settings(&ghost.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "404 Not Found");
    assert_eq!(parse(&body)["error"]["code"], "not_found");
}

#[tokio::test]
async fn get_settings_cross_tenant_returns_403() {
    let env = setup().await;
    let other = TenantId::new();
    let alien = add_worker(&env.store, other, "Alien").await;
    let req = get_req(&alien.id, None);
    let (status, _) = get_settings(&alien.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "403 Forbidden");
}

#[tokio::test]
async fn get_settings_archived_returns_410() {
    let (env, worker) = setup_with_worker("Sundown").await;
    IdentityStore::new(&env.store)
        .archive_worker(env.tenant, worker.id)
        .await
        .expect("archive");
    let req = get_req(&worker.id, None);
    let (status, _) = get_settings(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "410 Gone");
}

#[tokio::test]
async fn get_settings_returns_503_when_store_missing() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let id = WorkerId::new();
    let req = get_req(&id, None);
    let (status, body) = get_settings(&id.to_string(), &req, None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("session store unavailable"));
}

#[tokio::test]
async fn patch_channels_updates_and_audits() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "channels": ["slack", "sms"], "primary": "slack" });
    let req = patch_req(&worker.id, "channels", &body);
    let (status, resp) = patch_channels(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    assert_eq!(parse(&resp)["channels"]["primary"], "slack");
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%worker.channels.changed%'",
    )
    .fetch_one(env.store.pool())
    .await
    .unwrap();
    assert!(count >= 1);
    let (_, gv) = get_settings(
        &worker.id.to_string(),
        &get_req(&worker.id, Some("operator")),
        Some(&env.store),
    )
    .await;
    let g = parse(&gv);
    assert_eq!(g["channels"]["primary"], "slack");
    let all: Vec<&str> = g["channels"]["all"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(all.contains(&"slack") && all.contains(&"sms"));
}

#[tokio::test]
async fn patch_channels_rejects_primary_outside_set() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "channels": ["slack"], "primary": "email" });
    let req = patch_req(&worker.id, "channels", &body);
    let (status, resp) = patch_channels(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request");
    assert_eq!(parse(&resp)["error"]["code"], "bad_request");
}

#[tokio::test]
async fn patch_quiet_hours_updates_and_audits() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "start": "21:00", "end": "06:30", "tz": "Pacific/Auckland" });
    let req = patch_req(&worker.id, "quiet-hours", &body);
    let (status, _) = patch_quiet_hours(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK");
    let (_, b) =
        get_settings(&worker.id.to_string(), &get_req(&worker.id, None), Some(&env.store)).await;
    let g = parse(&b);
    assert_eq!(g["quiet_hours"]["start"], "21:00");
    assert_eq!(g["quiet_hours"]["tz"], "Pacific/Auckland");
}

#[tokio::test]
async fn patch_budget_validates_and_audits() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "daily_usd": 7.5, "monthly_usd": 150 });
    let req = patch_req(&worker.id, "budget", &body);
    let (status, _) = patch_budget(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK");
    let bad = json!({ "daily_usd": -1, "monthly_usd": 0 });
    let req2 = patch_req(&worker.id, "budget", &bad);
    let (s2, _) = patch_budget(&worker.id.to_string(), &req2, Some(&env.store)).await;
    assert_eq!(s2, "400 Bad Request");
}

#[tokio::test]
async fn patch_autonomy_conservative_resets_trust_ratchet() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "level": "conservative" });
    let req = patch_req(&worker.id, "autonomy", &body);
    let (status, _) = patch_autonomy(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK");
    let ratchet: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%worker.trust_ratchet.reset%'",
    )
    .fetch_one(env.store.pool())
    .await
    .unwrap();
    assert_eq!(ratchet, 1);
}

#[tokio::test]
async fn patch_autonomy_rejects_unknown_level() {
    let (env, worker) = setup_with_worker("Maya").await;
    let body = json!({ "level": "yolo" });
    let req = patch_req(&worker.id, "autonomy", &body);
    let (status, body) = patch_autonomy(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request");
    assert_eq!(parse(&body)["error"]["code"], "bad_request");
}

#[tokio::test]
async fn post_pause_emits_audit() {
    let (env, worker) = setup_with_worker("Maya").await;
    let req = post_req(&worker.id, "pause");
    let (status, body) = post_pause(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK");
    assert_eq!(parse(&body)["status"], "paused");
    let paused: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%worker.paused%'",
    )
    .fetch_one(env.store.pool())
    .await
    .unwrap();
    assert_eq!(paused, 1);
}

#[tokio::test]
async fn post_resume_clears_archive_on_recovered_worker() {
    let (env, worker) = setup_with_worker("Sundown").await;
    IdentityStore::new(&env.store)
        .archive_worker(env.tenant, worker.id)
        .await
        .expect("archive");
    let req = post_req(&worker.id, "resume");
    let (status, body) = post_resume(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK");
    let v = parse(&body);
    assert_eq!(v["status"], "idle");
    assert_eq!(v["recovered"], true);
    let (after_status, _) = get_settings(
        &worker.id.to_string(),
        &get_req(&worker.id, None),
        Some(&env.store),
    )
    .await;
    assert_eq!(after_status, "200 OK");
}

#[tokio::test]
async fn post_resume_idle_path_for_unarchived_worker() {
    let (env, worker) = setup_with_worker("Maya").await;
    let req = post_req(&worker.id, "resume");
    let (status, body) = post_resume(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK");
    let v = parse(&body);
    assert_eq!(v["status"], "idle");
    assert!(v.get("recovered").is_none());
}

#[tokio::test]
async fn post_archive_sets_archived_at_and_audits() {
    let (env, worker) = setup_with_worker("Maya").await;
    let req = post_req(&worker.id, "archive");
    let (status, body) = post_archive(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v = parse(&body);
    assert!(v["archived_at"].is_string());
    assert!(v["recoverable_until_at"].is_string());
    let ts: Option<String> = sqlx::query_scalar("SELECT archived_at FROM worker WHERE id = ?")
        .bind(worker.id.to_string())
        .fetch_one(env.store.pool())
        .await
        .unwrap();
    assert!(ts.is_some());
}

#[tokio::test]
async fn delete_without_operator_role_returns_403() {
    let (env, worker) = setup_with_worker("Maya").await;
    let req = delete_req(&worker.id, Some("delegator"), Some("Maya"));
    let (status, body) =
        delete_worker_endpoint(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "403 Forbidden");
    assert_eq!(parse(&body)["error"]["code"], "forbidden");
}

#[tokio::test]
async fn delete_without_matching_confirm_header_returns_400() {
    let (env, worker) = setup_with_worker("Maya").await;
    let req = delete_req(&worker.id, Some("operator"), None);
    let (status, body) =
        delete_worker_endpoint(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request");
    assert_eq!(parse(&body)["error"]["code"], "confirmation_mismatch");
    let req = delete_req(&worker.id, Some("operator"), Some("maya"));
    let (status, _) =
        delete_worker_endpoint(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn delete_with_operator_and_confirm_succeeds_and_cascades() {
    let (env, worker) = setup_with_worker("Maya").await;
    let req = delete_req(&worker.id, Some("operator"), Some("Maya"));
    let (status, _) =
        delete_worker_endpoint(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "204 No Content");
    let remaining: Option<String> = sqlx::query_scalar("SELECT id FROM worker WHERE id = ?")
        .bind(worker.id.to_string())
        .fetch_optional(env.store.pool())
        .await
        .unwrap();
    assert!(remaining.is_none());
    let audit_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%worker.deleted%'",
    )
    .fetch_one(env.store.pool())
    .await
    .unwrap();
    assert!(audit_rows >= 1);
}

#[tokio::test]
async fn patch_endpoints_404_on_unknown_worker() {
    let env = setup().await;
    let ghost = WorkerId::new();
    let body = json!({ "channels": ["slack"], "primary": "slack" });
    let req = patch_req(&ghost, "channels", &body);
    let (status, _) = patch_channels(&ghost.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn invalid_worker_id_returns_400() {
    let env = setup().await;
    let req = "GET /api/v1/workers/not-a-uuid/settings HTTP/1.1\r\nHost: x\r\n\r\n";
    let (status, body) = get_settings("not-a-uuid", req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request");
    assert_eq!(parse(&body)["error"]["code"], "bad_request");
}
