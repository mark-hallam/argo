//! Shared test fixtures for the `routes_duties` test modules. Mechanical
//! extraction from `routes_duties_tests.rs` (no behaviour change) so the
//! test suite splits across `routes_duties_tests.rs` +
//! `routes_duties_tests2.rs` without duplicating the in-memory-store
//! scaffolding. Both test modules pull this via `use super::duty_test_helpers::*`.

pub(super) use super::{
    classify_response, commit_response, delete_response, list_response, patch_response,
    preview_response,
};
pub(super) use argo_identity::{IdentityStore, WorkerId};
pub(super) use argo_types::TenantId;
pub(super) use serde_json::{json, Value};

use argo_identity::{Persona, Role, Worker};
use argo_store::SessionStore;
use std::path::PathBuf;
use std::sync::Arc;

pub(super) fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

pub(super) fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

pub(super) async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

pub(super) async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let (tenant, _) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .expect("ensure workspace");
    tenant
}

pub(super) async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named(name, "human:mark"),
        Role::new("Resolve customer questions."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

pub(super) fn post_req(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

pub(super) fn patch_request(path: &str, body: &str) -> String {
    format!(
        "PATCH {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

pub(super) fn delete_req(path: &str) -> String {
    format!("DELETE {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\n\r\n")
}

pub(super) fn get_req(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: human:mark@example.com\r\n\r\n")
}

pub(super) async fn commit_simple(
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
