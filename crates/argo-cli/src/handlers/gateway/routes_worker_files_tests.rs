//! Tests for `routes_worker_files` — the tenant-scoped file-ingress
//! endpoints `POST|GET /api/v1/workers/<id>/files` +
//! `DELETE /api/v1/workers/<id>/files/<name>`.
//!
//! Pins the brief's acceptance criteria:
//! 1. upload lands INSIDE `fs_root`; traversal names refused; over-size
//!    refused; disallowed extension refused; cross-tenant worker → 404;
//!    audit row present.
//! 2. live-style: the uploaded file is readable through `confine_path`
//!    (the same jail check the teammate's `file_read` applies) on the
//!    next run.
//!
//! `ARGO_HOME` + `ARGO_VERTICALS_DIR` are pinned under the process-wide
//! env lock so the cwd-derived workspace tenant is deterministic.

#![allow(clippy::expect_used)]

use super::{delete_response, list_response, upload_response};
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_tool::confine_path;
use argo_types::{TenantId, WorkspaceId};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const VERTICAL: &str = "customer-support";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn set_test_env(tmp: &Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn workspace_tenant(store: &Arc<SessionStore>) -> (TenantId, WorkspaceId) {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .expect("ensure workspace")
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId) -> Worker {
    let worker = Worker::draft(
        tenant,
        VERTICAL,
        Persona::named("Maya", "human:mark"),
        Role::new("Work on the files in my folder."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

/// Stamp `worker.fs_root` directly (the resolver reads the column).
async fn set_fs_root(store: &Arc<SessionStore>, worker: &Worker, root: &Path) {
    sqlx::query("UPDATE worker SET fs_root = ? WHERE id = ?")
        .bind(root.to_string_lossy().to_string())
        .bind(worker.id.to_string())
        .execute(store.pool())
        .await
        .expect("set fs_root");
}

fn upload_req(worker_id: &str, filename: &str, bytes: &[u8]) -> String {
    let body = serde_json::json!({
        "filename": filename,
        "content_base64": STANDARD.encode(bytes),
    })
    .to_string();
    format!(
        "POST /api/v1/workers/{worker_id}/files HTTP/1.1\r\nHost: x\r\n\r\n{body}"
    )
}

fn get_req(worker_id: &str) -> String {
    format!("GET /api/v1/workers/{worker_id}/files HTTP/1.1\r\nHost: x\r\n\r\n")
}

fn delete_req(worker_id: &str, name: &str) -> String {
    format!("DELETE /api/v1/workers/{worker_id}/files/{name} HTTP/1.1\r\nHost: x\r\n\r\n")
}

async fn audit_count(store: &SessionStore, endpoint: &str) -> i64 {
    let like = format!("%{endpoint}%");
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE ?")
        .bind(like)
        .fetch_one(store.pool())
        .await
        .expect("audit count")
}

#[tokio::test]
async fn upload_lands_inside_fs_root_and_audits() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (tenant, _ws) = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;
    let fs_root = tmp.path().join("fs_root");
    set_fs_root(&store, &worker, &fs_root).await;

    let req = upload_req(&worker.id.to_string(), "invoice.txt", b"acme 4471");
    let (status, body) = upload_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "201 Created", "body: {body}");

    let landed = fs_root.join("invoice.txt");
    assert!(landed.is_file(), "file did not land in fs_root");
    assert_eq!(std::fs::read(&landed).expect("read"), b"acme 4471");
    assert_eq!(audit_count(&store, "worker.files.uploaded").await, 1);
}

/// Acceptance #2 — the uploaded file is readable through `confine_path`,
/// the SAME jail check the teammate's `file_read` applies. If this passes,
/// the next run can read the ingested file.
#[tokio::test]
async fn uploaded_file_readable_via_confine_path() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (tenant, _ws) = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;
    let fs_root = tmp.path().join("fs_root");
    set_fs_root(&store, &worker, &fs_root).await;

    let req = upload_req(&worker.id.to_string(), "notes.md", b"# hi\n");
    let (status, _b) = upload_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "201 Created");

    // The fence permits this read because the path is inside fs_root.
    let resolved = confine_path(&fs_root, Path::new("notes.md")).expect("confine");
    assert_eq!(std::fs::read_to_string(resolved).expect("read"), "# hi\n");
}

#[tokio::test]
async fn traversal_filename_refused() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (tenant, _ws) = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;
    set_fs_root(&store, &worker, &tmp.path().join("fs_root")).await;

    for name in ["../escape.txt", "sub/evil.txt", "..", "/etc/passwd"] {
        let req = upload_req(&worker.id.to_string(), name, b"x");
        let (status, body) = upload_response(&worker.id.to_string(), &req, Some(&store)).await;
        assert_eq!(status, "400 Bad Request", "name {name} should be refused: {body}");
    }
    // Nothing escaped the tempdir.
    assert!(!tmp.path().join("escape.txt").exists());
}

#[tokio::test]
async fn disallowed_extension_refused() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (tenant, _ws) = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;
    set_fs_root(&store, &worker, &tmp.path().join("fs_root")).await;

    for name in ["payload.sh", "tool", "macro.exe", "lib.so"] {
        let req = upload_req(&worker.id.to_string(), name, b"x");
        let (status, _b) = upload_response(&worker.id.to_string(), &req, Some(&store)).await;
        assert_eq!(status, "400 Bad Request", "executable {name} should be refused");
    }
}

#[tokio::test]
async fn oversize_refused() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (tenant, _ws) = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;
    set_fs_root(&store, &worker, &tmp.path().join("fs_root")).await;

    // One byte over the 10 MiB per-file cap.
    let big = vec![b'a'; 10 * 1024 * 1024 + 1];
    let req = upload_req(&worker.id.to_string(), "big.txt", &big);
    let (status, _b) = upload_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "413 Payload Too Large");
}

#[tokio::test]
async fn cross_tenant_worker_is_404() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    // Caller resolves to the cwd-workspace tenant; the worker lives under a
    // DIFFERENT tenant, so the upload must 404 (existence not leaked).
    let (_caller, _cws) = workspace_tenant(&store).await;
    let (other_tenant, _ows) = store
        .ensure_workspace("other", "/tmp/other-tenant-files-root")
        .await
        .expect("other workspace");
    let worker = add_worker(&store, other_tenant).await;
    set_fs_root(&store, &worker, &tmp.path().join("fs_root")).await;

    let req = upload_req(&worker.id.to_string(), "x.txt", b"x");
    let (status, _b) = upload_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn list_then_delete_roundtrip() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (tenant, _ws) = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;
    let fs_root = tmp.path().join("fs_root");
    set_fs_root(&store, &worker, &fs_root).await;
    let wid = worker.id.to_string();

    // Upload two files.
    for (name, bytes) in [("a.txt", &b"hello"[..]), ("b.csv", &b"1,2,3"[..])] {
        let req = upload_req(&wid, name, bytes);
        let (status, _b) = upload_response(&wid, &req, Some(&store)).await;
        assert_eq!(status, "201 Created");
    }

    // List shows names + sizes, never contents.
    let (status, body) = list_response(&wid, &get_req(&wid), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).expect("json");
    let files = v["files"].as_array().expect("files array");
    assert_eq!(files.len(), 2);
    assert_eq!(files[0]["name"], "a.txt");
    assert_eq!(files[0]["size_bytes"], 5);
    assert!(files[0].get("content").is_none(), "list must not leak contents");
    assert_eq!(v["quota_bytes"], 100 * 1024 * 1024);

    // Delete one.
    let (status, _b) = delete_response(&wid, "a.txt", &delete_req(&wid, "a.txt"), Some(&store)).await;
    assert_eq!(status, "204 No Content");
    assert!(!fs_root.join("a.txt").exists());
    assert!(fs_root.join("b.csv").exists());
    assert_eq!(audit_count(&store, "worker.files.deleted").await, 1);

    // Deleting a missing file → 404.
    let (status, _b) = delete_response(&wid, "a.txt", &delete_req(&wid, "a.txt"), Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn guest_role_is_denied_upload() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let (tenant, _ws) = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant).await;
    set_fs_root(&store, &worker, &tmp.path().join("fs_root")).await;

    // An explicit guest role header is honored in the test build → 403.
    let body = serde_json::json!({
        "filename": "x.txt",
        "content_base64": STANDARD.encode(b"x"),
    })
    .to_string();
    let req = format!(
        "POST /api/v1/workers/{}/files HTTP/1.1\r\nHost: x\r\nX-Argo-Role: guest\r\n\r\n{body}",
        worker.id
    );
    let (status, _b) = upload_response(&worker.id.to_string(), &req, Some(&store)).await;
    assert_eq!(status, "403 Forbidden");
}
