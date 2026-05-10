//! Pure-function unit tests for the canary picker. DB-backed
//! `CanaryStore` + runner-integration tests live in
//! `crates/argo-agent-runner/tests/canary_routing.rs`.

use crate::canary::{is_canary, new_config, pick_version, seed_for_session_skill, CanaryConfig};
use argo_types::SessionId;

fn cfg(percent: u8) -> CanaryConfig {
    CanaryConfig {
        skill_id: "demo".into(),
        baseline_version: "v1".into(),
        canary_version: "v2".into(),
        traffic_percent: percent,
        started_at: 0,
        expires_at: None,
    }
}

#[test]
fn pick_zero_percent_always_baseline() {
    let c = cfg(0);
    for seed in 0..1_000_u64 {
        assert_eq!(pick_version(&c, seed), "v1");
    }
}

#[test]
fn pick_hundred_percent_always_canary() {
    let c = cfg(100);
    for seed in 0..1_000_u64 {
        assert_eq!(pick_version(&c, seed), "v2");
    }
    let c200 = cfg(200);
    for seed in 0..50_u64 {
        assert_eq!(pick_version(&c200, seed), "v2");
    }
}

#[test]
fn pick_is_deterministic_for_fixed_seed() {
    let c = cfg(42);
    let first = pick_version(&c, 12_345);
    for _ in 0..50 {
        assert_eq!(pick_version(&c, 12_345), first);
    }
}

#[test]
fn ten_percent_split_lands_within_two_percent() {
    let c = cfg(10);
    let mut canary_hits = 0_u64;
    let total = 10_000_u64;
    for seed in 0..total {
        if pick_version(&c, seed) == "v2" {
            canary_hits += 1;
        }
    }
    let rate = canary_hits as f64 / total as f64;
    assert!(
        (rate - 0.10).abs() < 0.02,
        "expected ~10% canary hits, got {rate}"
    );
}

#[test]
fn fifty_percent_split_is_close_to_half() {
    let c = cfg(50);
    let mut canary_hits = 0_u64;
    let total = 10_000_u64;
    for seed in 0..total {
        if pick_version(&c, seed) == "v2" {
            canary_hits += 1;
        }
    }
    let rate = canary_hits as f64 / total as f64;
    assert!(
        (rate - 0.50).abs() < 0.02,
        "expected ~50% canary hits, got {rate}"
    );
}

#[test]
fn is_canary_handles_extremes() {
    assert!(!is_canary(123, 0));
    assert!(is_canary(123, 100));
    assert!(is_canary(123, 200));
}

#[test]
fn new_config_rejects_identical_versions() {
    let err = new_config("demo", "v1", "v1", 10).unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::Schema(_)));
}

#[test]
fn new_config_rejects_out_of_range_percent() {
    let err = new_config("demo", "v1", "v2", 200).unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::Schema(_)));
}

#[test]
fn seed_for_session_skill_is_stable_and_specific() {
    let sid = SessionId(uuid::Uuid::nil());
    let a = seed_for_session_skill(sid, "demo");
    let b = seed_for_session_skill(sid, "demo");
    assert_eq!(a, b);
    let c = seed_for_session_skill(sid, "other");
    assert_ne!(a, c);
    let sid2 = SessionId(uuid::Uuid::from_bytes([1_u8; 16]));
    let d = seed_for_session_skill(sid2, "demo");
    assert_ne!(a, d);
}
