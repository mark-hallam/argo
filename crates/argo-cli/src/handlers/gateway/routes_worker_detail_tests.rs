//! Tests for `routes_worker_detail` — `GET /api/v1/workers/<id>` (Stream J7).
//!
//! Pinning `ARGO_HOME` + `ARGO_VERTICALS_DIR` per test under a
//! process-wide mutex keeps the cwd-derived workspace tenant +
//! pending-approval lookup deterministic without colliding with
//! parallel tests in the same crate.

#![allow(clippy::too_many_lines)]

use super::{resolve_permissions, show_response, PrincipalRole};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use uuid::Uuid;

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
    let (tenant, _) = store.ensure_workspace(name, &root).await.expect("ensure workspace");
    tenant
}

fn req_for(id: &str, role: Option<&str>, principal: Option<&str>) -> String {
    let mut req = format!("GET /api/v1/workers/{id} HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    if let Some(p) = principal {
        req.push_str(&format!("X-Argo-Principal: {p}\r\n"));
    }
    req.push_str("\r\n");
    req
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str, builder: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named(name, builder),
        Role::new("Resolve customer questions."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

async fn parse_ok(req: &str, store: &Arc<SessionStore>) -> Value {
    let (status, body) = show_response(extract_id(req), req, Some(store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    serde_json::from_str(&body).expect("json body")
}

fn extract_id(req: &str) -> &str {
    let path = req
        .strip_prefix("GET /api/v1/workers/")
        .expect("/api/v1/workers prefix");
    let head = path.split_once(' ').map_or(path, |(h, _)| h);
    head.split('?').next().unwrap_or(head)
}

#[tokio::test]
async fn happy_path_returns_required_fields() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let v = parse_ok(&req, &store).await;
    assert_eq!(v["id"], worker.id.to_string());
    assert_eq!(v["name"], "Maya");
    assert!(v["avatar_url"].is_null());
    assert_eq!(v["role"]["vertical"], "support");
    assert_eq!(v["role"]["tagline"], "Resolve customer questions.");
    assert_eq!(v["status"], "idle");
    assert!(v["department"].is_null());
    assert!(v["team"].is_null());
    assert_eq!(v["pending_approval_count"], 0);
    assert!(v["permissions"].is_object());
    for k in [
        "can_edit_charter",
        "can_edit_duties",
        "can_pause_resume",
        "can_archive",
        "can_delete",
        "can_view_audit",
        "can_view_settings",
    ] {
        assert!(v["permissions"][k].is_boolean(), "missing key {k}");
    }
}

#[tokio::test]
async fn invalid_worker_id_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let req = req_for("not-a-uuid", None, None);
    let (status, body) = show_response("not-a-uuid", &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn unknown_worker_id_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let ghost = WorkerId::new().to_string();
    let req = req_for(&ghost, None, None);
    let (status, body) = show_response(&ghost, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "not_found");
}

#[tokio::test]
async fn cross_tenant_worker_returns_403() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let other_tenant = TenantId::new();
    let alien = add_worker(&store, other_tenant, "Alien", "human:mark").await;
    let req = req_for(&alien.id.to_string(), None, None);
    let (status, body) = show_response(&alien.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "403 Forbidden", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "forbidden");
}

#[tokio::test]
async fn archived_worker_returns_410_with_recoverable_until() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Sundown", "human:mark").await;
    IdentityStore::new(&store)
        .archive_worker(tenant, worker.id)
        .await
        .expect("archive worker");
    let req = req_for(&worker.id.to_string(), None, None);
    let (status, body) = show_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "410 Gone", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "gone");
    assert!(v["archived_at"].is_string());
    assert!(v["recoverable_until_at"].is_string());
    assert!(v["retry_after_seconds"].is_number());
}

#[tokio::test]
async fn handler_returns_503_when_store_missing() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let id = WorkerId::new().to_string();
    let req = req_for(&id, None, None);
    let (status, body) = show_response(&id, &req, None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("session store unavailable"));
}

#[tokio::test]
async fn operator_role_grants_all_permissions() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:other").await;
    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:elise"));
    let v = parse_ok(&req, &store).await;
    let p = &v["permissions"];
    for k in [
        "can_edit_charter",
        "can_edit_duties",
        "can_pause_resume",
        "can_archive",
        "can_delete",
        "can_view_audit",
        "can_view_settings",
    ] {
        assert_eq!(p[k], Value::Bool(true), "operator should have {k}");
    }
}

#[tokio::test]
async fn auditor_role_grants_only_view_audit() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let req = req_for(&worker.id.to_string(), Some("auditor"), Some("human:auditbot"));
    let v = parse_ok(&req, &store).await;
    let p = &v["permissions"];
    assert_eq!(p["can_view_audit"], Value::Bool(true));
    assert_eq!(p["can_view_settings"], Value::Bool(false));
    for k in [
        "can_edit_charter",
        "can_edit_duties",
        "can_pause_resume",
        "can_archive",
        "can_delete",
    ] {
        assert_eq!(p[k], Value::Bool(false), "auditor should not have {k}");
    }
}

#[tokio::test]
async fn delegator_hire_author_can_edit_but_not_delete() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let req = req_for(&worker.id.to_string(), Some("delegator"), Some("human:mark"));
    let v = parse_ok(&req, &store).await;
    let p = &v["permissions"];
    assert_eq!(p["can_edit_charter"], Value::Bool(true));
    assert_eq!(p["can_edit_duties"], Value::Bool(true));
    assert_eq!(p["can_pause_resume"], Value::Bool(true));
    assert_eq!(p["can_delete"], Value::Bool(false));
    assert_eq!(p["can_view_settings"], Value::Bool(false));
}

#[tokio::test]
async fn delegator_non_hire_author_cannot_edit() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:author").await;
    let req = req_for(&worker.id.to_string(), Some("delegator"), Some("human:bystander"));
    let v = parse_ok(&req, &store).await;
    let p = &v["permissions"];
    assert_eq!(p["can_edit_charter"], Value::Bool(false));
    assert_eq!(p["can_edit_duties"], Value::Bool(false));
    assert_eq!(p["can_archive"], Value::Bool(false));
    assert_eq!(p["can_view_audit"], Value::Bool(false));
}

#[tokio::test]
async fn audit_row_written_per_view() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let req = req_for(&worker.id.to_string(), Some("operator"), Some("human:elise"));
    let _ = parse_ok(&req, &store).await;
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE kind = 'http.request' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .expect("audit row");
    assert!(
        payload.contains("worker.detail.viewed"),
        "audit payload should name the endpoint; got: {payload}"
    );
    assert!(
        payload.contains("\"viewer_role\":\"operator\""),
        "audit payload should record viewer_role; got: {payload}"
    );
}

#[tokio::test]
async fn resolve_permissions_role_table_is_exhaustive() {
    let ops = resolve_permissions(PrincipalRole::Operator, false);
    assert!(ops.can_delete && ops.can_view_settings);
    let audit = resolve_permissions(PrincipalRole::Auditor, false);
    assert!(audit.can_view_audit);
    assert!(!audit.can_edit_charter && !audit.can_delete && !audit.can_view_settings);
    let compliance = resolve_permissions(PrincipalRole::ComplianceOfficer, false);
    assert!(compliance.can_view_audit && compliance.can_archive);
    assert!(!compliance.can_edit_charter && !compliance.can_delete);
    let deleg_author = resolve_permissions(PrincipalRole::Delegator, true);
    assert!(deleg_author.can_edit_charter && deleg_author.can_archive);
    assert!(!deleg_author.can_delete && !deleg_author.can_view_settings);
    let deleg_other = resolve_permissions(PrincipalRole::Delegator, false);
    assert!(!deleg_other.can_edit_charter && !deleg_other.can_view_audit);
}

#[tokio::test]
async fn dispatch_path_ignores_query_string() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let id_with_q = format!("{}?ts=1", worker.id);
    let req = format!(
        "GET /api/v1/workers/{id_with_q} HTTP/1.1\r\nHost: x\r\nX-Argo-Role: operator\r\n\r\n"
    );
    let (status, body) = show_response(&id_with_q, &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["id"], worker.id.to_string());
}

#[tokio::test]
async fn unknown_id_in_random_uuid_is_404_not_403() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    // A bare-UUID that no row references → 404, not 403.
    let id = Uuid::now_v7().to_string();
    let req = req_for(&id, None, None);
    let (status, _body) = show_response(&id, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}
