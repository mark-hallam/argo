//! Tests for `routes_workers_hire` — `POST /api/v1/workers` (Stream J5).
//!
//! Uses a `:memory:` SQLite store + an `ARGO_VERTICALS_DIR` pointed at
//! the workspace `verticals/` directory so the catalog lookup resolves
//! real vertical ids. The auth gate is exercised at the dispatch layer
//! (`routes_dispatch_post_tests`) — these tests drive the handler
//! directly so the surface area stays inside the one crate.

#![allow(clippy::too_many_lines)]

use super::{hire_response, reset_idem_cache_for_tests};
use argo_store::SessionStore;
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

fn hire_req(body: &str) -> String {
    format!(
        "POST /api/v1/workers HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn hire_req_with_idem(body: &str, key: &str) -> String {
    format!(
        "POST /api/v1/workers HTTP/1.1\r\nHost: x\r\nIdempotency-Key: {key}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn valid_draft() -> Value {
    json!({
        "suggestedName": "Maya",
        "charter": "Own outbound marketplace follow-up end-to-end for B2B customers.",
        "suggestedVertical": "gtm-engineer",
        "approvalPolicy": {"level": "balanced"},
        "duties": [
            {
                "cron": "0 9 * * 1-5",
                "entryAction": "follow_up",
                "description": "Send approved follow-up.",
                "defaultEnabled": true,
            }
        ],
        "tagline": "GTM follow-up persona",
    })
}

fn valid_body() -> String {
    json!({
        "worker_draft": valid_draft(),
        "problem": "Need outbound follow-up across our weekly marketplace leads.",
        "answers": {
            "role": "marketplace seller",
            "channels": ["slack-dm", "email"],
        },
    })
    .to_string()
}

#[tokio::test]
async fn hire_happy_path_returns_201_and_worker_id() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let (status, body) = hire_response(&hire_req(&valid_body()), Some(&store)).await;
    assert_eq!(status, "201 Created", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["success"], true);
    assert!(v["worker_id"].as_str().is_some_and(|s| !s.is_empty()));
}

#[tokio::test]
async fn hire_persists_worker_with_persona_and_duties() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let (status, body) = hire_response(&hire_req(&valid_body()), Some(&store)).await;
    assert_eq!(status, "201 Created");
    let id_str = serde_json::from_str::<Value>(&body).unwrap()["worker_id"]
        .as_str()
        .unwrap()
        .to_string();
    let worker_id: argo_identity::WorkerId = id_str.parse().expect("worker_id parses");

    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store.ensure_workspace(name, &root).await.expect("workspace");
    let identities = argo_identity::IdentityStore::new(&store);
    let worker = identities
        .get_worker(tenant, worker_id)
        .await
        .expect("get")
        .expect("present");
    assert_eq!(worker.persona.name, "Maya");
    assert_eq!(worker.vertical_id, "gtm-engineer");
    assert_eq!(worker.role.duties.len(), 1);
    assert_eq!(worker.role.duties[0].entry_action, "follow_up");
    // balanced → auto-approval disabled
    assert!(!worker.auto_approval.enabled);
}

#[tokio::test]
async fn hire_rejects_empty_problem_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let body = json!({
        "worker_draft": valid_draft(),
        "problem": "",
        "answers": {},
    })
    .to_string();
    let (status, body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn hire_rejects_missing_suggested_name_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let mut draft = valid_draft();
    draft["suggestedName"] = json!("");
    let body = json!({
        "worker_draft": draft,
        "problem": "Need outbound follow-up across our weekly marketplace leads.",
        "answers": {},
    })
    .to_string();
    let (status, body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert!(v["error"]["message"]
        .as_str()
        .unwrap()
        .contains("suggestedName"));
}

#[tokio::test]
async fn hire_rejects_short_charter_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let mut draft = valid_draft();
    draft["charter"] = json!("too short");
    let body = json!({
        "worker_draft": draft,
        "problem": "Need outbound follow-up across our weekly marketplace leads.",
        "answers": {},
    })
    .to_string();
    let (status, _body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn hire_rejects_unknown_vertical_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let mut draft = valid_draft();
    draft["suggestedVertical"] = json!("no-such-vertical");
    let body = json!({
        "worker_draft": draft,
        "problem": "Need outbound follow-up across our weekly marketplace leads.",
        "answers": {},
    })
    .to_string();
    let (status, body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("unknown vertical"));
}

#[tokio::test]
async fn hire_rejects_unknown_approval_policy_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let mut draft = valid_draft();
    draft["approvalPolicy"]["level"] = json!("yolo");
    let body = json!({
        "worker_draft": draft,
        "problem": "Need outbound follow-up across our weekly marketplace leads.",
        "answers": {},
    })
    .to_string();
    let (status, _body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn hire_rejects_no_enabled_duty_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let mut draft = valid_draft();
    draft["duties"][0]["defaultEnabled"] = json!(false);
    let body = json!({
        "worker_draft": draft,
        "problem": "Need outbound follow-up across our weekly marketplace leads.",
        "answers": {},
    })
    .to_string();
    let (status, _body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn hire_idempotency_same_key_same_body_returns_same_worker_id() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let body = valid_body();
    let key = "key-1";
    let (status1, body1) = hire_response(&hire_req_with_idem(&body, key), Some(&store)).await;
    assert_eq!(status1, "201 Created");
    let first_id = serde_json::from_str::<Value>(&body1).unwrap()["worker_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status2, body2) = hire_response(&hire_req_with_idem(&body, key), Some(&store)).await;
    assert_eq!(status2, "200 OK");
    let second_id = serde_json::from_str::<Value>(&body2).unwrap()["worker_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(first_id, second_id);
}

#[tokio::test]
async fn hire_idempotency_same_key_different_body_returns_409() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let body1 = valid_body();
    let key = "key-2";
    let (status1, _) = hire_response(&hire_req_with_idem(&body1, key), Some(&store)).await;
    assert_eq!(status1, "201 Created");
    // Change one field to make the body different.
    let mut alt: Value = serde_json::from_str(&body1).unwrap();
    alt["worker_draft"]["suggestedName"] = json!("Other");
    let (status2, body2) = hire_response(&hire_req_with_idem(&alt.to_string(), key), Some(&store))
        .await;
    assert_eq!(status2, "409 Conflict");
    let v: Value = serde_json::from_str(&body2).unwrap();
    assert_eq!(v["error"]["code"], "idempotency_conflict");
}

#[tokio::test]
async fn hire_writes_audit_row_with_redacted_email() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let body = json!({
        "worker_draft": valid_draft(),
        "problem": "Reach out to sam@acmeco.com about the weekly follow-up.",
        "answers": {"contact": "sam@acmeco.com"},
    })
    .to_string();
    let (status, _body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "201 Created");

    // Pull the most-recent audit row and confirm the email was redacted.
    let pool = store.pool();
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE kind = 'http.request' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("audit row");
    assert!(
        payload.contains("[EMAIL]"),
        "audit payload should redact email; got: {payload}"
    );
    assert!(
        !payload.contains("sam@acmeco.com"),
        "raw email leaked to audit"
    );
}

#[tokio::test]
async fn hire_handler_returns_503_when_store_missing() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let (status, body) = hire_response(&hire_req(&valid_body()), None).await;
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("session store unavailable"));
}

#[tokio::test]
async fn hire_handler_rejects_bad_json_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let (status, body) = hire_response(&hire_req("not-json"), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(body.contains("parse body"));
}

#[tokio::test]
async fn hire_aggressive_policy_enables_auto_approval_on_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let mut draft = valid_draft();
    draft["approvalPolicy"]["level"] = json!("aggressive");
    let body = json!({
        "worker_draft": draft,
        "problem": "Need outbound follow-up across our weekly marketplace leads.",
        "answers": {},
    })
    .to_string();
    let (status, body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "201 Created");
    let id_str = serde_json::from_str::<Value>(&body).unwrap()["worker_id"]
        .as_str()
        .unwrap()
        .to_string();
    let worker_id: argo_identity::WorkerId = id_str.parse().expect("worker_id parses");

    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store.ensure_workspace(name, &root).await.expect("workspace");
    let identities = argo_identity::IdentityStore::new(&store);
    let worker = identities
        .get_worker(tenant, worker_id)
        .await
        .expect("get")
        .expect("present");
    assert!(worker.auto_approval.enabled);
}
