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
use crate::handlers::gateway::routes_genesis_gate::test_overrides;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OwnedMutexGuard;

/// Serialize on the GATEWAY-WIDE env lock so onboarding tests can't run
/// concurrently with the genesis-stream / workers-hire test modules. The
/// first-hire path now runs the Genesis hire gate, which reads process-global
/// `ARGO_VERTICALS_DIR` (and a test-only forced-signal override); a private
/// per-module mutex would let a concurrent gate-driving test clobber that
/// state mid-hire.
pub(super) async fn env_lock() -> OwnedMutexGuard<()> {
    let guard = crate::handlers::gateway::shared_env_lock().await;
    test_overrides::clear_forced_verdict();
    guard
}

/// Force the next Genesis hire gate run (inside `first_hire_response`) to
/// return a clean `Pass`, so a *gate-cleared* first-hire can be driven
/// end-to-end to assert downstream persistence / name handling. CI has no live
/// safety-judge provider and these tests are not testing the gate; the
/// dedicated `first_hire_blocks_*` suite exercises the real gate. Must be
/// called while holding the [`env_lock`] guard; the override is one-shot.
pub(super) fn force_safety_allow() {
    test_overrides::force_pass();
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

/// Grant the provisioned tenant for `email` a PAID ("enterprise") tier.
///
/// WS4 tier-gate context: a freshly-provisioned tenant defaults to the FREE
/// tier, which (a) lacks the `provider_real_keys` feature (so
/// `provider_key_response` returns a `403 tier_forbidden`) and (b) caps
/// `max_workers` at 1 (so a second hire is refused with `403 tier_worker_cap`).
/// These tests exercise the provider-key / hire behaviour itself, NOT the tier
/// gate, so they provision then upgrade to a paid tier first. Resolves the
/// SAME idempotent `provision_for_user` tenant the onboarding handlers resolve
/// (so the tier lands on the exact tenant the request runs against), then
/// upserts the tier via `argo_store::tenant_quota::set_workspace_tier`.
pub(super) async fn grant_paid_tier(store: &Arc<SessionStore>, email: &str) {
    let user = argo_identity::user_store::get_user_by_email(store, email)
        .await
        .expect("get user")
        .expect("user provisioned before grant_paid_tier");
    let outcome = argo_identity::provision_for_user(
        store,
        argo_identity::ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: user.email.split('@').next().unwrap_or("workspace"),
            company_name: None,
        },
    )
    .await
    .expect("resolve provisioned tenant");
    argo_store::tenant_quota::set_workspace_tier(store, outcome.tenant_id, "enterprise", "Acme")
        .await
        .expect("set enterprise tier");
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
