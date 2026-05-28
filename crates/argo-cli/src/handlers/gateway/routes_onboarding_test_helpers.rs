//! Shared test fixture helpers for `routes_onboarding_tests.rs` (and the
//! sibling `routes_onboarding_first_hire_genesis_tests.rs`).
//!
//! Split out of `routes_onboarding_tests.rs` per HARD RULE 6 (500-LOC
//! ceiling) — see PR #2524 push it to 502 LOC; conductor mechanical
//! extraction restores headroom.

#![cfg(test)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use crate::handlers::gateway::onboarding_provision::provision_response;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

pub(super) async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

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
    std::env::set_var(
        argo_credentials::WORKSPACE_ROOT_KEY_ENV,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
    );
}

pub(super) async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

pub(super) fn req(path: &str, body: &str, principal: Option<&str>) -> String {
    let mut head = format!("POST {path} HTTP/1.1\r\nHost: x\r\n");
    if let Some(p) = principal {
        head.push_str(&format!("X-Argo-Principal: {p}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    head
}

pub(super) async fn pre_provision(store: &Arc<SessionStore>, email: &str) {
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

pub(super) async fn audit_payloads_for_kind(store: &SessionStore, kind: &str) -> Vec<String> {
    sqlx::query_scalar("SELECT payload_json FROM audit_record WHERE kind = ? ORDER BY seq ASC")
        .bind(kind)
        .fetch_all(store.pool())
        .await
        .unwrap()
}

// Used by some sibling tests that assert on the raw `Value`.
#[allow(dead_code)]
pub(super) async fn audit_values_for_kind(store: &SessionStore, kind: &str) -> Vec<Value> {
    audit_payloads_for_kind(store, kind)
        .await
        .into_iter()
        .map(|s| serde_json::from_str(&s).expect("payload json"))
        .collect()
}
