//! Tests for `routes_duties` — Stream J13.
//!
//! Drives the five entry points directly against an in-memory SQLite
//! store seeded with a `Worker` row. Bearer auth + principal-header
//! parsing live upstream in `routes_dispatch_post`; these tests focus
//! on body shapes, validation, conflict detection, and audit-row
//! side-effects.

#![allow(clippy::too_many_lines)]

use super::{
    classify_response, commit_response, delete_response, patch_response, preview_response,
};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
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
    let (tenant, _) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .expect("ensure workspace");
    tenant
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named(name, "human:mark"),
        Role::new("Resolve customer questions."),
    );
    IdentityStore::new(store).create_worker(&worker).await.expect("create worker");
    worker
}

fn post_req(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn patch_request(path: &str, body: &str) -> String {
    format!(
        "PATCH {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn delete_req(path: &str) -> String {
    format!("DELETE {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\n\r\n")
}

async fn commit_simple(
    store: &Arc<SessionStore>,
    worker: &WorkerId,
    schedule: &str,
    label: &str,
) -> String {
    let body = json!({
        "duty_proposal": { "label": label, "schedule": schedule, "entry_action": "ping" }
    })
    .to_string();
    let req = post_req(&format!("/api/v1/workers/{worker}/duties/commit"), &body);
    let (status, resp) = commit_response(&worker.to_string(), &req, Some(store)).await;
    assert_eq!(status, "201 Created", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    v["duty_id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn classify_returns_proposal_with_samples_and_no_conflicts_on_fresh_worker() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let body = json!({ "input": "send outbound sales emails every morning" }).to_string();
    let req = post_req(&format!("/api/v1/workers/{}/duties/classify", worker.id), &body);
    let (status, resp) = classify_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert!(v["duty_proposal"]["schedule"].as_str().is_some());
    assert!(v["duty_proposal"]["entry_action"].as_str().is_some());
    assert!(v["duty_proposal"]["label"].as_str().is_some());
    let samples = v["sample_outputs"].as_array().expect("array");
    assert_eq!(samples.len(), 3);
    assert!(v["conflicts"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn classify_rejects_malformed_cron_hint_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Devon").await;
    let body = json!({ "input": "ops check daily", "cron_hint": "not a cron" }).to_string();
    let req = post_req(&format!("/api/v1/workers/{}/duties/classify", worker.id), &body);
    let (status, resp) = classify_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn classify_returns_404_for_unknown_worker() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let ghost = WorkerId::new();
    let body = json!({ "input": "summarize day" }).to_string();
    let req = post_req(&format!("/api/v1/workers/{ghost}/duties/classify"), &body);
    let (status, resp) = classify_response(&ghost.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found", "body: {resp}");
}

#[tokio::test]
async fn classify_cross_tenant_worker_returns_403() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let other = TenantId::new();
    let alien = add_worker(&store, other, "Alien").await;
    let body = json!({ "input": "alien duty" }).to_string();
    let req = post_req(&format!("/api/v1/workers/{}/duties/classify", alien.id), &body);
    let (status, _) = classify_response(&alien.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "403 Forbidden");
}

#[tokio::test]
async fn classify_archived_worker_returns_410() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Sundown").await;
    IdentityStore::new(&store)
        .archive_worker(tenant, worker.id)
        .await
        .expect("archive worker");
    let body = json!({ "input": "x" }).to_string();
    let req = post_req(&format!("/api/v1/workers/{}/duties/classify", worker.id), &body);
    let (status, resp) = classify_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "410 Gone", "body: {resp}");
}

#[tokio::test]
async fn commit_inserts_duty_and_returns_first_fire() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let id = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let row: Option<String> =
        sqlx::query_scalar("SELECT description FROM role_duty WHERE id = ?")
            .bind(&id)
            .fetch_optional(store.pool())
            .await
            .unwrap();
    assert_eq!(row.as_deref(), Some("Daily summary"));
}

#[tokio::test]
async fn commit_rejects_short_label_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let body = json!({
        "duty_proposal": { "label": "abc", "schedule": "0 6 * * *", "entry_action": "ping" }
    }).to_string();
    let req = post_req(&format!("/api/v1/workers/{}/duties/commit", worker.id), &body);
    let (status, resp) = commit_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert!(v["error"]["message"].as_str().unwrap().contains("label"));
}

#[tokio::test]
async fn commit_rejects_non_5_field_cron_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let body = json!({
        "duty_proposal": { "label": "Daily ping", "schedule": "0 0 6 * * *", "entry_action": "ping" }
    }).to_string();
    let req = post_req(&format!("/api/v1/workers/{}/duties/commit", worker.id), &body);
    let (status, _) = commit_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn commit_detects_cron_conflict_and_requires_resolution() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let _first = commit_simple(&store, &worker.id, "0 6 * * *", "First duty").await;
    let body = json!({
        "duty_proposal": { "label": "Second duty", "schedule": "0 6 * * *", "entry_action": "ping" }
    }).to_string();
    let req = post_req(&format!("/api/v1/workers/{}/duties/commit", worker.id), &body);
    let (status, resp) = commit_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "409 Conflict", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["error"]["code"], "duty_conflict");
}

#[tokio::test]
async fn commit_with_replace_resolution_deletes_overlap() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let first = commit_simple(&store, &worker.id, "0 6 * * *", "First duty").await;
    let body = json!({
        "duty_proposal": { "label": "Replacement", "schedule": "0 6 * * *", "entry_action": "ping" },
        "accepted_conflict_resolution": "replace",
    }).to_string();
    let req = post_req(&format!("/api/v1/workers/{}/duties/commit", worker.id), &body);
    let (status, _) = commit_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "201 Created");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM role_duty WHERE id = ?")
        .bind(&first)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 0, "old duty should have been deleted");
}

#[tokio::test]
async fn preview_returns_requested_samples() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let duty_id = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let body = json!({ "duty_id": duty_id, "sample_size": 2 }).to_string();
    let req = post_req(&format!("/api/v1/workers/{}/duties/preview", worker.id), &body);
    let (status, resp) = preview_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["samples"].as_array().unwrap().len(), 2);
    assert!(v["samples"][0]["would_send"].as_str().is_some());
}

#[tokio::test]
async fn preview_unknown_duty_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let body = json!({ "duty_id": "does-not-exist", "sample_size": 3 }).to_string();
    let req = post_req(&format!("/api/v1/workers/{}/duties/preview", worker.id), &body);
    let (status, _) = preview_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn patch_updates_label_and_schedule_and_enabled() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let duty_id = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let body = json!({ "label": "Updated summary", "schedule": "0 7 * * *", "enabled": false }).to_string();
    let path = format!("/api/v1/workers/{}/duties/{duty_id}", worker.id);
    let req = patch_request(&path, &body);
    let (status, resp) = patch_response(&worker.id.to_string(), &duty_id, &req, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["duty"]["label"], "Updated summary");
    assert_eq!(v["duty"]["enabled"], false);
    assert!(v["duty"]["cron"].as_str().unwrap().contains("0 7"));
}

#[tokio::test]
async fn patch_empty_body_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let duty_id = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let req = patch_request(
        &format!("/api/v1/workers/{}/duties/{duty_id}", worker.id),
        "{}",
    );
    let (status, _) = patch_response(&worker.id.to_string(), &duty_id, &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn delete_removes_row_and_returns_204() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let duty_id = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let path = format!("/api/v1/workers/{}/duties/{duty_id}", worker.id);
    let (status, body) =
        delete_response(&worker.id.to_string(), &duty_id, &delete_req(&path), Some(&store)).await;
    assert_eq!(status, "204 No Content");
    assert!(body.is_empty(), "204 should have empty body, got: {body}");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM role_duty WHERE id = ?")
        .bind(&duty_id)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn delete_unknown_duty_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let path = format!("/api/v1/workers/{}/duties/missing", worker.id);
    let (status, _) =
        delete_response(&worker.id.to_string(), "missing", &delete_req(&path), Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn returns_503_when_store_missing() {
    let id = WorkerId::new();
    let body = json!({ "input": "x" }).to_string();
    let req = post_req(&format!("/api/v1/workers/{id}/duties/classify"), &body);
    let (status, resp) = classify_response(&id.to_string(), &req, None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(resp.contains("session store unavailable"));
}

#[tokio::test]
async fn commit_emits_audit_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%duty.commit%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let _ = commit_simple(&store, &worker.id, "0 6 * * *", "Daily summary").await;
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%duty.commit%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(after, before + 1);
}
