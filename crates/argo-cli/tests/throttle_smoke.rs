//! `mission-throttle-registry-wire-dispatch` smoke test.
//!
//! Exercises the runtime helpers the CLI dispatcher delegates to.
//! Each CLI invocation is its own process with a fresh registry,
//! so end-to-end CLI rate-limiting isn't observable — these tests
//! pin the contract the wire relies on instead.

use argo_runtime::action_throttle::{
    acquire_or_wait, build_audit_payload, should_emit_audit, ThrottleConfig, ThrottlePolicy,
    ThrottleRegistry, AUDIT_KIND_LABEL, DEFAULT_AUDIT_THRESHOLD,
};
use chrono::Utc;
use std::collections::HashMap;
use std::time::{Duration, Instant};

fn cfg_with(action: &str, tokens_per_sec: f32, burst: u32) -> ThrottleConfig {
    let mut policies = HashMap::new();
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

#[tokio::test(flavor = "current_thread")]
async fn ten_rapid_sends_gate_at_configured_rate() {
    // 1000 tok/s, burst 1 → first acquire proceeds; subsequent 9
    // each wait the MIN_WAIT_DELAY floor (10ms) for the bucket to
    // refill. Total wall-clock ≈ 90ms — fast enough to stay a
    // smoke test, slow enough to assert the gate held.
    let registry = ThrottleRegistry::new(cfg_with("send_email", 1000.0, 1));
    let started = Instant::now();
    let mut total = Duration::ZERO;
    for i in 0..10 {
        let waited = acquire_or_wait(
            &registry,
            "tenant-a",
            "send_email",
            Utc::now,
            tokio::time::sleep,
        )
        .await;
        if i == 0 {
            assert_eq!(
                waited,
                Duration::ZERO,
                "burst slot 0 should not wait, got {waited:?}"
            );
        } else {
            assert!(
                waited > Duration::ZERO,
                "burst-exhausted slot {i} should wait > 0, got {waited:?}"
            );
        }
        total = total.saturating_add(waited);
    }
    // Sanity: the 10 dispatches together consumed a meaningful
    // amount of throttled wait — 9 refills @ 100 tok/s ~= 90ms.
    assert!(
        total >= Duration::from_millis(50),
        "10 acquires should accrue ≥50ms of waits, got {total:?}"
    );
    // Real wall-clock stays modest — 10 refills @ MIN_WAIT_DELAY
    // ceiling sit comfortably under 1s on a healthy CI runner.
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "took too long, {:?}",
        started.elapsed()
    );
}

#[tokio::test(flavor = "current_thread")]
async fn different_action_types_interleave_freely() {
    // send_email is drained, post_listing has fresh burst — both
    // proceed without one starving the other (per-action-type
    // bucket isolation under the same tenant).
    let mut policies = HashMap::new();
    policies.insert(
        "send_email".to_string(),
        ThrottlePolicy::new(1.0, 1).expect("valid"),
    );
    policies.insert(
        "post_listing".to_string(),
        ThrottlePolicy::new(1.0, 1).expect("valid"),
    );
    let registry = ThrottleRegistry::new(ThrottleConfig {
        enabled: true,
        policies,
        audit_threshold: DEFAULT_AUDIT_THRESHOLD,
    });
    // Drain send_email's burst; post_listing must still proceed
    // immediately on the next call.
    let first = acquire_or_wait(
            &registry,
            "tenant-a",
            "send_email",
            Utc::now,
            tokio::time::sleep,
        )
        .await;
    assert_eq!(first, Duration::ZERO);
    let second = acquire_or_wait(
        &registry,
        "tenant-a",
        "post_listing",
        Utc::now,
        tokio::time::sleep,
    )
    .await;
    assert_eq!(
        second,
        Duration::ZERO,
        "post_listing must not be gated by an empty send_email bucket"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn disabled_registry_never_waits() {
    // The pre-brief contract: when [runtime.action_throttle] is
    // off, dispatch is zero-overhead. acquire_or_wait must short-
    // circuit because try_acquire returns Proceed without taking
    // the bucket lock.
    let registry = ThrottleRegistry::disabled();
    for _ in 0..50 {
        let waited =
            acquire_or_wait(
            &registry,
            "tenant-a",
            "send_email",
            Utc::now,
            tokio::time::sleep,
        )
        .await;
        assert_eq!(waited, Duration::ZERO);
    }
    assert!(
        registry.snapshot(Utc::now()).is_empty(),
        "disabled registry must not allocate buckets"
    );
}

#[test]
fn audit_signal_fires_past_threshold() {
    // The dispatcher uses should_emit_audit + build_audit_payload
    // to decide whether to emit a `runtime.action_throttled` row.
    // Under the default 10s threshold, a 12s wait crosses; a 5s
    // wait does not.
    let cfg = ThrottleConfig::default();
    assert!(!should_emit_audit(&cfg, Duration::from_secs(5)));
    assert!(should_emit_audit(&cfg, Duration::from_secs(12)));
    let payload =
        build_audit_payload("tenant-a", "send_email", Duration::from_secs(12));
    assert_eq!(payload["tenant_id"], "tenant-a");
    assert_eq!(payload["action_type"], "send_email");
    assert!(
        (payload["queued_for_secs"].as_f64().unwrap() - 12.0).abs() < 1e-6,
        "got {}",
        payload["queued_for_secs"]
    );
    // The dispatcher's tracing kind label is the same string the
    // doctor + the future first-class AuditKind variant key on.
    assert_eq!(AUDIT_KIND_LABEL, "runtime.action_throttled");
}
