//! Tests for `routes_worker_memory` — `GET /api/v1/workers/<id>/memory`
//! and `*.../memory/facts*` CRUD (Stream J10).
//!
//! Mirrors `routes_worker_detail_tests` infrastructure (env-lock,
//! tempdir-pinned `ARGO_HOME`, cwd-derived workspace tenant) so the
//! handler's `ensure_workspace` resolution is deterministic.

#![allow(clippy::too_many_lines)]

use super::{
    fact_create_response, fact_delete_response, fact_update_response, list_response,
    FACT_TOMBSTONE_MARKER,
};
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
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store
        .ensure_workspace(name, &root)
        .await
        .expect("ensure workspace");
    tenant
}

async fn add_worker(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    name: &str,
    builder: &str,
) -> Worker {
    let mut worker = Worker::draft(
        tenant,
        "support",
        Persona::named(name, builder),
        Role::new("Resolve customer questions."),
    );
    worker.persona.voice_profile.preamble = "Be warm and direct.".into();
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

fn get_req(id: &str, role: Option<&str>, principal: Option<&str>) -> String {
    let mut req = format!("GET /api/v1/workers/{id}/memory HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    if let Some(p) = principal {
        req.push_str(&format!("X-Argo-Principal: {p}\r\n"));
    }
    req.push_str("\r\n");
    req
}

fn post_req(id: &str, body: &str) -> String {
    let body_with_clrf = format!(
        "POST /api/v1/workers/{id}/memory/facts HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    body_with_clrf
}

fn put_req(id: &str, fact_id: &str, body: &str) -> String {
    format!(
        "PUT /api/v1/workers/{id}/memory/facts/{fact_id} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn delete_req(id: &str, fact_id: &str) -> String {
    format!("DELETE /api/v1/workers/{id}/memory/facts/{fact_id} HTTP/1.1\r\nHost: x\r\n\r\n")
}

async fn create_fact(store: &Arc<SessionStore>, worker: &Worker, text: &str) -> String {
    let req = post_req(&worker.id.to_string(), &format!(r#"{{"text":"{text}"}}"#));
    let (status, body) =
        fact_create_response(&worker.id.to_string(), &req, Some(store)).await;
    assert_eq!(status, "201 Created", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    v["fact_id"].as_str().expect("fact_id").to_string()
}

// =========================================================================

#[tokio::test]
async fn get_memory_returns_bundled_shape() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let req = get_req(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let (status, body) = list_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    for key in ["facts", "active_conversations", "recent_decisions"] {
        assert!(v[key].is_array(), "missing array key {key}");
    }
    assert!(v["voice_profile"].is_object());
    assert_eq!(v["voice_profile"]["preamble"], "Be warm and direct.");
}

#[tokio::test]
async fn get_memory_hides_namespace_id_from_delegator() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let req = get_req(&worker.id.to_string(), Some("delegator"), Some("human:mark"));
    let (status, body) = list_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert!(v["namespace_id"].is_null(), "delegator must not see namespace_id");
}

#[tokio::test]
async fn get_memory_shows_namespace_id_to_operator() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let req = get_req(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let (status, body) = list_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    let ns = v["namespace_id"].as_str().expect("namespace_id");
    assert!(ns.contains('/'), "namespace_id format: {ns}");
}

#[tokio::test]
async fn get_memory_invalid_worker_id_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let (status, body) = list_response("not-a-uuid", "GET /\r\n\r\n", Some(&store)).await;
    assert_eq!(status, "400 Bad Request", "body: {body}");
}

#[tokio::test]
async fn get_memory_unknown_worker_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let ghost = Uuid::now_v7().to_string();
    let req = get_req(&ghost, None, None);
    let (status, body) = list_response(&ghost, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found", "body: {body}");
}

#[tokio::test]
async fn get_memory_cross_tenant_worker_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let other_tenant = TenantId::new();
    let alien = add_worker(&store, other_tenant, "Alien", "human:mark").await;
    let req = get_req(&alien.id.to_string(), None, None);
    let (status, body) = list_response(&alien.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found", "body: {body}");
}

#[tokio::test]
async fn post_fact_creates_row_and_returns_201() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let fact_id = create_fact(&store, &worker, "Customer prefers email").await;
    assert!(Uuid::parse_str(&fact_id).is_ok(), "fact_id parses: {fact_id}");
    // Visible in subsequent GET.
    let req = get_req(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let (_, body) = list_response(&worker.id.to_string(), &req, Some(&store)).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    let facts = v["facts"].as_array().expect("facts array");
    assert!(facts.iter().any(|f| f["fact_id"] == fact_id), "fact present");
}

#[tokio::test]
async fn post_empty_text_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let req = post_req(&worker.id.to_string(), r#"{"text":""}"#);
    let (status, body) =
        fact_create_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request", "body: {body}");
}

#[tokio::test]
async fn post_oversized_text_returns_400_with_limit() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let big = "x".repeat(1001);
    let req = post_req(&worker.id.to_string(), &format!(r#"{{"text":"{big}"}}"#));
    let (status, body) =
        fact_create_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request", "body: {body}");
    assert!(body.contains("1000"), "limit message: {body}");
}

#[tokio::test]
async fn post_tombstone_marker_rejected() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let body_json = serde_json::to_string(&serde_json::json!({
        "text": format!("trick {FACT_TOMBSTONE_MARKER}"),
    }))
    .unwrap();
    let req = post_req(&worker.id.to_string(), &body_json);
    let (status, body) =
        fact_create_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request", "body: {body}");
}

#[tokio::test]
async fn put_fact_updates_text_and_returns_200() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let fact_id = create_fact(&store, &worker, "old text").await;
    let req = put_req(&worker.id.to_string(), &fact_id, r#"{"text":"new text"}"#);
    let (status, body) =
        fact_update_response(&worker.id.to_string(), &fact_id, &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["fact_id"], fact_id, "original fact_id preserved");
    assert_eq!(v["text"], "new text");
    assert!(v["edited_at"].is_string());
}

#[tokio::test]
async fn put_unknown_fact_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let ghost = Uuid::now_v7().to_string();
    let req = put_req(&worker.id.to_string(), &ghost, r#"{"text":"new"}"#);
    let (status, body) =
        fact_update_response(&worker.id.to_string(), &ghost, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found", "body: {body}");
}

#[tokio::test]
async fn put_cross_worker_fact_returns_404() {
    // Fact created under worker A; PUT under worker B's URL must 404.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker_a = add_worker(&store, tenant, "Aya", "human:mark").await;
    let worker_b = add_worker(&store, tenant, "Bryn", "human:mark").await;
    let fact_id = create_fact(&store, &worker_a, "private to Aya").await;
    let req = put_req(&worker_b.id.to_string(), &fact_id, r#"{"text":"hijack"}"#);
    let (status, body) =
        fact_update_response(&worker_b.id.to_string(), &fact_id, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found", "body: {body}");
}

#[tokio::test]
async fn delete_fact_returns_204_and_hides_from_get() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let fact_id = create_fact(&store, &worker, "sensitive").await;
    let req = delete_req(&worker.id.to_string(), &fact_id);
    let (status, body) =
        fact_delete_response(&worker.id.to_string(), &fact_id, &req, Some(&store)).await;
    assert_eq!(status, "204 No Content");
    assert!(body.is_empty());
    // Subsequent GET no longer shows it.
    let get = get_req(&worker.id.to_string(), Some("operator"), Some("human:mark"));
    let (_, body) = list_response(&worker.id.to_string(), &get, Some(&store)).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    let facts = v["facts"].as_array().unwrap();
    assert!(
        !facts.iter().any(|f| f["fact_id"] == fact_id),
        "deleted fact must not surface"
    );
}

#[tokio::test]
async fn delete_unknown_fact_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    let ghost = Uuid::now_v7().to_string();
    let req = delete_req(&worker.id.to_string(), &ghost);
    let (status, _) =
        fact_delete_response(&worker.id.to_string(), &ghost, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn audit_row_written_per_mutation() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya", "human:mark").await;
    // One fact exercises the edit path; a second exercises delete.
    // Delete-after-edit would target the now-superseded id and 404.
    let fact_a = create_fact(&store, &worker, "initial").await;
    let put = put_req(&worker.id.to_string(), &fact_a, r#"{"text":"updated"}"#);
    let _ = fact_update_response(&worker.id.to_string(), &fact_a, &put, Some(&store)).await;
    let fact_b = create_fact(&store, &worker, "to be deleted").await;
    let del = delete_req(&worker.id.to_string(), &fact_b);
    let _ = fact_delete_response(&worker.id.to_string(), &fact_b, &del, Some(&store)).await;
    let endpoints: Vec<String> = sqlx::query_scalar(
        "SELECT json_extract(payload_json, '$.endpoint') FROM audit_record \
         WHERE kind = 'http.request' AND json_extract(payload_json, '$.endpoint') \
               LIKE 'worker.memory.%' \
         ORDER BY seq ASC",
    )
    .fetch_all(store.pool())
    .await
    .expect("audit scan");
    assert!(
        endpoints.iter().any(|e| e == "worker.memory.fact.created"),
        "created audit row missing: {endpoints:?}"
    );
    assert!(
        endpoints.iter().any(|e| e == "worker.memory.fact.edited"),
        "edited audit row missing: {endpoints:?}"
    );
    assert!(
        endpoints.iter().any(|e| e == "worker.memory.fact.deleted"),
        "deleted audit row missing: {endpoints:?}"
    );
}

#[tokio::test]
async fn handler_returns_503_when_store_missing() {
    let _g = env_lock().await;
    let id = Uuid::now_v7().to_string();
    let (status, _) = list_response(&id, &get_req(&id, None, None), None).await;
    assert_eq!(status, "503 Service Unavailable");
    let (status, _) = fact_create_response(&id, &post_req(&id, r#"{"text":"x"}"#), None).await;
    assert_eq!(status, "503 Service Unavailable");
    let fid = Uuid::now_v7().to_string();
    let (status, _) =
        fact_update_response(&id, &fid, &put_req(&id, &fid, r#"{"text":"x"}"#), None).await;
    assert_eq!(status, "503 Service Unavailable");
    let (status, _) = fact_delete_response(&id, &fid, &delete_req(&id, &fid), None).await;
    assert_eq!(status, "503 Service Unavailable");
}
