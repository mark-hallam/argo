//! Tests for the Genesis-invocation branch of
//! `routes_onboarding::first_hire_response`.
//!
//! Split out of `routes_onboarding_tests.rs` to keep both files under
//! the 500-LOC budget. The fixture mirrors the sibling file
//! (`:memory:` sqlite, tempdir `ARGO_HOME`, workspace verticals dir,
//! the deterministic workspace root key from `mvp-credentials-cross-tenant`).

#![allow(clippy::too_many_lines)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use super::super::onboarding_provision::provision_response;
use super::first_hire_response;
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
    std::env::set_var(
        argo_credentials::WORKSPACE_ROOT_KEY_ENV,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
    );
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn req(path: &str, body: &str, principal: Option<&str>) -> String {
    let mut head = format!("POST {path} HTTP/1.1\r\nHost: x\r\n");
    if let Some(p) = principal {
        head.push_str(&format!("X-Argo-Principal: {p}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    head
}

async fn pre_provision(store: &Arc<SessionStore>, email: &str) {
    let body = json!({ "workspace_name": "Acme", "company_name": "Acme Co" }).to_string();
    let r = req(
        "/api/v1/onboarding/provision",
        &body,
        Some(&format!("human:{email}")),
    );
    let (status, body) = provision_response(&r, Some(store)).await;
    assert!(
        status.starts_with("201") || status.starts_with("200"),
        "pre-provision failed: {status} {body}"
    );
}

#[tokio::test]
async fn first_hire_invokes_genesis_and_persists_synthesised_charter() {
    // F5 closure: the first-hire endpoint must call `argo_genesis::plan`
    // and persist a synthesised charter shape — not the pre-Genesis
    // "First hire for this workspace..." preamble. The investor-demo
    // talk-track for Scene 3 depends on this code path actually running.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({
        "description": "Mariah, you book outbound demo calls and qualify leads",
        "vertical_id": "gtm-engineer",
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(status, "201 Created", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["genesis_invoked"], true, "genesis must run: {resp}");
    assert_eq!(v["genesis_fallback"], false);

    let charter: String = sqlx::query_scalar(
        "SELECT charter_md FROM worker WHERE vertical_id = 'gtm-engineer'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(
        !charter.starts_with("First hire for this workspace"),
        "charter must NOT be the pre-Genesis templated preamble: {charter}",
    );
    assert!(
        charter.contains("synthesised by Genesis"),
        "charter must narrate Genesis synthesis: {charter}",
    );

    // F9 closure: the audit row's endpoint flips to the
    // `genesis_invoked` variant + carries plan_id + charter_hash.
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE payload_json LIKE '%onboarding.first_hire.genesis_invoked%'",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert_eq!(rows.len(), 1, "expected one genesis_invoked audit row");
    let payload: Value = serde_json::from_str(&rows[0]).unwrap();
    assert_eq!(payload["endpoint"], "onboarding.first_hire.genesis_invoked");
    assert_eq!(payload["genesis_invoked"], true);
    assert_eq!(payload["genesis_fallback"], false);
    assert!(
        payload["plan_id"].as_str().is_some_and(|s| !s.is_empty()),
        "plan_id must be present: {payload}",
    );
    assert!(
        payload["charter_hash"]
            .as_str()
            .is_some_and(|s| s.len() == 64),
        "charter_hash must be a 64-char hex digest: {payload}",
    );
}

#[tokio::test]
async fn first_hire_synthesises_multiple_duties_from_picked_vertical_actions() {
    // F10 closure: the worker must carry duties derived from the picked
    // vertical's autonomous-safe actions, NOT a single hardcoded
    // `daily_check_in`. gtm-engineer declares three low-risk
    // no-approval actions (create_listing_draft, verify_price,
    // request_photos) so the worker should land with at least two
    // matching duties.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({
        "description": "Mariah, you book outbound demo calls",
        "vertical_id": "gtm-engineer",
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(status, "201 Created", "body: {resp}");

    let entry_actions: Vec<String> = sqlx::query_scalar(
        "SELECT entry_action FROM duty d \
         JOIN worker w ON w.id = d.worker_id \
         WHERE w.vertical_id = 'gtm-engineer' ORDER BY entry_action",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert!(
        entry_actions.len() >= 2,
        "expected ≥2 duties; got: {entry_actions:?}",
    );
    assert!(
        !entry_actions.iter().any(|a| a == "daily_check_in"),
        "the hardcoded daily_check_in duty must not appear when the \
         vertical declares autonomous-safe actions: {entry_actions:?}",
    );
}

#[tokio::test]
async fn first_hire_falls_back_to_templated_charter_when_genesis_errors() {
    // Fallback path: the operator's brief doesn't classify into any
    // archetype, Genesis returns UnknownArchetype, and the wizard
    // still completes. The audit row flags `genesis_fallback=true`.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({
        // "Pioneer" has a capital letter so `name_from_description`
        // returns it, but the rest of the brief intentionally avoids
        // every archetype keyword so `classify_charter` returns
        // `UnknownArchetype` and we exercise the fallback branch.
        "description": "Pioneer xyzzy plugh blorf nonsense filler",
        "vertical_id": "gtm-engineer",
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(status, "201 Created", "wizard must not hard-fail: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["genesis_invoked"], false);
    assert_eq!(v["genesis_fallback"], true);

    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE payload_json LIKE '%onboarding.first_hire.genesis_fallback%'",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert_eq!(rows.len(), 1, "expected one genesis_fallback audit row");
    let payload: Value = serde_json::from_str(&rows[0]).unwrap();
    assert_eq!(
        payload["endpoint"],
        "onboarding.first_hire.genesis_fallback",
    );
    assert_eq!(payload["genesis_fallback"], true);
    assert_eq!(payload["fallback_reason"], "unknown_archetype");
    assert!(payload["plan_id"].is_null());

    let charter: String = sqlx::query_scalar(
        "SELECT charter_md FROM worker WHERE vertical_id = 'gtm-engineer'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(
        charter.starts_with("First hire for this workspace"),
        "fallback charter must preserve the pre-Genesis preamble: {charter}",
    );
}
