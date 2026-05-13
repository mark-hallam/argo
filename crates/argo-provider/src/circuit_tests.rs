//! Unit tests for [`super::Breaker`].
//!
//! Time-sensitive transitions are tested via the `_at` variants that
//! take an explicit `Instant`, so tests don't sleep.

use super::{is_breaker_failure, Breaker, BreakerConfig, CircuitState};
use argo_types::ArgoError;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn cfg(threshold: u32, cooldown_secs: u64, max_secs: u64) -> BreakerConfig {
    BreakerConfig {
        failure_threshold: threshold,
        cooldown: Duration::from_secs(cooldown_secs),
        cooldown_max: Duration::from_secs(max_secs),
    }
}

#[test]
fn closed_admits_until_threshold_then_opens() {
    let breaker = Breaker::new(cfg(3, 30, 300));
    let now = Instant::now();
    for _ in 0..2 {
        let p = breaker.try_acquire_at("p", now).unwrap();
        p.record_failure_at(now);
    }
    assert_eq!(breaker.state_of("p"), CircuitState::Closed);
    let p = breaker.try_acquire_at("p", now).unwrap();
    p.record_failure_at(now);
    assert_eq!(breaker.state_of("p"), CircuitState::Open);
}

#[test]
fn success_resets_consecutive_failure_counter() {
    let breaker = Breaker::new(cfg(3, 30, 300));
    let now = Instant::now();
    for _ in 0..2 {
        breaker
            .try_acquire_at("p", now)
            .unwrap()
            .record_failure_at(now);
    }
    breaker.try_acquire_at("p", now).unwrap().record_success();
    breaker
        .try_acquire_at("p", now)
        .unwrap()
        .record_failure_at(now);
    breaker
        .try_acquire_at("p", now)
        .unwrap()
        .record_failure_at(now);
    assert_eq!(breaker.state_of("p"), CircuitState::Closed);
}

#[test]
fn open_rejects_fast_and_does_not_call_provider() {
    let breaker = Breaker::new(cfg(1, 30, 300));
    let t0 = Instant::now();
    breaker
        .try_acquire_at("p", t0)
        .unwrap()
        .record_failure_at(t0);
    assert_eq!(breaker.state_of("p"), CircuitState::Open);
    let err = breaker
        .try_acquire_at("p", t0 + Duration::from_secs(5))
        .unwrap_err();
    match err {
        ArgoError::Provider(msg) => assert!(msg.contains("circuit open")),
        other => panic!("expected Provider, got {other:?}"),
    }
}

#[test]
fn cooldown_elapses_to_half_open_with_single_probe() {
    let breaker = Breaker::new(cfg(1, 10, 300));
    let t0 = Instant::now();
    breaker
        .try_acquire_at("p", t0)
        .unwrap()
        .record_failure_at(t0);
    let after = t0 + Duration::from_secs(11);
    let probe = breaker.try_acquire_at("p", after).unwrap();
    assert!(probe.is_probe());
    assert_eq!(breaker.state_of("p"), CircuitState::HalfOpen);
    let concurrent = breaker.try_acquire_at("p", after).unwrap_err();
    assert!(matches!(concurrent, ArgoError::Provider(m) if m.contains("circuit open")));
    probe.record_success();
    assert_eq!(breaker.state_of("p"), CircuitState::Closed);
}

#[test]
fn half_open_failure_reopens_and_doubles_cooldown() {
    let breaker = Breaker::new(cfg(1, 10, 300));
    let t0 = Instant::now();
    breaker
        .try_acquire_at("p", t0)
        .unwrap()
        .record_failure_at(t0);
    let probe_t = t0 + Duration::from_secs(11);
    let probe = breaker.try_acquire_at("p", probe_t).unwrap();
    probe.record_failure_at(probe_t);
    assert_eq!(breaker.state_of("p"), CircuitState::Open);
    // Initial cooldown 10s; doubled to 20s after probe failure.
    let still_open = breaker
        .try_acquire_at("p", probe_t + Duration::from_secs(15))
        .unwrap_err();
    assert!(matches!(still_open, ArgoError::Provider(m) if m.contains("circuit open")));
    let admitted = breaker
        .try_acquire_at("p", probe_t + Duration::from_secs(21))
        .unwrap();
    assert!(admitted.is_probe());
}

#[test]
fn cooldown_cap_enforced() {
    let breaker = Breaker::new(cfg(1, 10, 25));
    let mut t = Instant::now();
    breaker.try_acquire_at("p", t).unwrap().record_failure_at(t);
    // Repeatedly fail probes; cooldown should saturate at 25s.
    for _ in 0..6 {
        t += Duration::from_secs(60);
        let probe = breaker.try_acquire_at("p", t).unwrap();
        probe.record_failure_at(t);
    }
    // After saturation, a probe is admitted exactly 25s after the last
    // failure, not later.
    let too_early = t + Duration::from_secs(24);
    assert!(breaker.try_acquire_at("p", too_early).is_err());
    let just_right = t + Duration::from_secs(25);
    assert!(breaker.try_acquire_at("p", just_right).is_ok());
}

#[test]
fn ignored_outcome_leaves_state_unchanged() {
    let breaker = Breaker::new(cfg(2, 30, 300));
    let now = Instant::now();
    breaker
        .try_acquire_at("p", now)
        .unwrap()
        .record_failure_at(now);
    breaker.try_acquire_at("p", now).unwrap().record_ignored();
    // Counter still 1 — one more failure should trip.
    breaker
        .try_acquire_at("p", now)
        .unwrap()
        .record_failure_at(now);
    assert_eq!(breaker.state_of("p"), CircuitState::Open);
}

#[test]
fn dropped_probe_releases_slot_and_reopens() {
    let breaker = Breaker::new(cfg(1, 10, 300));
    let t0 = Instant::now();
    breaker
        .try_acquire_at("p", t0)
        .unwrap()
        .record_failure_at(t0);
    let probe_t = t0 + Duration::from_secs(11);
    {
        let _probe = breaker.try_acquire_at("p", probe_t).unwrap();
        // dropped without recording — Drop fires record_failure
    }
    assert_eq!(breaker.state_of("p"), CircuitState::Open);
}

#[test]
fn separate_providers_track_independently() {
    let breaker = Breaker::new(cfg(2, 30, 300));
    let now = Instant::now();
    for _ in 0..2 {
        breaker
            .try_acquire_at("anthropic", now)
            .unwrap()
            .record_failure_at(now);
    }
    assert_eq!(breaker.state_of("anthropic"), CircuitState::Open);
    assert_eq!(breaker.state_of("openai"), CircuitState::Closed);
    breaker
        .try_acquire_at("openai", now)
        .unwrap()
        .record_success();
    assert_eq!(breaker.state_of("openai"), CircuitState::Closed);
}

#[test]
fn env_overrides_honored() {
    let _g = ENV_LOCK.lock().unwrap();
    let prev: Vec<(_, _)> = [
        "ARGO_CB_FAILURE_THRESHOLD",
        "ARGO_CB_COOLDOWN_SECS",
        "ARGO_CB_COOLDOWN_MAX_SECS",
    ]
    .iter()
    .map(|k| (*k, std::env::var(k).ok()))
    .collect();
    // SAFETY: serialised via ENV_LOCK; restored below.
    unsafe {
        std::env::set_var("ARGO_CB_FAILURE_THRESHOLD", "9");
        std::env::set_var("ARGO_CB_COOLDOWN_SECS", "7");
        std::env::set_var("ARGO_CB_COOLDOWN_MAX_SECS", "11");
    }
    let cfg = BreakerConfig::from_env();
    assert_eq!(cfg.failure_threshold, 9);
    assert_eq!(cfg.cooldown, Duration::from_secs(7));
    assert_eq!(cfg.cooldown_max, Duration::from_secs(11));
    // Invalid values fall back.
    unsafe {
        std::env::set_var("ARGO_CB_FAILURE_THRESHOLD", "0");
        std::env::set_var("ARGO_CB_COOLDOWN_SECS", "garbage");
        std::env::remove_var("ARGO_CB_COOLDOWN_MAX_SECS");
    }
    let defaulted = BreakerConfig::from_env();
    assert_eq!(
        defaulted.failure_threshold,
        super::DEFAULT_FAILURE_THRESHOLD
    );
    assert_eq!(
        defaulted.cooldown,
        Duration::from_secs(super::DEFAULT_COOLDOWN_SECS)
    );
    // Restore env.
    unsafe {
        for (k, v) in prev {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }
}

#[test]
fn classifier_treats_5xx_and_network_as_failures_only() {
    assert!(is_breaker_failure(&ArgoError::Provider(
        "anthropic HTTP 503: bad gateway".into()
    )));
    assert!(is_breaker_failure(&ArgoError::Provider(
        "openai HTTP 500: internal".into()
    )));
    assert!(is_breaker_failure(&ArgoError::Provider(
        "reqwest send: connection refused".into()
    )));
    assert!(is_breaker_failure(&ArgoError::Provider(
        "reqwest send (stream): timed out".into()
    )));
    assert!(!is_breaker_failure(&ArgoError::Provider(
        "anthropic HTTP 429: rate limited".into()
    )));
    assert!(!is_breaker_failure(&ArgoError::Provider(
        "openai HTTP 400: bad request".into()
    )));
    assert!(!is_breaker_failure(&ArgoError::Provider(
        "anthropic response parse: bad json".into()
    )));
    assert!(!is_breaker_failure(&ArgoError::Schema("oops".into())));
}
