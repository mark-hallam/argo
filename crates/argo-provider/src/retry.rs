//! Shared non-streaming retry/backoff helpers for provider transports.
//!
//! The retry loops in the Anthropic, OpenAI-compatible, and OpenAI
//! Responses transports keep their provider-specific HTTP request and
//! error-classification code, but their sleep calculation is delegated
//! to `argo-mcp-connector-shared` so provider calls and MCP connector
//! calls use the same exponential backoff primitive.

use argo_mcp_connector_shared::retry::{backoff_delay_ms, RetryPolicy};
use std::time::Duration;

const PROVIDER_RETRY_MULTIPLIER: f64 = 2.0;
const PROVIDER_RETRY_JITTER_MS: u64 = 100;

/// Return the provider retry policy corresponding to manifest transport
/// retry settings.
///
/// Provider manifests express `max_retries` as retries after the first
/// attempt. The shared MCP connector policy expresses `max_attempts` as
/// total attempts, so this adapter adds the initial attempt back in.
#[must_use]
pub fn provider_retry_policy(max_retries: u32, initial_ms: u64, max_ms: u64) -> RetryPolicy {
    RetryPolicy {
        max_attempts: max_retries.saturating_add(1).max(1),
        initial_delay_ms: initial_ms.max(1),
        multiplier: PROVIDER_RETRY_MULTIPLIER,
        max_delay_ms: max_ms.max(initial_ms).max(1),
        jitter_ms: PROVIDER_RETRY_JITTER_MS,
    }
}

/// Return whether retry attempt `attempt` would exceed `policy`.
///
/// Provider loops index the first retry as `attempt = 0`, while the
/// shared policy counts the initial call inside `max_attempts`.
#[must_use]
pub fn provider_retries_exhausted(policy: &RetryPolicy, attempt: u32) -> bool {
    attempt.saturating_add(1) >= policy.max_attempts.max(1)
}

/// Compute the delay before provider retry `attempt`.
///
/// Provider retry loops index the first retry as `attempt = 0`; the
/// shared MCP helper indexes it as `attempt = 1`, so this performs the
/// conversion and samples cheap jitter for thundering-herd smoothing.
#[must_use]
pub fn provider_backoff_delay(policy: &RetryPolicy, attempt: u32) -> Duration {
    provider_backoff_delay_with_jitter(policy, attempt, sample_jitter_value())
}

pub(crate) fn provider_backoff_delay_with_jitter(
    policy: &RetryPolicy,
    attempt: u32,
    jitter_value: u64,
) -> Duration {
    Duration::from_millis(backoff_delay_ms(
        policy,
        attempt.saturating_add(1),
        jitter_value,
    ))
}

fn sample_jitter_value() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::from(d.subsec_nanos()))
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;
