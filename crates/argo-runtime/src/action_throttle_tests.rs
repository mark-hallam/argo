//! Unit tests for [`crate::action_throttle`]. Included via `#[path]`
//! from `action_throttle.rs` so the symbols stay private to the
//! parent module while the test file keeps the parent under the
//! 500-LOC ceiling.
//!
//! The brief calls for 7 tests; each one pins one acceptance-
//! criteria row from `streams/mission-per-action-type-throttling.md`:
//!
//! 1. `refills_at_configured_rate` — refill behaviour
//! 2. `allows_configured_burst` — burst capacity
//! 3. `wait_on_empty_returns_typed_delay` — non-busy-wait gate
//! 4. `tenants_have_isolated_buckets` — per-tenant isolation
//! 5. `audit_threshold_classifies_long_waits` — 10s audit row
//! 6. `disabled_registry_always_proceeds` — backward-compat bypass
//! 7. `multi_action_types_isolated` — bucket key correctness

use super::*;
use chrono::TimeZone;
use std::time::Duration;

fn t(offset_ms: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc
        .with_ymd_and_hms(2026, 5, 13, 0, 0, 0)
        .unwrap()
        + chrono::Duration::milliseconds(offset_ms)
}

fn cfg_with(action: &str, tokens_per_sec: f32, burst: u32) -> ThrottleConfig {
    let mut policies = std::collections::HashMap::new();
    policies.insert(
        action.to_string(),
        ThrottlePolicy::new(tokens_per_sec, burst).expect("valid policy"),
    );
    ThrottleConfig {
        enabled: true,
        policies,
        audit_threshold: DEFAULT_AUDIT_THRESHOLD,
    }
}

#[test]
fn refills_at_configured_rate() {
    // 2 tok/s, burst 1 → drain the single token, wait 1s, then a
    // second dispatch must succeed (1 token refilled in 0.5s; we
    // step a full second so the bucket is at full burst again).
    let registry = ThrottleRegistry::new(cfg_with("send_email", 2.0, 1));
    assert_eq!(
        registry.try_acquire("t1", "send_email", t(0)),
        ThrottleDecision::Proceed
    );
    // Immediately after, no token available.
    match registry.try_acquire("t1", "send_email", t(1)) {
        ThrottleDecision::Wait { .. } => {}
        other => panic!("expected Wait, got {other:?}"),
    }
    // After 1s, bucket has refilled — second dispatch proceeds.
    assert_eq!(
        registry.try_acquire("t1", "send_email", t(1_000)),
        ThrottleDecision::Proceed
    );
}

#[test]
fn allows_configured_burst() {
    // burst = 3 → three back-to-back dispatches at t=0 proceed.
    let registry = ThrottleRegistry::new(cfg_with("post_listing", 1.0, 3));
    for i in 0..3 {
        assert_eq!(
            registry.try_acquire("t1", "post_listing", t(0)),
            ThrottleDecision::Proceed,
            "burst dispatch {i} should proceed"
        );
    }
    // The fourth waits — burst is exhausted, refill is 1 tok/s so
    // the next slot is ~1s out.
    match registry.try_acquire("t1", "post_listing", t(0)) {
        ThrottleDecision::Wait { delay } => {
            // 1 token short, refill 1/s → ~1000ms delay.
            assert!(delay >= Duration::from_millis(900));
            assert!(delay <= Duration::from_millis(1100));
        }
        other => panic!("expected Wait, got {other:?}"),
    }
}

#[test]
fn wait_on_empty_returns_typed_delay() {
    // The Wait decision must carry a concrete Duration so callers
    // can yield to the scheduler (no busy-wait). The delay is
    // clamped at MIN_WAIT_DELAY on the low end.
    let registry = ThrottleRegistry::new(cfg_with("create_invoice", 1000.0, 1));
    let _ = registry.try_acquire("t1", "create_invoice", t(0));
    let decision = registry.try_acquire("t1", "create_invoice", t(0));
    match decision {
        ThrottleDecision::Wait { delay } => {
            assert!(
                delay >= MIN_WAIT_DELAY,
                "delay {delay:?} below floor {MIN_WAIT_DELAY:?}"
            );
        }
        other => panic!("expected Wait, got {other:?}"),
    }
}

#[test]
fn tenants_have_isolated_buckets() {
    // One slow tenant must not starve another. With burst=1, the
    // first tenant exhausts its bucket; the second tenant still
    // gets through on the same instant.
    let registry = ThrottleRegistry::new(cfg_with("send_email", 1.0, 1));
    assert_eq!(
        registry.try_acquire("tenant-a", "send_email", t(0)),
        ThrottleDecision::Proceed
    );
    // Tenant A is now empty.
    match registry.try_acquire("tenant-a", "send_email", t(0)) {
        ThrottleDecision::Wait { .. } => {}
        other => panic!("tenant-a should be empty, got {other:?}"),
    }
    // Tenant B at the same instant must still proceed — proves
    // the (tenant_id, action_type) keying.
    assert_eq!(
        registry.try_acquire("tenant-b", "send_email", t(0)),
        ThrottleDecision::Proceed
    );
}

#[test]
fn audit_threshold_classifies_long_waits() {
    // build_audit_payload + should_emit_audit are the operator-
    // visible seam; both must agree on the 10s default threshold
    // and the payload shape documented in the brief.
    let config = ThrottleConfig::default();
    assert!(!should_emit_audit(&config, Duration::from_secs(5)));
    assert!(should_emit_audit(&config, Duration::from_secs(10)));
    assert!(should_emit_audit(&config, Duration::from_secs(42)));
    let payload = build_audit_payload("tenant-q", "send_email", Duration::from_millis(12_500));
    assert_eq!(payload["tenant_id"], "tenant-q");
    assert_eq!(payload["action_type"], "send_email");
    assert!(
        (payload["queued_for_secs"].as_f64().unwrap() - 12.5).abs() < 1e-6,
        "got {}",
        payload["queued_for_secs"]
    );
    // Tunable threshold — operator can lift the bar.
    let lifted = ThrottleConfig {
        audit_threshold: Duration::from_secs(60),
        ..ThrottleConfig::default()
    };
    assert!(!should_emit_audit(&lifted, Duration::from_secs(30)));
    assert!(should_emit_audit(&lifted, Duration::from_secs(60)));
}

#[test]
fn disabled_registry_always_proceeds() {
    // Zero-overhead bypass is the backward-compatibility guarantee.
    // A disabled registry returns Proceed for every key, regardless
    // of how many calls have been made, and never populates the
    // bucket map.
    let registry = ThrottleRegistry::disabled();
    for i in 0..50 {
        assert_eq!(
            registry.try_acquire("tenant", "send_email", t(i)),
            ThrottleDecision::Proceed
        );
    }
    let snap = registry.snapshot(t(0));
    assert!(snap.is_empty(), "disabled registry must not allocate buckets");
}

#[test]
fn multi_action_types_isolated() {
    // Two different action_types under the same tenant must have
    // independent buckets — draining `send_email` must not affect
    // `post_listing`. Also exercises the snapshot ordering used by
    // `argo doctor throttles`.
    let mut policies = std::collections::HashMap::new();
    policies.insert(
        "send_email".to_string(),
        ThrottlePolicy::new(1.0, 1).expect("valid"),
    );
    policies.insert(
        "post_listing".to_string(),
        ThrottlePolicy::new(1.0, 2).expect("valid"),
    );
    let registry = ThrottleRegistry::new(ThrottleConfig {
        enabled: true,
        policies,
        audit_threshold: DEFAULT_AUDIT_THRESHOLD,
    });
    // Drain send_email completely.
    assert_eq!(
        registry.try_acquire("t1", "send_email", t(0)),
        ThrottleDecision::Proceed
    );
    match registry.try_acquire("t1", "send_email", t(0)) {
        ThrottleDecision::Wait { .. } => {}
        other => panic!("send_email should be empty, got {other:?}"),
    }
    // post_listing still has its independent burst capacity.
    assert_eq!(
        registry.try_acquire("t1", "post_listing", t(0)),
        ThrottleDecision::Proceed
    );
    assert_eq!(
        registry.try_acquire("t1", "post_listing", t(0)),
        ThrottleDecision::Proceed
    );
    // Snapshot surfaces both buckets, sorted by (tenant, action).
    let snap = registry.snapshot(t(0));
    assert_eq!(snap.len(), 2);
    assert_eq!(snap[0].action_type, "post_listing");
    assert_eq!(snap[1].action_type, "send_email");
    assert_eq!(snap[0].tenant_id, "t1");
}

#[test]
fn invalid_policies_reject_at_construction() {
    // A `tokens_per_sec <= 0` or `burst == 0` mis-config returns
    // `None` so the loader can refuse to install a foot-gun policy
    // instead of dispatching forever-blocked buckets.
    assert!(ThrottlePolicy::new(0.0, 5).is_none());
    assert!(ThrottlePolicy::new(-1.0, 5).is_none());
    assert!(ThrottlePolicy::new(f32::NAN, 5).is_none());
    assert!(ThrottlePolicy::new(1.0, 0).is_none());
    assert!(ThrottlePolicy::new(1.0, 1).is_some());
}

#[test]
fn unknown_action_type_proceeds_when_no_policy() {
    // An action_type absent from the policy map is not throttled
    // even when the registry is enabled — operators opt-in per
    // action_type, never globally.
    let registry = ThrottleRegistry::new(cfg_with("send_email", 1.0, 1));
    for _ in 0..10 {
        assert_eq!(
            registry.try_acquire("t1", "ungoverned_action", t(0)),
            ThrottleDecision::Proceed
        );
    }
}
