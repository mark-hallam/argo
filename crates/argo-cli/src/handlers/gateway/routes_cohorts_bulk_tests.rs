//! Integration tests for `POST /api/v1/cohorts/<id>/bulk-assign-skill`
//! (Stream `mission-cohort-detail-bulk-assign-skill`). Split out of
//! [`super::routes_cohorts_tests`] so both test files stay under the
//! 500-LOC HARD RULE 6 ceiling.

#![allow(clippy::too_many_lines)]

use super::{bulk_assign_skill_response, detail_response};
use argo_identity::{IdentityStore, Persona, Role, Worker};
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

async fn seed(store: &Arc<SessionStore>, tenant: TenantId, name: &str, vertical: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        vertical,
        Persona::named(name, "test"),
        Role::new("test"),
    );
    let identities = IdentityStore::new(store);
    identities
        .create_worker(&worker)
        .await
        .expect("create worker");
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

fn req_bulk_assign(cohort_id: &str, query: Option<&str>, body: &str) -> String {
    let suffix = query.map_or(String::new(), |q| format!("?{q}"));
    format!(
        "POST /api/v1/cohorts/{cohort_id}/bulk-assign-skill{suffix} HTTP/1.1\r\n\
         Host: x\r\nContent-Type: application/json\r\nContent-Length: {len}\r\n\r\n{body}",
        len = body.len()
    )
}

fn req_detail(cohort_id: &str) -> String {
    format!("GET /api/v1/cohorts/{cohort_id} HTTP/1.1\r\nHost: x\r\n\r\n")
}

#[tokio::test]
async fn bulk_assign_happy_path_assigns_new_skill_to_members() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let w1 = seed(&store, tenant, "Maya", "support").await;
    let w2 = seed(&store, tenant, "Priya", "support").await;
    set_trust(&store, &w1.id.to_string(), 2).await;
    set_trust(&store, &w2.id.to_string(), 2).await;
    let body = serde_json::json!({
        "worker_ids": [w1.id.to_string(), w2.id.to_string()],
        "skill_id": "follow_up",
    })
    .to_string();
    let req = req_bulk_assign("support::developing", None, &body);
    let (status, resp_body) =
        bulk_assign_skill_response(&req, "support::developing", Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp_body}");
    let v: Value = serde_json::from_str(&resp_body).expect("json");
    assert_eq!(v["assigned"], 2);
    assert_eq!(v["already_had"], 0);
    assert_eq!(v["errors"].as_array().unwrap().len(), 0);
    // The worker_skill rows now hold the skill — verify via the cohort
    // detail endpoint which reads the same table.
    let detail_req = req_detail("support::developing");
    let (d_status, d_body) =
        detail_response(&detail_req, "support::developing", Some(&store)).await;
    assert_eq!(d_status, "200 OK");
    let d: Value = serde_json::from_str(&d_body).expect("detail json");
    assert_eq!(d["skill_count"], 1);
}

#[tokio::test]
async fn bulk_assign_idempotent_reapplication_counts_already_had() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let w1 = seed(&store, tenant, "Maya", "support").await;
    set_trust(&store, &w1.id.to_string(), 2).await;
    add_skill(&store, &w1.id.to_string(), "follow_up").await;
    let body = serde_json::json!({
        "worker_ids": [w1.id.to_string()],
        "skill_id": "follow_up",
    })
    .to_string();
    let req = req_bulk_assign("support::developing", None, &body);
    let (status, resp_body) =
        bulk_assign_skill_response(&req, "support::developing", Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp_body}");
    let v: Value = serde_json::from_str(&resp_body).expect("json");
    assert_eq!(v["assigned"], 0);
    assert_eq!(v["already_had"], 1);
}

#[tokio::test]
async fn bulk_assign_404_when_cohort_unknown() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let body = serde_json::json!({
        "worker_ids": [Uuid::now_v7().to_string()],
        "skill_id": "follow_up",
    })
    .to_string();
    let req = req_bulk_assign("support::trusted", None, &body);
    let (status, _b) =
        bulk_assign_skill_response(&req, "support::trusted", Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn bulk_assign_400_when_worker_not_in_cohort() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let in_cohort = seed(&store, tenant, "Maya", "support").await;
    let stranger = seed(&store, tenant, "Sam", "sales").await;
    set_trust(&store, &in_cohort.id.to_string(), 2).await;
    set_trust(&store, &stranger.id.to_string(), 2).await;
    let body = serde_json::json!({
        "worker_ids": [in_cohort.id.to_string(), stranger.id.to_string()],
        "skill_id": "follow_up",
    })
    .to_string();
    let req = req_bulk_assign("support::developing", None, &body);
    let (status, resp_body) =
        bulk_assign_skill_response(&req, "support::developing", Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(resp_body.contains("not in cohort"), "body: {resp_body}");
}

/// Load-bearing cross-tenant negative test (per brief). An explicit
/// `?tenant_id=<other>` query is the cheapest way to construct a
/// cross-tenant call inside the test harness; the gateway treats it
/// exactly the same as a session-tenant mismatch (403, no row touched).
#[tokio::test]
async fn bulk_assign_403_on_cross_tenant_query() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let w = seed(&store, tenant, "Maya", "support").await;
    set_trust(&store, &w.id.to_string(), 2).await;
    let body = serde_json::json!({
        "worker_ids": [w.id.to_string()],
        "skill_id": "follow_up",
    })
    .to_string();
    let other = TenantId::new();
    let q = format!("tenant_id={other}");
    let req = req_bulk_assign("support::developing", Some(&q), &body);
    let (status, _b) =
        bulk_assign_skill_response(&req, "support::developing", Some(&store)).await;
    assert_eq!(status, "403 Forbidden");
    // Sanity: no worker_skill row written.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker_skill")
        .fetch_one(store.pool())
        .await
        .expect("count");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn bulk_assign_400_on_invalid_json_body() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let req = req_bulk_assign("support::developing", None, "not json");
    let (status, _b) =
        bulk_assign_skill_response(&req, "support::developing", Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn bulk_assign_400_on_empty_worker_ids() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let body = serde_json::json!({
        "worker_ids": [],
        "skill_id": "follow_up",
    })
    .to_string();
    let req = req_bulk_assign("support::developing", None, &body);
    let (status, _b) =
        bulk_assign_skill_response(&req, "support::developing", Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn bulk_assign_writes_one_audit_row_per_worker() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let w1 = seed(&store, tenant, "Maya", "support").await;
    let w2 = seed(&store, tenant, "Priya", "support").await;
    set_trust(&store, &w1.id.to_string(), 2).await;
    set_trust(&store, &w2.id.to_string(), 2).await;
    let body = serde_json::json!({
        "worker_ids": [w1.id.to_string(), w2.id.to_string()],
        "skill_id": "follow_up",
    })
    .to_string();
    let req = req_bulk_assign("support::developing", None, &body);
    let (status, _b) =
        bulk_assign_skill_response(&req, "support::developing", Some(&store)).await;
    assert_eq!(status, "200 OK");
    // Two audit rows must surface in audit_record — one per worker. The
    // payload's `endpoint` field equals "skill_assigned" so the cohort
    // drawer activity feed picks it up via the summary fallback.
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT worker_id, payload_json FROM audit_record \
         WHERE tenant_id = ? AND payload_json LIKE '%skill_assigned%'",
    )
    .bind(tenant.to_string())
    .fetch_all(store.pool())
    .await
    .expect("audit rows");
    assert_eq!(rows.len(), 2, "expected one audit row per worker");
    let worker_ids: std::collections::BTreeSet<String> =
        rows.into_iter().map(|(wid, _)| wid).collect();
    assert!(worker_ids.contains(&w1.id.to_string()));
    assert!(worker_ids.contains(&w2.id.to_string()));
}
