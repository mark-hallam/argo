//! Tests for `routes_worker_activity` — `GET /api/v1/workers/<id>/activity`
//! (Stream J9). Seeds a `:memory:` store with synthesized
//! `audit_record` rows whose `payload_json` carries `worker_id`,
//! `action_type`, etc., then drives `list_response` directly. The
//! bearer-auth gate lives upstream in `routes.rs::dispatch_inner`.

#![allow(clippy::too_many_lines)]

use super::list_response;
use argo_audit::{AuditKind, AuditSink};
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
    let root = cwd.to_string_lossy();
    let (tenant, _) = store
        .ensure_workspace(name, &root)
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
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

async fn seed_row(store: &Arc<SessionStore>, payload: Value) {
    AuditSink::new(Arc::clone(store))
        .write(None, AuditKind::HttpRequest, payload, None)
        .await
        .expect("write audit row");
}

fn row_payload(
    worker_id: &WorkerId,
    action_type: &str,
    status: &str,
    verb: &str,
    approver: Option<&str>,
) -> Value {
    let mut v = json!({
        "endpoint": "test.action",
        "worker_id": worker_id.to_string(),
        "worker_name": "stale-name",
        "action_type": action_type,
        "outcome_status": status,
        "verb": verb,
        "channel": "web",
        "data_classification": "internal",
        "cost_usd": 0.5_f64,
    });
    if let Some(a) = approver {
        v["approver_id"] = json!(a);
        v["approver_name"] = json!(format!("Approver-{a}"));
    }
    v
}

fn get_req(worker_id: &WorkerId, query: &str) -> String {
    if query.is_empty() {
        format!("GET /api/v1/workers/{worker_id}/activity HTTP/1.1\r\nHost: x\r\n\r\n")
    } else {
        format!("GET /api/v1/workers/{worker_id}/activity?{query} HTTP/1.1\r\nHost: x\r\n\r\n")
    }
}

async fn call(
    worker_id: &WorkerId,
    query: &str,
    store: &Arc<SessionStore>,
) -> (&'static str, Value) {
    let req = get_req(worker_id, query);
    let (status, body) = list_response(&worker_id.to_string(), &req, Some(store)).await;
    let v: Value = serde_json::from_str(&body)
        .unwrap_or_else(|_| panic!("invalid JSON body: {body}"));
    (status, v)
}

#[tokio::test]
async fn returns_200_with_items_and_required_shape() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    seed_row(
        &store,
        row_payload(&worker.id, "message.sent", "succeeded", "wrote update", Some("p1")),
    )
    .await;
    let (status, body) = call(&worker.id, "", &store).await;
    assert_eq!(status, "200 OK");
    let items = body["items"].as_array().expect("items array");
    assert_eq!(items.len(), 1, "expected exactly the seeded row");
    let item = &items[0];
    for key in [
        "id",
        "timestamp",
        "worker",
        "action_type",
        "verb",
        "status",
        "approver",
        "channel",
        "cost",
        "data_classification",
    ] {
        assert!(item.get(key).is_some(), "missing key {key}: {item}");
    }
    assert_eq!(item["worker"]["id"], worker.id.to_string());
    // Live persona name wins over the audit row's stale embedded name.
    assert_eq!(item["worker"]["name"], "Maya");
}

#[tokio::test]
async fn scopes_rows_to_path_worker_id() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let a = add_worker(&store, tenant, "Maya").await;
    let b = add_worker(&store, tenant, "Devon").await;
    seed_row(
        &store,
        row_payload(&a.id, "message.sent", "succeeded", "for A", None),
    )
    .await;
    seed_row(
        &store,
        row_payload(&b.id, "message.sent", "succeeded", "for B", None),
    )
    .await;
    let (_, body) = call(&a.id, "", &store).await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    for it in items {
        assert_eq!(it["worker"]["id"], a.id.to_string());
        assert_eq!(it["verb"], "for A");
    }
}

#[tokio::test]
async fn filters_by_action_type_and_status() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    seed_row(
        &store,
        row_payload(&worker.id, "message.sent", "succeeded", "ok msg", None),
    )
    .await;
    seed_row(
        &store,
        row_payload(&worker.id, "message.sent", "failed", "boom", None),
    )
    .await;
    seed_row(
        &store,
        row_payload(&worker.id, "approval.decided", "succeeded", "approved", None),
    )
    .await;
    let (_, by_type) = call(&worker.id, "type=approval.decided", &store).await;
    let items = by_type["items"].as_array().unwrap();
    assert!(items.iter().all(|it| it["action_type"] == "approval.decided"));
    assert!(items.iter().any(|it| it["verb"] == "approved"));
    let (_, by_status) =
        call(&worker.id, "type=message.sent&status=failed", &store).await;
    let items = by_status["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["status"], "failed");
}

#[tokio::test]
async fn cross_tenant_worker_returns_403() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let other = TenantId::new();
    let alien = add_worker(&store, other, "Alien").await;
    let (status, body) = call(&alien.id, "", &store).await;
    assert_eq!(status, "403 Forbidden", "body: {body}");
    assert_eq!(body["error"]["code"], "forbidden");
}

#[tokio::test]
async fn unknown_worker_id_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let ghost = WorkerId::new();
    let (status, body) = call(&ghost, "", &store).await;
    assert_eq!(status, "404 Not Found", "body: {body:?}");
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn archived_worker_returns_410() {
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
    let (status, body) = call(&worker.id, "", &store).await;
    assert_eq!(status, "410 Gone", "body: {body:?}");
    assert_eq!(body["error"]["code"], "gone");
    assert!(body["archived_at"].is_string());
    assert!(body["recoverable_until_at"].is_string());
}

#[tokio::test]
async fn returns_503_when_store_missing() {
    let id = WorkerId::new();
    let req = get_req(&id, "");
    let (status, body) = list_response(&id.to_string(), &req, None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("session store unavailable"));
}

#[tokio::test]
async fn bad_worker_id_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let req = "GET /api/v1/workers/not-a-uuid/activity HTTP/1.1\r\nHost: x\r\n\r\n";
    let (status, body) = list_response("not-a-uuid", req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn rejects_invalid_limit_and_range_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let (status, body) = call(&worker.id, "limit=0", &store).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body["error"]["message"].as_str().unwrap().contains("limit"));
    let (status, body) = call(&worker.id, "limit=200", &store).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body["error"]["message"].as_str().unwrap().contains("limit"));
    let (status, body) = call(&worker.id, "range=zzz", &store).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body["error"]["message"].as_str().unwrap().contains("range"));
    let (status, body) = call(&worker.id, "cursor=not_b64!!!", &store).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body["error"]["message"].as_str().unwrap().contains("cursor"));
}

#[tokio::test]
async fn pagination_emits_cursor_and_continues() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    for i in 0..5 {
        seed_row(
            &store,
            row_payload(
                &worker.id,
                "message.sent",
                "succeeded",
                &format!("msg {i}"),
                None,
            ),
        )
        .await;
    }
    let (_, page1) = call(&worker.id, "limit=2", &store).await;
    let first_items = page1["items"].as_array().unwrap().clone();
    assert_eq!(first_items.len(), 2);
    let cursor = page1["next_cursor"]
        .as_str()
        .expect("next_cursor present on full page");
    let (_, page2) = call(&worker.id, &format!("limit=2&cursor={cursor}"), &store).await;
    let second_items = page2["items"].as_array().unwrap().clone();
    assert_eq!(second_items.len(), 2);
    let first_ids: Vec<&str> = first_items
        .iter()
        .map(|it| it["id"].as_str().unwrap())
        .collect();
    for it in &second_items {
        assert!(
            !first_ids.contains(&it["id"].as_str().unwrap()),
            "page 2 row {it} also appeared on page 1"
        );
    }
}

#[tokio::test]
async fn redacts_email_in_verb_by_default() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    seed_row(
        &store,
        row_payload(
            &worker.id,
            "message.sent",
            "succeeded",
            "ping sam@acmeco.com to confirm",
            None,
        ),
    )
    .await;
    let (_, body) = call(&worker.id, "", &store).await;
    let items = body["items"].as_array().unwrap();
    let verbs: Vec<&str> = items.iter().filter_map(|it| it["verb"].as_str()).collect();
    assert!(verbs.iter().any(|v| v.contains("[EMAIL]")));
    assert!(verbs.iter().all(|v| !v.contains("sam@acmeco.com")));
}

#[tokio::test]
async fn emits_audit_row_per_view() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%worker.activity.viewed%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let (status, _) = call(&worker.id, "", &store).await;
    assert_eq!(status, "200 OK");
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%worker.activity.viewed%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(after, before + 1, "expected exactly one worker.activity.viewed row");
}

#[tokio::test]
async fn cursor_is_opaque_base64_round_trip() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    for i in 0..4 {
        seed_row(
            &store,
            row_payload(
                &worker.id,
                "message.sent",
                "succeeded",
                &format!("r{i}"),
                None,
            ),
        )
        .await;
    }
    let (_, body) = call(&worker.id, "limit=2", &store).await;
    let cursor = body["next_cursor"].as_str().expect("cursor present");
    // Client passes back unchanged → no extra processing required.
    assert!(!cursor.is_empty());
    assert!(!cursor.contains('|'), "cursor should not leak the encoded tuple");
}
