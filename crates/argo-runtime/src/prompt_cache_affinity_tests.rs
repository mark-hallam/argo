//! Unit + integration tests for
//! [`crate::prompt_cache_affinity::PromptCacheAffinity`]. Cover the
//! load-bearing correctness invariants:
//!
//! - within-TTL reuse,
//! - post-TTL fresh mint,
//! - per-tenant isolation (same worker_id, different tenants),
//! - per-worker isolation (same tenant, different worker_ids),
//! - disabled-config path returns fresh every call,
//! - turn-end `record` advances the recorded session,
//! - 4-turn replay sees one recorded id (`ttl >= turn_interval`),
//! - `forget` drops the stored entry.
use crate::prompt_cache_affinity::PromptCacheAffinity;
use argo_types::{SessionId, TenantId};
use std::time::Duration;

#[test]
fn within_ttl_reuses_session() {
    let affinity = PromptCacheAffinity::enabled(Duration::from_secs(300));
    let tenant = TenantId::new();
    let first = affinity.acquire_or_mint(tenant, "worker-a");
    let second = affinity.acquire_or_mint(tenant, "worker-a");
    assert_eq!(first, second, "same pair within TTL must reuse id");
    assert_eq!(affinity.len(), 1, "single entry after two same-pair calls");
}

#[test]
fn zero_ttl_mints_fresh_every_call() {
    let affinity = PromptCacheAffinity::enabled(Duration::from_secs(0));
    let tenant = TenantId::new();
    let first = affinity.acquire_or_mint(tenant, "worker-a");
    // Spin briefly so `Instant::now()` strictly advances past the
    // recorded instant; the saturating_duration_since check uses
    // `<=` so the comparison must exceed zero by at least 1ns.
    std::thread::sleep(Duration::from_millis(2));
    let second = affinity.acquire_or_mint(tenant, "worker-a");
    assert_ne!(first, second, "zero-TTL must always mint a fresh id");
}

#[test]
fn per_tenant_isolation_same_worker_id() {
    let affinity = PromptCacheAffinity::enabled(Duration::from_secs(300));
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let id_a = affinity.acquire_or_mint(tenant_a, "worker-x");
    let id_b = affinity.acquire_or_mint(tenant_b, "worker-x");
    assert_ne!(id_a, id_b, "tenant_a must NOT see tenant_b's session id");

    // Cross-call after both rows recorded — each pair still sees
    // its own session.
    let id_a_again = affinity.acquire_or_mint(tenant_a, "worker-x");
    let id_b_again = affinity.acquire_or_mint(tenant_b, "worker-x");
    assert_eq!(id_a, id_a_again);
    assert_eq!(id_b, id_b_again);
    assert_eq!(affinity.len(), 2);
}

#[test]
fn per_worker_isolation_same_tenant() {
    let affinity = PromptCacheAffinity::enabled(Duration::from_secs(300));
    let tenant = TenantId::new();
    let id_x = affinity.acquire_or_mint(tenant, "worker-x");
    let id_y = affinity.acquire_or_mint(tenant, "worker-y");
    assert_ne!(id_x, id_y);
    assert_eq!(id_x, affinity.acquire_or_mint(tenant, "worker-x"));
    assert_eq!(id_y, affinity.acquire_or_mint(tenant, "worker-y"));
}

#[test]
fn disabled_config_mints_fresh_every_call() {
    let affinity = PromptCacheAffinity::disabled();
    assert!(!affinity.is_enabled());
    let tenant = TenantId::new();
    let first = affinity.acquire_or_mint(tenant, "worker-a");
    let second = affinity.acquire_or_mint(tenant, "worker-a");
    assert_ne!(first, second, "disabled affinity must mint fresh each call");
    assert_eq!(affinity.len(), 0, "disabled registry must not store rows");
}

#[test]
fn disabled_record_is_no_op() {
    let affinity = PromptCacheAffinity::disabled();
    let tenant = TenantId::new();
    affinity.record(tenant, "worker-a", SessionId::new());
    assert!(affinity.is_empty(), "disabled record must not store row");
}

#[test]
fn record_overrides_acquired_session() {
    let affinity = PromptCacheAffinity::enabled(Duration::from_secs(300));
    let tenant = TenantId::new();
    let acquired = affinity.acquire_or_mint(tenant, "worker-a");
    let real = SessionId::new();
    assert_ne!(acquired, real);
    affinity.record(tenant, "worker-a", real);
    let next = affinity.acquire_or_mint(tenant, "worker-a");
    assert_eq!(next, real, "subsequent acquire must return recorded id");
}

#[test]
fn forget_drops_recorded_entry() {
    let affinity = PromptCacheAffinity::enabled(Duration::from_secs(300));
    let tenant = TenantId::new();
    let first = affinity.acquire_or_mint(tenant, "worker-a");
    affinity.forget(tenant, "worker-a");
    assert!(affinity.is_empty());
    // Subsequent acquire mints a brand-new id.
    let second = affinity.acquire_or_mint(tenant, "worker-a");
    assert_ne!(first, second);
}

#[test]
fn four_turn_replay_sees_one_session_when_ttl_large() {
    // The brief's required integration check: a four-turn replay
    // for the same `(tenant, worker)` pair within a generous TTL
    // must see the same recorded session id across all four
    // turns. This is the property that makes the Anthropic cache
    // read counter climb.
    let affinity = PromptCacheAffinity::enabled(Duration::from_secs(600));
    let tenant = TenantId::new();
    let worker = "worker-replay";
    let mut seen = Vec::with_capacity(4);
    for _ in 0..4 {
        let id = affinity.acquire_or_mint(tenant, worker);
        affinity.record(tenant, worker, id);
        seen.push(id);
    }
    assert_eq!(
        seen.iter().collect::<std::collections::HashSet<_>>().len(),
        1,
        "all four turns must observe a single recorded session id"
    );
    assert_eq!(affinity.len(), 1);
}

#[test]
fn default_is_enabled_with_270s_ttl() {
    let affinity = PromptCacheAffinity::default();
    assert!(affinity.is_enabled());
    assert_eq!(affinity.ttl(), Duration::from_secs(270));
}

#[test]
fn cross_tenant_collision_isolated_at_identical_wallclock() {
    // Per the brief: per-tenant isolation must hold even at
    // identical wall-clocks. We approximate "identical" by
    // touching both tenants back-to-back; the underlying
    // `Instant::now()` resolution is much finer than a real
    // tenant-switch interval, but the keys themselves provide
    // the isolation guarantee — clock parity is irrelevant.
    let affinity = PromptCacheAffinity::enabled(Duration::from_secs(300));
    let t_a = TenantId::new();
    let t_b = TenantId::new();
    let pairs: Vec<_> = (0..8)
        .map(|i| {
            let tenant = if i % 2 == 0 { t_a } else { t_b };
            (tenant, affinity.acquire_or_mint(tenant, "shared-worker"))
        })
        .collect();
    let session_a = pairs.iter().find(|(t, _)| *t == t_a).map(|(_, s)| *s).unwrap();
    let session_b = pairs.iter().find(|(t, _)| *t == t_b).map(|(_, s)| *s).unwrap();
    assert_ne!(session_a, session_b);
    for (t, s) in &pairs {
        if *t == t_a {
            assert_eq!(*s, session_a);
        } else {
            assert_eq!(*s, session_b);
        }
    }
}
