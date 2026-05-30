//! Genesis P4 role-contract operator-confirmation tests for the
//! `routes_onboarding::first_hire_response` seam (DEC-025 #1 + #4).
//!
//! Split out of `routes_onboarding_first_hire_genesis_tests.rs` to keep both
//! files under the 500-LOC budget. Same fixture (`:memory:` sqlite, tempdir
//! `ARGO_HOME`, workspace verticals dir). These prove the no-activation-
//! without-a-confirmed-contract invariant END-TO-END at the first-hire HTTP
//! seam (no test-override — the real gate runs).

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use super::super::onboarding_provision::provision_response;
use super::first_hire_response;
use crate::handlers::gateway::routes_genesis_gate::test_overrides;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OwnedMutexGuard;

async fn env_lock() -> OwnedMutexGuard<()> {
    let guard = crate::handlers::gateway::shared_env_lock().await;
    test_overrides::clear_forced_verdict();
    guard
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

/// Count of persisted worker rows — a blocked hire must leave this at 0.
async fn worker_count(store: &Arc<SessionStore>) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM worker")
        .fetch_one(store.pool())
        .await
        .unwrap()
}

/// The parsed payload of the single first-hire genesis-gate audit row.
async fn first_hire_gate_audit(store: &Arc<SessionStore>) -> Value {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE payload_json LIKE '%onboarding.first_hire.genesis_gate%'",
    )
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert!(!rows.is_empty(), "expected a first-hire gate audit row");
    serde_json::from_str(&rows[0]).unwrap()
}

#[tokio::test]
async fn first_hire_without_contract_confirmation_does_not_activate() {
    // ADVERSARIAL (DEC-025 #1 + #4 — no activation without a confirmed role
    // contract, at the first-hire seam). A rich brief that clears the eval
    // floor, but with NO `confirmed_contract_digest` in the request body, must
    // NOT 201. The end-to-end audit row proves the contract axis ran and is
    // `unconfirmed` (human-review-before-activation held the hire). No override
    // here — the real gate runs.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    // NB: body has NO confirmed_contract_digest.
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
    assert_eq!(
        status, "422 Unprocessable Entity",
        "an unconfirmed role contract must NOT activate a teammate: {resp}"
    );
    assert_eq!(0i64, worker_count(&store).await, "no worker may persist");

    // The audit row carries the contract-confirmation axis = unconfirmed.
    let payload = first_hire_gate_audit(&store).await;
    assert_eq!(
        payload["gate_contract_confirmation"], "unconfirmed",
        "audit must record the contract was unconfirmed: {payload}"
    );
}

#[tokio::test]
async fn first_hire_with_a_forged_contract_digest_is_hard_blocked() {
    // ADVERSARIAL (forgery/staleness at the first-hire seam): a body that
    // supplies a digest NOT matching the teammate's truthful contract is a
    // HARD block — a stale/forged confirmation can never activate a teammate.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({
        "description": "Mariah, you book outbound demo calls and qualify leads",
        "vertical_id": "gtm-engineer",
        "confirmed_contract_digest": "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
    })
    .to_string();
    let r = req(
        "/api/v1/onboarding/first-hire",
        &body,
        Some("human:alice@acme.com"),
    );
    let (status, resp) = first_hire_response(&r, Some(&store)).await;
    assert_eq!(status, "422 Unprocessable Entity", "forged digest must block: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(v["gate"]["state"], "blocked", "mismatch is a HARD block: {resp}");
    assert_eq!(v["gate"]["contract_confirmation"], "mismatch");
    assert_eq!(
        0i64,
        worker_count(&store).await,
        "no worker may persist on a forged confirmation"
    );
}
