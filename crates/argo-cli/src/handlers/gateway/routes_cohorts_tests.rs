//! Tests for `routes_cohorts` — `GET /api/v1/cohorts` and
//! `GET /api/v1/cohorts/<id>` (Stream `mission-skill-cohort-ui`).

#![allow(clippy::too_many_lines)]

use super::{detail_response, list_response};
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
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store
        .ensure_workspace(name, &root)
        .await
        .expect("ensure workspace");
    tenant
}

fn req_list(query: Option<&str>) -> String {
    let suffix = query.map_or(String::new(), |q| format!("?{q}"));
    format!("GET /api/v1/cohorts{suffix} HTTP/1.1\r\nHost: x\r\n\r\n")
}

fn req_detail(cohort_id: &str, query: Option<&str>) -> String {
    let suffix = query.map_or(String::new(), |q| format!("?{q}"));
    format!("GET /api/v1/cohorts/{cohort_id}{suffix} HTTP/1.1\r\nHost: x\r\n\r\n")
}

async fn seed(store: &Arc<SessionStore>, tenant: TenantId, name: &str, vertical: &str) -> Worker {
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

async fn add_skill(store: &Arc<SessionStore>, worker_id: &str, skill: &str) {
    sqlx::query(
        "INSERT INTO worker_skill (worker_id, skill_id, proficiency)
         VALUES (?, ?, ?)
         ON CONFLICT(worker_id, skill_id) DO NOTHING",
    )
    .bind(worker_id)
    .bind(skill)
    .bind(3_i64)
    .execute(store.pool())
    .await
    .expect("worker_skill insert");
}

async fn set_trust(store: &Arc<SessionStore>, worker_id: &str, grade: i64) {
    sqlx::query("UPDATE worker SET trust_grade = ? WHERE id = ?")
        .bind(grade)
        .bind(worker_id)
        .execute(store.pool())
        .await
        .expect("trust update");
}

#[tokio::test]
async fn list_empty_returns_zero_count() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let (status, body) = list_response(&req_list(None), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["cohorts"].as_array().unwrap().len(), 0);
    assert_eq!(v["total_count"], 0);
}

#[tokio::test]
async fn list_groups_workers_by_vertical_and_band() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let w_support = seed(&store, tenant, "Maya", "support").await;
    let w_sales = seed(&store, tenant, "Sam", "sales").await;
    set_trust(&store, &w_support.id.to_string(), 2).await;
    set_trust(&store, &w_sales.id.to_string(), 2).await;
    let (status, body) = list_response(&req_list(None), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).expect("json");
    let cohorts = v["cohorts"].as_array().expect("array");
    assert_eq!(cohorts.len(), 2);
    let ids: Vec<&str> = cohorts
        .iter()
        .filter_map(|c| c["cohort_id"].as_str())
        .collect();
    assert!(ids.contains(&"support::developing"));
    assert!(ids.contains(&"sales::developing"));
}

#[tokio::test]
async fn list_503_when_store_missing() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let (status, body) = list_response(&req_list(None), None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("session store unavailable"));
}

#[tokio::test]
async fn list_cross_tenant_query_returns_403() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let other = TenantId::new();
    let q = format!("tenant_id={other}");
    let (status, body) = list_response(&req_list(Some(&q)), Some(&store)).await;
    assert_eq!(status, "403 Forbidden");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "forbidden");
}

#[tokio::test]
async fn list_invalid_tenant_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let (status, body) =
        list_response(&req_list(Some("tenant_id=not-a-uuid")), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("invalid tenant_id"));
}

#[tokio::test]
async fn detail_returns_members_and_skills() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let w1 = seed(&store, tenant, "Maya", "support").await;
    let w2 = seed(&store, tenant, "Priya", "support").await;
    set_trust(&store, &w1.id.to_string(), 2).await;
    set_trust(&store, &w2.id.to_string(), 2).await;
    add_skill(&store, &w1.id.to_string(), "follow_up").await;
    add_skill(&store, &w2.id.to_string(), "follow_up").await;
    add_skill(&store, &w1.id.to_string(), "qualify").await;

    let (status, body) = detail_response(
        &req_detail("support::developing", None),
        "support::developing",
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["cohort_id"], "support::developing");
    assert_eq!(v["member_count"], 2);
    let members = v["members"].as_array().expect("members");
    assert_eq!(members.len(), 2);
    let skills = v["skills"].as_array().expect("skills");
    let names: Vec<&str> = skills.iter().filter_map(|s| s["skill_name"].as_str()).collect();
    assert!(names.contains(&"follow_up"));
    assert!(names.contains(&"qualify"));
    // follow_up is 100% (2/2); qualify is 50% (1/2). DESC sort → follow_up first.
    assert_eq!(skills[0]["skill_name"], "follow_up");
    assert!((skills[0]["member_coverage_pct"].as_f64().unwrap() - 100.0).abs() < 0.001);
}

#[tokio::test]
async fn detail_404_when_cohort_unknown() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let (status, body) = detail_response(
        &req_detail("support::trusted", None),
        "support::trusted",
        Some(&store),
    )
    .await;
    assert_eq!(status, "404 Not Found");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "not_found");
}

#[tokio::test]
async fn detail_decodes_percent_encoded_separator() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let w = seed(&store, tenant, "Maya", "support").await;
    set_trust(&store, &w.id.to_string(), 2).await;
    // Some clients percent-encode `:` as `%3A`; verify the handler
    // decodes the segment before passing it to the store layer.
    let encoded = "support%3A%3Adeveloping";
    let (status, _body) =
        detail_response(&req_detail(encoded, None), encoded, Some(&store)).await;
    assert_eq!(status, "200 OK");
}
