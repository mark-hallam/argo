use super::{
    provider_backoff_delay, provider_backoff_delay_with_jitter, provider_retries_exhausted,
    provider_retry_policy,
};
use argo_mcp_connector_shared::retry::{backoff_delay_ms, RetryPolicy};

#[test]
fn provider_policy_counts_initial_attempt_like_shared_policy() {
    let policy = provider_retry_policy(3, 100, 800);
    assert_eq!(policy.max_attempts, 4);
    assert_eq!(policy.initial_delay_ms, 100);
    assert_eq!(policy.max_delay_ms, 800);
    assert_eq!(policy.multiplier, 2.0);
}

#[test]
fn provider_delay_matches_mcp_connector_primitive() {
    let policy = RetryPolicy {
        max_attempts: 5,
        initial_delay_ms: 100,
        multiplier: 2.0,
        max_delay_ms: 800,
        jitter_ms: 0,
    };
    for provider_attempt in 0..6 {
        let expected = backoff_delay_ms(&policy, provider_attempt + 1, 0);
        let got = provider_backoff_delay_with_jitter(&policy, provider_attempt, 0);
        assert_eq!(got.as_millis() as u64, expected);
    }
}

#[test]
fn provider_delay_clamps_to_configured_ceiling_before_jitter() {
    let policy = RetryPolicy {
        max_attempts: 10,
        initial_delay_ms: 250,
        multiplier: 2.0,
        max_delay_ms: 1_000,
        jitter_ms: 0,
    };
    assert_eq!(
        provider_backoff_delay_with_jitter(&policy, 0, 0).as_millis(),
        250
    );
    assert_eq!(
        provider_backoff_delay_with_jitter(&policy, 1, 0).as_millis(),
        500
    );
    assert_eq!(
        provider_backoff_delay_with_jitter(&policy, 2, 0).as_millis(),
        1_000
    );
    assert_eq!(
        provider_backoff_delay_with_jitter(&policy, 30, 0).as_millis(),
        1_000
    );
}

#[test]
fn provider_delay_applies_shared_jitter_window() {
    let policy = RetryPolicy {
        max_attempts: 3,
        initial_delay_ms: 100,
        multiplier: 2.0,
        max_delay_ms: 800,
        jitter_ms: 50,
    };
    assert_eq!(
        provider_backoff_delay_with_jitter(&policy, 0, 0).as_millis(),
        100
    );
    assert_eq!(
        provider_backoff_delay_with_jitter(&policy, 0, 49).as_millis(),
        149
    );
    assert_eq!(
        provider_backoff_delay_with_jitter(&policy, 0, 50).as_millis(),
        100
    );
}

#[test]
fn provider_retry_exhaustion_matches_provider_retry_count() {
    let policy = provider_retry_policy(3, 100, 800);
    assert!(!provider_retries_exhausted(&policy, 0));
    assert!(!provider_retries_exhausted(&policy, 1));
    assert!(!provider_retries_exhausted(&policy, 2));
    assert!(provider_retries_exhausted(&policy, 3));
}

#[test]
fn provider_public_delay_never_returns_zero() {
    let zero_policy = provider_retry_policy(1, 0, 0);
    let normal_policy = provider_retry_policy(1, 100, 800);
    assert!(provider_backoff_delay(&zero_policy, 0).as_millis() >= 1);
    assert!(provider_backoff_delay(&normal_policy, u32::MAX).as_millis() >= 1);
}
