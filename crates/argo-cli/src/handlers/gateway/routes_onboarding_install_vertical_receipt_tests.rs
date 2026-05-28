//! Regression tests for UX-AUDIT-T0119MEL F8 (P1):
//! `POST /api/v1/onboarding/install-vertical` must surface
//! `memory_schema_attached` + `approval_gates_added` alongside the
//! existing `skills_added`, so the wizard's 3-row "what just got
//! installed" receipt matches the sub-copy promise.
//!
//! The happy-path coverage (gtm-engineer: memory schema + ≥1 gate)
//! lives inline in `routes_onboarding_tests::install_vertical_happy_path_returns_counts`.
//! This sibling file pins the "pack ships zero gates" branch (the
//! receptionist pack's `human_approval_required = []`) so the SPA's
//! "no approval gates" rendering can't silently regress to the
//! all-gates path.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use super::super::onboarding_provision::provision_response;
use super::install_vertical_response;
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

fn post_req(path: &str, body: &str, principal: &str) -> String {
    let mut head = format!("POST {path} HTTP/1.1\r\nHost: x\r\n");
    head.push_str(&format!("X-Argo-Principal: {principal}\r\n"));
    head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    head
}

async fn pre_provision(store: &Arc<SessionStore>, email: &str) {
    let body = json!({ "workspace_name": "Acme", "company_name": "Acme Co" }).to_string();
    let r = post_req(
        "/api/v1/onboarding/provision",
        &body,
        &format!("human:{email}"),
    );
    let (status, body) = provision_response(&r, Some(store)).await;
    assert!(
        status.starts_with("201") || status.starts_with("200"),
        "pre-provision failed: {status} {body}"
    );
}

#[tokio::test]
async fn install_vertical_receipt_reports_zero_gates_when_pack_has_none() {
    // The receptionist pack ships with `human_approval_required = []`
    // — the receipt's "approval gates" row must render the
    // "0 policies" branch rather than implying gates were attached.
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = json!({ "vertical_id": "receptionist" }).to_string();
    let r = post_req(
        "/api/v1/onboarding/install-vertical",
        &body,
        "human:alice@acme.com",
    );
    let (status, resp) = install_vertical_response(&r, Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    let v: Value = serde_json::from_str(&resp).unwrap();
    // Short-circuit the SPA uses for redirect must stay intact.
    assert_eq!(v["installed"], true);
    assert_eq!(v["vertical_id"], "receptionist");
    // Zero gates branch: the receipt's third row renders "No approval
    // gates in this pack" instead of "✓ Approval gates (N policies)".
    assert_eq!(v["approval_gates_added"], 0);
    // Memory schema is still attached — the receptionist pack does
    // ship a `[memory].schema`, so this row stays a green ✓.
    assert_eq!(v["memory_schema_attached"], true);
}
