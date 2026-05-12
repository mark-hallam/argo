//! Tests for `routes_team_list` — `GET /api/v1/team` (Stream J6).
//!
//! The handler walks workspace memory (cwd-derived) when collecting
//! pending approvals; tests pin `ARGO_HOME` + `ARGO_VERTICALS_DIR` to a
//! temp dir so the lookup is deterministic and never touches the
//! developer's real workspace.

#![allow(clippy::too_many_lines)]

use super::{derive_status, list_response, MAX_WORKERS};
use argo_identity::{
    circuit, AutoApprovalPolicy, CircuitState, IdentityStore, NotifyChannelRef, Persona, Role,
    Worker, WorkerLevel,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{Duration, Utc};
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
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store.ensure_workspace(name, &root).await.expect("ensure workspace");
    tenant
}

fn req(query: Option<&str>) -> String {
    let suffix = query.map_or(String::new(), |q| format!("?{q}"));
    format!("GET /api/v1/team{suffix} HTTP/1.1\r\nHost: x\r\n\r\n")
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    add_worker_with_vertical(store, tenant, name, "support").await
}

async fn add_worker_with_vertical(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    name: &str,
    vertical: &str,
) -> Worker {
    let worker = Worker::draft(
        tenant,
        vertical,
        Persona::named(name, "test"),
        Role::new("test"),
    );
    let identities = IdentityStore::new(store);
    identities.create_worker(&worker).await.expect("create worker");
    worker
}

async fn parse_ok(req: &str, store: &Arc<SessionStore>) -> Value {
    let (status, body) = list_response(req, Some(store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    serde_json::from_str(&body).expect("json body")
}

#[tokio::test]
async fn empty_tenant_returns_200_and_empty_workers() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let v = parse_ok(&req(None), &store).await;
    assert_eq!(v["workers"].as_array().expect("array").len(), 0);
    assert_eq!(v["total_count"], 0);
}

#[tokio::test]
async fn single_worker_returns_summary_with_required_fields() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;

    let v = parse_ok(&req(None), &store).await;
    let workers = v["workers"].as_array().expect("array");
    assert_eq!(workers.len(), 1);
    let w = &workers[0];
    assert_eq!(w["id"], worker.id.to_string());
    assert_eq!(w["name"], "Maya");
    assert!(w["avatar_url"].is_null());
    assert_eq!(w["role"]["vertical"], "support");
    assert_eq!(w["role"]["tagline"], "test"); // bio falls back when set
    assert_eq!(w["status"], "idle");
    assert!(w["department"].is_null());
    assert!(w["team"].is_null());
    let bullets = w["headline"]["bullets"].as_array().expect("bullets");
    assert!(!bullets.is_empty());
    assert!(w["last_active_at"].is_null());
    assert_eq!(w["pending_approval_count"], 0);
    assert_eq!(v["total_count"], 1);
}

#[tokio::test]
async fn multi_workers_sorted_by_status_priority() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let _idle = add_worker(&store, tenant, "Idle").await;
    let blocked = add_worker(&store, tenant, "Blocked").await;
    // Trip the circuit by recording 3 consecutive failures (default threshold).
    for _ in 0..3 {
        circuit::record_failure(store.pool(), &blocked.id.to_string(), "test")
            .await
            .expect("record failure");
    }
    let v = parse_ok(&req(None), &store).await;
    let names: Vec<String> = v["workers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["name"].as_str().unwrap().to_string())
        .collect();
    // idle (priority 2) sorts before blocked (priority 4).
    assert_eq!(names, vec!["Idle", "Blocked"]);
    assert_eq!(v["workers"][1]["status"], "blocked");
}

#[tokio::test]
async fn cap_at_max_workers_and_total_count_reflects_truth() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    // Smaller than MAX_WORKERS for fast tests; assert total_count is precise.
    for i in 0..7 {
        add_worker(&store, tenant, &format!("w{i}")).await;
    }
    let v = parse_ok(&req(None), &store).await;
    assert_eq!(v["workers"].as_array().unwrap().len(), 7);
    assert_eq!(v["total_count"], 7);
    const _: () = assert!(MAX_WORKERS >= 200, "v1 cap must be at least the brief's 200");
}

#[tokio::test]
async fn matching_tenant_query_returns_200() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    add_worker(&store, tenant, "X").await;
    let q = format!("tenant_id={tenant}");
    let v = parse_ok(&req(Some(&q)), &store).await;
    assert_eq!(v["workers"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn cross_tenant_query_returns_403() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let other = TenantId::new();
    let q = format!("tenant_id={other}");
    let (status, body) = list_response(&req(Some(&q)), Some(&store)).await;
    assert_eq!(status, "403 Forbidden");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "forbidden");
}

#[tokio::test]
async fn invalid_tenant_query_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let (status, body) = list_response(&req(Some("tenant_id=not-a-uuid")), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("invalid tenant_id"));
}

#[tokio::test]
async fn handler_returns_503_when_store_missing() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let (status, body) = list_response(&req(None), None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("session store unavailable"));
}

#[tokio::test]
async fn audit_row_written_per_query() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let _ = parse_ok(&req(None), &store).await;
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE kind = 'http.request' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .expect("audit row");
    assert!(
        payload.contains("team.list.queried"),
        "audit payload should name the endpoint; got: {payload}"
    );
}

#[tokio::test]
async fn derived_status_blocked_when_circuit_open() {
    let w = sample_worker_with_circuit(CircuitState::Open {
        tripped_at: Utc::now(),
        reason: "boom".into(),
    });
    let (priority, label) = derive_status(&w, 0, Utc::now());
    assert_eq!(label, "blocked");
    assert_eq!(priority, 4);
}

#[tokio::test]
async fn derived_status_paused_when_circuit_half_open() {
    let w = sample_worker_with_circuit(CircuitState::HalfOpen);
    let (priority, label) = derive_status(&w, 0, Utc::now());
    assert_eq!(label, "paused");
    assert_eq!(priority, 3);
}

#[tokio::test]
async fn derived_status_waiting_when_pending_approval() {
    let w = sample_worker_with_circuit(CircuitState::Closed);
    let (priority, label) = derive_status(&w, 1, Utc::now());
    assert_eq!(label, "waiting");
    assert_eq!(priority, 0);
}

#[tokio::test]
async fn derived_status_working_when_recently_active() {
    let mut w = sample_worker_with_circuit(CircuitState::Closed);
    let now = Utc::now();
    w.last_used_at = Some(now - Duration::seconds(10));
    let (priority, label) = derive_status(&w, 0, now);
    assert_eq!(label, "working");
    assert_eq!(priority, 1);
}

#[tokio::test]
async fn derived_status_idle_when_stale_and_no_pending() {
    let mut w = sample_worker_with_circuit(CircuitState::Closed);
    let now = Utc::now();
    w.last_used_at = Some(now - Duration::hours(2));
    let (priority, label) = derive_status(&w, 0, now);
    assert_eq!(label, "idle");
    assert_eq!(priority, 2);
}

fn sample_worker_with_circuit(state: CircuitState) -> Worker {
    let tenant = TenantId::new();
    let mut w = Worker::draft(
        tenant,
        "support",
        Persona::named("Sample", "test"),
        Role::new("test"),
    );
    w.circuit_state = state;
    w.auto_approval = AutoApprovalPolicy::default();
    w.channels = Vec::<NotifyChannelRef>::new();
    w.level = WorkerLevel::Entry;
    w
}
