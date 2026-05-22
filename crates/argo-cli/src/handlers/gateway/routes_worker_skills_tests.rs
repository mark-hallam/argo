//! Tests for `routes_worker_skills` — five endpoints behind the
//! locked C2 worker-detail Skills tab.
//!
//! Mirrors `routes_worker_memory_tests` infrastructure: env-lock,
//! tempdir-pinned `ARGO_HOME`, cwd-derived workspace tenant so the
//! handler's `ensure_workspace` resolution is deterministic.

#![allow(clippy::too_many_lines)]

use super::{
    add_response, dispatch_post, history_response, list_response, promote_response,
    remove_response,
};
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};
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

fn skill_doc(name: &str, version: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: d\n\
         provenance:\n  source: builtin\n  hash: sha256-x\n---\nbody"
    )
}

async fn install_skill(store: &Arc<SessionStore>, name: &str) -> SkillId {
    let skills = SkillStore::new(Arc::clone(store));
    let skill = skills
        .install(&skill_doc(name, "0.1.0"), None)
        .await
        .expect("install skill");
    skill.id
}

fn get_req(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

fn post_req(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn delete_req(path: &str) -> String {
    format!("DELETE {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

// =========================================================================
// GET /api/v1/workers/<id>/skills
// =========================================================================

#[tokio::test]
async fn get_skills_returns_three_section_envelope() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    install_skill(&store, "answer-customer").await;

    let path = format!("/api/v1/workers/{}/skills", worker.id);
    let req = get_req(&path);
    let (status, body) = list_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("parse json");
    for key in ["active", "library", "learned"] {
        assert!(v[key].is_array(), "missing array key {key}: {v}");
    }
    assert!(v["total_counts"].is_object(), "total_counts: {v}");
    assert_eq!(v["total_counts"]["learned"], 0);
}

#[tokio::test]
async fn get_skills_library_includes_available_skills_not_active() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let _sid = install_skill(&store, "answer-customer").await;

    let req = get_req(&format!("/api/v1/workers/{}/skills", worker.id));
    let (status, body) = list_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("parse json");
    // The freshly installed skill is in the library (no overlay row yet).
    let lib = v["library"].as_array().expect("library array");
    assert!(
        lib.iter()
            .any(|e| e["name"].as_str() == Some("answer-customer")),
        "library should include the unattached skill: {v}"
    );
}

#[tokio::test]
async fn get_skills_cross_tenant_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _cwd_tenant = workspace_tenant(&store).await;
    let alien_tenant = TenantId::new();
    let alien = add_worker(&store, alien_tenant, "Alien").await;
    let req = get_req(&format!("/api/v1/workers/{}/skills", alien.id));
    let (status, _) = list_response(&alien.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn get_skills_unknown_worker_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let ghost = uuid::Uuid::now_v7();
    let req = get_req(&format!("/api/v1/workers/{ghost}/skills"));
    let (status, _) = list_response(&ghost.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn get_skills_invalid_worker_id_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let (status, _) = list_response("not-a-uuid", "GET /\r\n\r\n", Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

// =========================================================================
// POST /api/v1/workers/<id>/skills (add)
// =========================================================================

#[tokio::test]
async fn post_skill_adds_to_active_overlay() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let skill_id = install_skill(&store, "auto-decline").await;

    let body = format!(r#"{{"skill_id":"{}"}}"#, skill_id.0);
    let path = format!("/api/v1/workers/{}/skills", worker.id);
    let req = post_req(&path, &body);
    let (status, resp_body) = add_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "201 Created", "body: {resp_body}");

    // Verify via raw `worker_skill` read — the canonical
    // `SkillStore::list_for_worker` JOIN is broken upstream by a
    // schema mismatch (`skill.id BLOB` vs `worker_skill.skill_id
    // TEXT`), separate brief.
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT skill_id, proficiency FROM worker_skill WHERE worker_id = ?")
            .bind(worker.id.to_string())
            .fetch_all(store.pool())
            .await
            .expect("query worker_skill");
    assert_eq!(rows.len(), 1, "expected one overlay row");
    assert_eq!(rows[0].0, skill_id.0.to_string());
}

#[tokio::test]
async fn post_skill_missing_body_field_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;

    let path = format!("/api/v1/workers/{}/skills", worker.id);
    let req = post_req(&path, "{}");
    let (status, _) = add_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

// =========================================================================
// DELETE /api/v1/workers/<id>/skills/<sid>
// =========================================================================

#[tokio::test]
async fn delete_skill_removes_from_active_overlay() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let skill_id = install_skill(&store, "drop-me").await;
    // Seed an overlay row.
    IdentityStore::new(&store)
        .add_worker_skill(worker.id, &skill_id.0.to_string(), 3)
        .await
        .expect("seed overlay");

    let path = format!("/api/v1/workers/{}/skills/{}", worker.id, skill_id.0);
    let req = delete_req(&path);
    let (status, _) =
        remove_response(&worker.id.to_string(), &skill_id.0.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "204 No Content");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker_skill WHERE worker_id = ?")
        .bind(worker.id.to_string())
        .fetch_one(store.pool())
        .await
        .expect("count");
    assert_eq!(count, 0, "overlay should be empty after DELETE");
}

/// Load-bearing cross-tenant guard (per conductor pattern post-#1983):
/// a caller in tenant A may not remove a skill from a worker in
/// tenant B. The handler resolves the workspace tenant from cwd and
/// then looks the worker up under that tenant — an alien worker
/// surfaces as 404 (never leaks existence) and the overlay row in
/// the alien tenant is untouched.
#[tokio::test]
async fn delete_skill_cross_tenant_returns_404_and_does_not_touch_overlay() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _cwd_tenant = workspace_tenant(&store).await;
    let alien_tenant = TenantId::new();
    let alien = add_worker(&store, alien_tenant, "Alien").await;
    let skill_id = install_skill(&store, "alien-skill").await;
    IdentityStore::new(&store)
        .add_worker_skill(alien.id, &skill_id.0.to_string(), 3)
        .await
        .expect("seed alien overlay");

    let path = format!("/api/v1/workers/{}/skills/{}", alien.id, skill_id.0);
    let req = delete_req(&path);
    let (status, _) =
        remove_response(&alien.id.to_string(), &skill_id.0.to_string(), &req, Some(&store)).await;
    assert_eq!(
        status, "404 Not Found",
        "cross-tenant DELETE must surface as 404 (no existence leak)"
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker_skill WHERE worker_id = ?")
        .bind(alien.id.to_string())
        .fetch_one(store.pool())
        .await
        .expect("count");
    assert_eq!(
        count, 1,
        "cross-tenant DELETE must not touch the alien tenant's overlay rows"
    );
}

// =========================================================================
// GET /api/v1/workers/<id>/skills/<sid>/history
// =========================================================================

#[tokio::test]
async fn history_returns_audit_events_for_skill() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let skill_id = install_skill(&store, "history-skill").await;

    // Seed a `worker.skills.added` event by issuing a POST.
    let post_path = format!("/api/v1/workers/{}/skills", worker.id);
    let post_body = format!(r#"{{"skill_id":"{}"}}"#, skill_id.0);
    let (s, _) = add_response(
        &worker.id.to_string(),
        &post_req(&post_path, &post_body),
        Some(&store),
    )
    .await;
    assert_eq!(s, "201 Created");

    let hist_path = format!(
        "/api/v1/workers/{}/skills/{}/history",
        worker.id, skill_id.0
    );
    let (status, body) = history_response(
        &worker.id.to_string(),
        &skill_id.0.to_string(),
        &get_req(&hist_path),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).expect("json");
    let events = v["events"].as_array().expect("events array");
    assert!(
        !events.is_empty(),
        "history should surface the seeded add event: {v}"
    );
}

// =========================================================================
// POST /api/v1/workers/<id>/skills/<sid>/promote
// =========================================================================

#[tokio::test]
async fn promote_attaches_skill_and_writes_audit_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let skill_id = install_skill(&store, "promote-me").await;

    let path = format!(
        "/api/v1/workers/{}/skills/{}/promote",
        worker.id, skill_id.0
    );
    let (status, body) = promote_response(
        &worker.id.to_string(),
        &skill_id.0.to_string(),
        &post_req(&path, "{}"),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK", "body: {body}");

    // Overlay row is present (verified via raw read — see note in
    // `post_skill_adds_to_active_overlay`).
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker_skill WHERE worker_id = ?")
        .bind(worker.id.to_string())
        .fetch_one(store.pool())
        .await
        .expect("count");
    assert_eq!(count, 1);
    // And an `SkillCandidatePromoted` audit row landed.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE kind = 'skill.candidate.promoted'",
    )
    .fetch_one(store.pool())
    .await
    .expect("count audit");
    assert!(count >= 1, "promote should write audit row, found {count}");
}

#[tokio::test]
async fn promote_writes_trust_ratchet_max_proficiency() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let skill_id = install_skill(&store, "ratchet-me").await;

    let path = format!(
        "/api/v1/workers/{}/skills/{}/promote",
        worker.id, skill_id.0
    );
    let (status, _) = promote_response(
        &worker.id.to_string(),
        &skill_id.0.to_string(),
        &post_req(&path, "{}"),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK");
    let prof: Vec<(String, i64)> = IdentityStore::new(&store)
        .skills_for_worker(worker.id)
        .await
        .expect("prof lookup");
    assert_eq!(prof.len(), 1);
    assert_eq!(prof[0].1, 5, "promote should ratchet proficiency to 5");
}

// =========================================================================
// Dispatch wire-up smoke
// =========================================================================

#[tokio::test]
async fn dispatch_post_routes_add_and_delete() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let skill_id = install_skill(&store, "dispatched").await;

    let add_path = format!("/api/v1/workers/{}/skills", worker.id);
    let body = format!(r#"{{"skill_id":"{}"}}"#, skill_id.0);
    let resp = dispatch_post(&post_req(&add_path, &body), Some(&store)).await;
    assert!(resp.is_some(), "POST should match dispatcher");
    assert_eq!(resp.unwrap().0, "201 Created");

    let del_path = format!("/api/v1/workers/{}/skills/{}", worker.id, skill_id.0);
    let resp = dispatch_post(&delete_req(&del_path), Some(&store)).await;
    assert!(resp.is_some(), "DELETE should match dispatcher");
    assert_eq!(resp.unwrap().0, "204 No Content");
}
