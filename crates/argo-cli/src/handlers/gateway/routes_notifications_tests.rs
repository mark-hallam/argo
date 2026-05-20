//! Tests for the bell-popover notifications gateway routes
//! (Stream `web-gateway-endpoints-notifications-batch`).
//!
//! The handlers resolve the workspace tenant from cwd (same convention
//! as `routes_team_list`), so tests pin `ARGO_HOME` + `ARGO_VERTICALS_DIR`
//! and share an env lock so the cwd-derived tenant stays deterministic
//! across the test set.

#![allow(clippy::too_many_lines)]

use super::{dispatch_get, dispatch_post};
use argo_store::notifications::{insert, NotificationInsert};
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

fn req_get(path: &str, principal: Option<&str>) -> String {
    let mut s = format!("GET {path} HTTP/1.1\r\nHost: x\r\n");
    if let Some(p) = principal {
        s.push_str(&format!("X-Argo-Principal: {p}\r\n"));
    }
    s.push_str("\r\n");
    s
}

fn req_post(path: &str, principal: Option<&str>) -> String {
    let mut s = format!("POST {path} HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n");
    if let Some(p) = principal {
        s.push_str(&format!("X-Argo-Principal: {p}\r\n"));
    }
    s.push_str("\r\n");
    s
}

async fn seed(store: &Arc<SessionStore>, tenant: TenantId, id: &str, principal: Option<&str>, ts: i64) {
    insert(
        store.pool(),
        &NotificationInsert {
            id,
            tenant_id: &tenant.to_string(),
            principal_id: principal,
            kind: "duty-output-ready",
            body: "Body for test",
            created_at: ts,
            related_audit_id: None,
            severity: "info",
        },
    )
    .await
    .expect("insert");
}

async fn dispatch_get_unwrap(req: &str, store: &Arc<SessionStore>) -> (&'static str, String) {
    dispatch_get(req, Some(store))
        .await
        .expect("dispatch_get matched")
}

async fn dispatch_post_unwrap(req: &str, store: &Arc<SessionStore>) -> (&'static str, String) {
    dispatch_post(req, Some(store))
        .await
        .expect("dispatch_post matched")
}

#[tokio::test]
async fn list_returns_envelope_with_items() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    seed(&store, tenant, "nt_a", Some("human:alice"), 100).await;
    seed(&store, tenant, "nt_b", Some("human:alice"), 200).await;

    let r = req_get("/api/v1/notifications", Some("human:alice"));
    let (status, body) = dispatch_get_unwrap(&r, &store).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).unwrap();
    let items = v["items"].as_array().expect("items array");
    assert_eq!(items.len(), 2);
    // Newest-first by created_at.
    assert_eq!(items[0]["id"], "nt_b");
    assert_eq!(items[1]["id"], "nt_a");
    assert_eq!(v["has_more"], false);
    assert!(v["cursor"].is_null());
}

#[tokio::test]
async fn list_pagination_returns_cursor() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    for n in 0..3 {
        seed(
            &store,
            tenant,
            &format!("nt_{n}"),
            Some("human:alice"),
            100 + i64::from(n),
        )
        .await;
    }

    let r = req_get("/api/v1/notifications?limit=2", Some("human:alice"));
    let (status, body) = dispatch_get_unwrap(&r, &store).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["items"].as_array().unwrap().len(), 2);
    assert_eq!(v["has_more"], true);
    let cursor = v["cursor"].as_i64().expect("cursor present");
    assert_eq!(cursor, 101);
}

#[tokio::test]
async fn unread_count_endpoint_returns_count() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    seed(&store, tenant, "nt_a", Some("human:alice"), 100).await;
    seed(&store, tenant, "nt_b", Some("human:alice"), 200).await;

    let r = req_get("/api/v1/notifications/unread-count", Some("human:alice"));
    let (status, body) = dispatch_get_unwrap(&r, &store).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["unread"], 2);
}

#[tokio::test]
async fn dismiss_one_idempotent_and_removes_from_list() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    seed(&store, tenant, "nt_a", Some("human:alice"), 100).await;

    let r = req_post("/api/v1/notifications/nt_a/dismiss", Some("human:alice"));
    let (status, body) = dispatch_post_unwrap(&r, &store).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["notificationId"], "nt_a");

    // Second call still 200 — idempotent.
    let (status2, _) = dispatch_post_unwrap(&r, &store).await;
    assert_eq!(status2, "200 OK");

    // List excludes the dismissed row.
    let list = req_get("/api/v1/notifications", Some("human:alice"));
    let (_, body) = dispatch_get_unwrap(&list, &store).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn dismiss_all_marks_every_visible_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    seed(&store, tenant, "nt_a", Some("human:alice"), 100).await;
    seed(&store, tenant, "nt_b", Some("human:alice"), 200).await;
    seed(&store, tenant, "nt_c", Some("human:bob"), 200).await;

    let r = req_post("/api/v1/notifications/dismiss-all", Some("human:alice"));
    let (status, body) = dispatch_post_unwrap(&r, &store).await;
    assert_eq!(status, "200 OK");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["dismissed"], 2);

    // Alice now has nothing visible; bob still has his row.
    let bob = req_get("/api/v1/notifications", Some("human:bob"));
    let (_, body) = dispatch_get_unwrap(&bob, &store).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn cross_principal_cannot_see_other_inbox() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    seed(&store, tenant, "nt_a", Some("human:alice"), 100).await;
    seed(&store, tenant, "nt_b", Some("human:bob"), 100).await;

    let alice = req_get("/api/v1/notifications", Some("human:alice"));
    let (_, body) = dispatch_get_unwrap(&alice, &store).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    let items = v["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "nt_a");
}

#[tokio::test]
async fn cross_principal_cannot_dismiss_others_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    seed(&store, tenant, "nt_bob", Some("human:bob"), 100).await;

    let r = req_post("/api/v1/notifications/nt_bob/dismiss", Some("human:alice"));
    let (status, body) = dispatch_post_unwrap(&r, &store).await;
    assert_eq!(status, "200 OK"); // idempotent contract
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["ok"], true);

    // Bob still sees his row — alice's dismiss didn't touch it.
    let bob = req_get("/api/v1/notifications", Some("human:bob"));
    let (_, body) = dispatch_get_unwrap(&bob, &store).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    let items = v["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
}

#[tokio::test]
async fn broadcast_rows_are_visible_to_every_principal() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    seed(&store, tenant, "nt_bx", None, 100).await;

    let alice = req_get("/api/v1/notifications", Some("human:alice"));
    let (_, body) = dispatch_get_unwrap(&alice, &store).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["items"].as_array().unwrap().len(), 1);

    let bob = req_get("/api/v1/notifications", Some("human:bob"));
    let (_, body) = dispatch_get_unwrap(&bob, &store).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn dismiss_one_rejects_malformed_id() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    // Empty id after stripping `/dismiss` is rejected before the SQL.
    // We can't easily hit empty-id via the path matcher (it bails at
    // the `!id.is_empty()` guard), so this exercises the inner helper
    // by calling the POST dispatcher with a non-alphanumeric id.
    let r = req_post(
        "/api/v1/notifications/has-space%20bad/dismiss",
        Some("human:alice"),
    );
    let (status, body) = dispatch_post_unwrap(&r, &store).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("invalid notification id"));
}

#[tokio::test]
async fn handlers_return_503_when_store_missing() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let list = dispatch_get(&req_get("/api/v1/notifications", None), None)
        .await
        .unwrap();
    assert_eq!(list.0, "503 Service Unavailable");
    let count = dispatch_get(&req_get("/api/v1/notifications/unread-count", None), None)
        .await
        .unwrap();
    assert_eq!(count.0, "503 Service Unavailable");
    let dismiss = dispatch_post(
        &req_post("/api/v1/notifications/nt_a/dismiss", None),
        None,
    )
    .await
    .unwrap();
    assert_eq!(dismiss.0, "503 Service Unavailable");
    let dismiss_all = dispatch_post(
        &req_post("/api/v1/notifications/dismiss-all", None),
        None,
    )
    .await
    .unwrap();
    assert_eq!(dismiss_all.0, "503 Service Unavailable");
}
