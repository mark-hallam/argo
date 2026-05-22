//! Tests for `routes_workers_hire` — Phase 3 body-level
//! `idempotency_key` + 1h-windowed dedupe (Stream J5 / brief
//! `claude-web-hire-idempotency-key`).
//!
//! Split out of `routes_workers_hire_tests.rs` to keep both files
//! under the 500-LOC ceiling. Test-helper fixtures (`env_lock`,
//! `mem_store`, `hire_req`, etc.) are re-exported from the sibling
//! `tests` module via `pub(super)`.

#![allow(clippy::too_many_lines)]

use super::tests::{
    count_worker_rows, env_lock, hire_req, mem_store, set_test_env, valid_body,
};
use super::{hire_response, reset_idem_cache_for_tests};
use serde_json::{json, Value};

fn body_with_idem_key(key: &str) -> String {
    let mut v: Value = serde_json::from_str(&valid_body()).unwrap();
    v["idempotency_key"] = json!(key);
    v.to_string()
}

/// Same `idempotency_key` submitted twice → exactly one `worker` row
/// is persisted, second response is `200 OK` carrying `duplicate_of`
/// pointing at the first row's id. This is the load-bearing acceptance
/// gate for the brief.
#[tokio::test]
async fn hire_body_idempotency_key_dedupes_to_single_row() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let body = body_with_idem_key("0190f000-0000-7000-8000-000000000001");

    let (status1, body1) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status1, "201 Created", "first hire: {body1}");
    let first_id = serde_json::from_str::<Value>(&body1).unwrap()["worker_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(count_worker_rows(&store).await, 1);

    let (status2, body2) = hire_response(&hire_req(&body), Some(&store)).await;
    assert_eq!(status2, "200 OK", "second hire: {body2}");
    let v2: Value = serde_json::from_str(&body2).unwrap();
    assert_eq!(v2["worker_id"].as_str(), Some(first_id.as_str()));
    assert_eq!(v2["duplicate_of"].as_str(), Some(first_id.as_str()));
    assert_eq!(v2["message"].as_str(), Some("Already hired (idempotent)"));
    assert_eq!(
        count_worker_rows(&store).await,
        1,
        "second hire must not persist a duplicate row",
    );
}

/// Two distinct `idempotency_key` values from the same tenant → two
/// distinct worker rows. Guards against an over-eager cache that
/// dedupes on body alone.
#[tokio::test]
async fn hire_distinct_idempotency_keys_create_distinct_rows() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    reset_idem_cache_for_tests();
    let store = mem_store().await;
    let body_a = body_with_idem_key("0190f000-0000-7000-8000-00000000000a");
    let body_b = body_with_idem_key("0190f000-0000-7000-8000-00000000000b");

    let (status_a, raw_a) = hire_response(&hire_req(&body_a), Some(&store)).await;
    assert_eq!(status_a, "201 Created");
    let (status_b, raw_b) = hire_response(&hire_req(&body_b), Some(&store)).await;
    assert_eq!(status_b, "201 Created");

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

/// Cross-tenant isolation: the idempotency cache is keyed on
/// `(tenant_id, key)`, so a cache entry under tenant_a is invisible to
/// tenant_b. tenant_b sending the same key sees `IdempotencyHit::None`
/// and proceeds to create its own row (rather than receiving tenant_a's
/// worker id — which would be a tenant-isolation breach, mirroring the
/// canonical 403/404 pattern from the SessionStore boundary).
///
/// The handler binds tenant via `ensure_workspace(name, root)` from
/// cwd, so a full end-to-end cross-tenant drive would require
/// chdir-ing the process per hire. The cache module is what the
/// brief's invariant lives on, so the test exercises it directly:
/// remember under tenant_a, then probe under tenant_b → `None`.
#[tokio::test]
async fn hire_idempotency_cache_does_not_leak_across_tenants() {
    let _g = env_lock().await;
    reset_idem_cache_for_tests();
    let tenant_a = argo_types::TenantId::new();
    let tenant_b = argo_types::TenantId::new();
    assert_ne!(tenant_a, tenant_b);
    let key = "0190f000-0000-7000-8000-00000000000c";
    let digest = "deadbeef";
    let worker_a = argo_identity::WorkerId::new();

    super::remember_idempotency(tenant_a, key, digest, worker_a);

    // Same key, same digest, but tenant_b — must be a cache miss.
    let probe = super::check_idempotency(tenant_b, key, digest);
    assert!(matches!(probe, super::IdempotencyHit::None));

    // Sanity: tenant_a still gets the cached hit.
    let hit = super::check_idempotency(tenant_a, key, digest);
    assert!(matches!(
        hit,
        super::IdempotencyHit::Match(id) if id == worker_a
    ));
}
