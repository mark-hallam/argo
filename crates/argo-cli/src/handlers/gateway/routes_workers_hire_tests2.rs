//! Tests for `routes_workers_hire` (part 2 — store-missing / bad-json /
//! Genesis-fallback audit rows / aggressive-policy auto-approval). Split out
//! of `routes_workers_hire_tests.rs` to keep each file under the 500-LOC
//! ceiling. Test-helper fixtures (`env_lock`, `mem_store`, `hire_req`, etc.)
//! are re-used from the sibling `tests` module.

#![allow(clippy::too_many_lines)]

use super::tests::{env_lock, hire_req, mem_store, set_test_env, valid_body, valid_draft};
use super::{hire_response, reset_idem_cache_for_tests};
use serde_json::{json, Value};

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
async fn hire_with_genesis_fallback_emits_named_audit_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let body = json!({
        "worker_draft": valid_draft(),
        "problem": "Need outbound follow-up across our weekly marketplace leads.",
        "answers": {"role": "marketplace seller"},
        "genesis_fallback": true,
    })
    .to_string();
    let (status, _body) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status, "201 Created");

    let pool = store.pool();
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE kind = 'genesis.fallback_used'",
    )
    .fetch_one(pool)
    .await
    .expect("count");
    assert_eq!(count, 1, "expected exactly one genesis.fallback_used row");
}

#[tokio::test]
async fn hire_without_genesis_fallback_emits_no_fallback_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let (status, _body) = hire_response(&hire_req(&valid_body()), Some(&store)).await;
    assert_eq!(status, "201 Created");

    let pool = store.pool();
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE kind = 'genesis.fallback_used'",
    )
    .fetch_one(pool)
    .await
    .expect("count");
    assert_eq!(count, 0, "no fallback row should be emitted by default");
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
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store
        .ensure_workspace(name, &root)
        .await
        .expect("workspace");
    let identities = argo_identity::IdentityStore::new(&store);
    let worker = identities
        .get_worker(tenant, worker_id)
        .await
        .expect("get")
        .expect("present");
    assert!(worker.auto_approval.enabled);
}
