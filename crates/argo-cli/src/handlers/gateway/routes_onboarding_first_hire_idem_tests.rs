//! Idempotency tests for `POST /api/v1/onboarding/first-hire`
//! (defense-in-depth mirror of `routes_workers_hire_idem_tests.rs`).
//!
//! The SPA sends the SAME `idempotency_key` on the initial submit AND the
//! post-review CONFIRM call. These tests drive the CONFIRM path (which is
//! the only path that mints a worker — the needs_review submit returns 422
//! and creates nothing) and assert:
//!
//! 1. Two CONFIRM calls carrying the same key collapse to ONE `worker`
//!    row (the second is a `201` idempotent replay of the first).
//! 2. Two distinct keys from the same tenant create TWO rows.
//! 3. The shared `(tenant, key)` cache does not leak across tenants.
//!
//! Idempotency is consulted AFTER the Genesis hire gate clears, so each
//! confirm call forces the gate to a clean Pass via `test_overrides`
//! (one-shot per call) exactly as the sibling `second_call` regression
//! test does — these tests are about de-dupe, not the gate verdict.

#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use super::super::onboarding_provision::provision_response;
use super::super::routes_workers_hire::reset_idem_cache_for_tests;
use super::first_hire_response;
use crate::handlers::gateway::routes_genesis_gate::test_overrides;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OwnedMutexGuard;

/// Gateway-wide env lock: the first-hire path runs the Genesis hire gate
/// which reads process-global state (forced verdict + the SHARED
/// idempotency cache), so this module must serialize against the other
/// gate-/cache-driving test modules.
async fn env_lock() -> OwnedMutexGuard<()> {
    let guard = crate::handlers::gateway::shared_env_lock().await;
    test_overrides::clear_forced_verdict();
    reset_idem_cache_for_tests();
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

async fn count_worker_rows(store: &Arc<SessionStore>) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM worker")
        .fetch_one(store.pool())
        .await
        .expect("count worker rows")
}

/// A CONFIRM body — carries `confirmed_contract_digest` (so it is the
/// commit call, not the needs_review submit) plus an optional
/// `idempotency_key`. The gate is forced to Pass, so the digest value
/// itself is not checked here.
fn confirm_body(name: &str, description: &str, idem_key: Option<&str>) -> String {
    let mut v = json!({
        "name": name,
        "description": description,
        "vertical_id": "gtm-engineer",
        "confirmed_contract_digest": "test-confirmed-digest",
    });
    if let Some(k) = idem_key {
        v["idempotency_key"] = json!(k);
    }
    v.to_string()
}

/// Drive ONE confirm call. Forces the gate to a clean Pass first (one-shot
/// per call — idempotency is consulted only after the gate clears).
async fn confirm(
    store: &Arc<SessionStore>,
    email: &str,
    body: &str,
) -> (&'static str, String) {
    test_overrides::force_pass();
    let r = post_req(
        "/api/v1/onboarding/first-hire",
        body,
        &format!("human:{email}"),
    );
    first_hire_response(&r, Some(store)).await
}

/// Same `idempotency_key` on two confirm calls → exactly one worker row;
/// the second response is a `201` idempotent replay pointing at the first
/// worker's id.
#[tokio::test]
async fn first_hire_same_idempotency_key_dedupes_to_single_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body = confirm_body(
        "Sara",
        "Sara, you draft outbound nurture emails for new signups.",
        Some("0190f000-0000-7000-8000-000000000001"),
    );

    let (status1, body1) = confirm(&store, "alice@acme.com", &body).await;
    assert_eq!(status1, "201 Created", "first confirm: {body1}");
    let first_id = serde_json::from_str::<Value>(&body1).unwrap()["worker_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(count_worker_rows(&store).await, 1);

    let (status2, body2) = confirm(&store, "alice@acme.com", &body).await;
    assert_eq!(status2, "201 Created", "second confirm: {body2}");
    let v2: Value = serde_json::from_str(&body2).unwrap();
    assert_eq!(
        v2["worker_id"].as_str(),
        Some(first_id.as_str()),
        "replay must echo the first worker id",
    );
    assert_eq!(v2["duplicate_of"].as_str(), Some(first_id.as_str()));
    assert_eq!(v2["idempotent"].as_bool(), Some(true));
    assert_eq!(
        count_worker_rows(&store).await,
        1,
        "second confirm must NOT persist a duplicate teammate",
    );
}

/// Two distinct `idempotency_key` values from the same tenant → two
/// distinct worker rows (guards against an over-eager body-only cache).
#[tokio::test]
async fn first_hire_distinct_idempotency_keys_create_distinct_rows() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    pre_provision(&store, "alice@acme.com").await;
    let body_a = confirm_body(
        "Sara",
        "Sara, you draft outbound nurture emails.",
        Some("0190f000-0000-7000-8000-00000000000a"),
    );
    let body_b = confirm_body(
        "Bryn",
        "Bryn, you triage inbound support tickets.",
        Some("0190f000-0000-7000-8000-00000000000b"),
    );

    let (status_a, raw_a) = confirm(&store, "alice@acme.com", &body_a).await;
    assert_eq!(status_a, "201 Created", "{raw_a}");
    let (status_b, raw_b) = confirm(&store, "alice@acme.com", &body_b).await;
    assert_eq!(status_b, "201 Created", "{raw_b}");

    let id_a = serde_json::from_str::<Value>(&raw_a).unwrap()["worker_id"]
        .as_str()
        .unwrap()
        .to_string();
    let id_b = serde_json::from_str::<Value>(&raw_b).unwrap()["worker_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(id_a, id_b);
    assert_eq!(count_worker_rows(&store).await, 2);
}

/// Cross-tenant isolation of the SHARED cache: an entry remembered under
/// tenant_a is invisible to tenant_b probing with the same key — so the
/// onboarding seam cannot replay another tenant's worker id (which would
/// be a tenant-isolation breach). The cache is the module-level invariant,
/// so exercise it directly (driving two real tenants end-to-end through
/// the cwd-derived resolver would require per-call chdir). Mirrors the
/// workers-hire `hire_idempotency_cache_does_not_leak_across_tenants` test
/// against the SAME underlying cache.
#[tokio::test]
async fn first_hire_idempotency_cache_does_not_leak_across_tenants() {
    let _g = env_lock().await;
    let tenant_a = argo_types::TenantId::new();
    let tenant_b = argo_types::TenantId::new();
    assert_ne!(tenant_a, tenant_b);
    let key = "0190f000-0000-7000-8000-00000000000c";
    let digest = "deadbeef";
    let worker_a = argo_identity::WorkerId::new();

    super::super::routes_workers_hire::remember_idempotency(tenant_a, key, digest, worker_a);

    // Same key + same digest, but tenant_b — must be a cache miss.
    let probe =
        super::super::routes_workers_hire::check_idempotency(tenant_b, key, digest);
    assert!(matches!(
        probe,
        super::super::routes_workers_hire::IdempotencyHit::None
    ));

    // Sanity: tenant_a still gets the cached hit for its own worker.
    let hit = super::super::routes_workers_hire::check_idempotency(tenant_a, key, digest);
    assert!(matches!(
        hit,
        super::super::routes_workers_hire::IdempotencyHit::Match(id) if id == worker_a
    ));
}
