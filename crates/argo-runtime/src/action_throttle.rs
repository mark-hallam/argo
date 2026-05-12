//! Per-action-type throttling
//! (`mission-per-action-type-throttling` / MISSION_BACKLOG §8 #254).
//!
//! Some action types (`send_email`, `post_listing`, `create_invoice`)
//! have natural per-minute rate limits at the downstream API. Without
//! a worker-side gate, dispatch fires as fast as the loop spins, hits
//! 429s, and burns retry budget on avoidable load.
//!
//! This module is the dispatch-layer gate. A [`ThrottleRegistry`]
//! holds one [`TokenBucket`] per `(tenant_id, action_type)` tuple,
//! refills tokens at the operator-configured rate, and returns a
//! typed [`ThrottleDecision`] from [`ThrottleRegistry::try_acquire`]:
//!
//! - [`ThrottleDecision::Proceed`] — token consumed, dispatch now.
//! - [`ThrottleDecision::Wait { delay }`] — bucket empty, the
//!   caller yields to the scheduler for `delay` and retries.
//!
//! ## Correctness invariant
//!
//! Per-tenant isolation. The bucket key is the
//! `(tenant_id, action_type)` tuple, not just `action_type`, so one
//! slow tenant cannot starve another tenant's dispatch.
//!
//! ## Operator-visible signal
//!
//! When the cumulative wait for one dispatch exceeds
//! [`ThrottleConfig::audit_threshold`] (default 10s), the caller
//! emits a `runtime.action_throttled` audit row via
//! [`build_audit_payload`]. Sustained throttling at a given
//! `(tenant_id, action_type)` means the operator's
//! `tokens_per_sec` is too tight for the workload.
//!
//! ## Disabled by default
//!
//! [`ThrottleConfig::enabled`] defaults to `false` so the wire is
//! zero-overhead until an operator opts in via
//! `[runtime.action_throttle] enabled = true`. A disabled registry
//! returns [`ThrottleDecision::Proceed`] without touching the bucket
//! map.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

/// Stable `kind` label for audit rows emitted when a dispatch waits
/// longer than [`ThrottleConfig::audit_threshold`].
pub const AUDIT_KIND_LABEL: &str = "runtime.action_throttled";

/// Default cumulative-wait threshold past which a dispatch emits a
/// `runtime.action_throttled` audit row.
pub const DEFAULT_AUDIT_THRESHOLD: Duration = Duration::from_secs(10);

/// Minimum poll delay returned by [`ThrottleDecision::Wait`] when
/// the bucket is empty but the configured rate would refill very
/// quickly. Below 10ms the scheduler spin-cost dominates so we
/// clamp on the low end.
pub const MIN_WAIT_DELAY: Duration = Duration::from_millis(10);

/// Operator-tunable throttle policy for one action type. Mirrors
/// the TOML shape `[runtime.action_throttle.<action_type>]
/// tokens_per_sec = N; burst = M`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThrottlePolicy {
    /// Average sustained dispatch rate, in tokens per second. Must
    /// be `> 0`; a `0` value rejects all dispatches and is treated
    /// as a misconfiguration by [`ThrottleRegistry::try_acquire`].
    pub tokens_per_sec: f32,
    /// Maximum burst depth in tokens. The bucket refills at
    /// `tokens_per_sec` and clamps at `burst`; setting `burst =
    /// tokens_per_sec` removes burst capacity entirely.
    pub burst: u32,
}

impl ThrottlePolicy {
    /// Build a policy after validating the rate is strictly
    /// positive and the burst is non-zero. Returns `None` on
    /// either misconfiguration so callers can degrade gracefully
    /// instead of panicking at registry-load time.
    #[must_use]
    pub fn new(tokens_per_sec: f32, burst: u32) -> Option<Self> {
        if tokens_per_sec <= 0.0 || !tokens_per_sec.is_finite() || burst == 0 {
            return None;
        }
        Some(Self {
            tokens_per_sec,
            burst,
        })
    }
}

/// Top-level throttle config. Mirrors `[runtime.action_throttle]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThrottleConfig {
    /// Master enable. Off by default for backward compatibility;
    /// when `false` the registry returns
    /// [`ThrottleDecision::Proceed`] immediately and never touches
    /// the bucket map.
    pub enabled: bool,
    /// Per-action-type policies. Action types absent from this map
    /// dispatch without throttling even when `enabled = true`.
    pub policies: HashMap<String, ThrottlePolicy>,
    /// Cumulative-wait threshold past which a dispatch emits a
    /// `runtime.action_throttled` audit row. Operator-tunable so
    /// monitoring tenants with naturally chatty bursts can lift
    /// the bar.
    pub audit_threshold: Duration,
}

impl Default for ThrottleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            policies: HashMap::new(),
            audit_threshold: DEFAULT_AUDIT_THRESHOLD,
        }
    }
}

/// Verdict from [`ThrottleRegistry::try_acquire`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ThrottleDecision {
    /// Token consumed; dispatch immediately.
    Proceed,
    /// Bucket empty. Caller yields to the scheduler for `delay`
    /// then re-invokes [`ThrottleRegistry::try_acquire`]. The
    /// caller never busy-waits — the gate stays at the dispatch
    /// layer.
    Wait {
        /// Minimum delay before the next try (clamped at
        /// [`MIN_WAIT_DELAY`]).
        delay: Duration,
    },
}

/// Snapshot of one bucket's level. Returned by
/// [`ThrottleRegistry::snapshot`] for the `argo doctor throttles`
/// surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BucketSnapshot {
    /// Tenant id the bucket is scoped to.
    pub tenant_id: String,
    /// Action type the bucket is scoped to.
    pub action_type: String,
    /// Tokens currently available (refilled to wall-clock `now`).
    pub tokens_available: f32,
    /// Configured burst capacity.
    pub burst: u32,
    /// Configured refill rate.
    pub tokens_per_sec: f32,
    /// Last refill timestamp.
    pub last_refill: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f32,
    last_refill: DateTime<Utc>,
    policy: ThrottlePolicy,
}

impl TokenBucket {
    fn new(policy: ThrottlePolicy, now: DateTime<Utc>) -> Self {
        Self {
            tokens: f32::from(u16::try_from(policy.burst.min(u32::from(u16::MAX))).unwrap_or(u16::MAX)),
            last_refill: now,
            policy,
        }
    }

    fn refill(&mut self, now: DateTime<Utc>) {
        let elapsed_ms = (now - self.last_refill).num_milliseconds().max(0);
        if elapsed_ms == 0 {
            return;
        }
        let elapsed_secs = (elapsed_ms as f32) / 1000.0;
        let added = elapsed_secs * self.policy.tokens_per_sec;
        let ceiling = self.policy.burst as f32;
        self.tokens = (self.tokens + added).min(ceiling);
        self.last_refill = now;
    }

    fn try_consume(&mut self, now: DateTime<Utc>) -> ThrottleDecision {
        self.refill(now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            ThrottleDecision::Proceed
        } else {
            let deficit = 1.0 - self.tokens;
            let secs_needed = (deficit / self.policy.tokens_per_sec).max(0.0);
            let delay = Duration::from_millis((secs_needed * 1000.0).ceil() as u64).max(MIN_WAIT_DELAY);
            ThrottleDecision::Wait { delay }
        }
    }
}

/// Per-`(tenant_id, action_type)` token-bucket registry. Build one
/// per supervisor / dispatcher; cheap to clone the [`std::sync::Arc`]
/// wrapper around it.
#[derive(Debug)]
pub struct ThrottleRegistry {
    config: ThrottleConfig,
    buckets: Mutex<HashMap<(String, String), TokenBucket>>,
}

impl ThrottleRegistry {
    /// Build a registry against the operator-loaded config.
    #[must_use]
    pub fn new(config: ThrottleConfig) -> Self {
        Self {
            config,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Build a registry with [`ThrottleConfig::default`] (disabled,
    /// no policies). Useful for tests + the default supervisor wiring.
    #[must_use]
    pub fn disabled() -> Self {
        Self::new(ThrottleConfig::default())
    }

    /// Borrow the config so callers can introspect threshold /
    /// policy-map state without taking the bucket lock.
    #[must_use]
    pub fn config(&self) -> &ThrottleConfig {
        &self.config
    }

    /// Attempt to consume one token for `(tenant_id, action_type)`.
    ///
    /// Returns [`ThrottleDecision::Proceed`] when:
    /// - the registry is disabled, OR
    /// - no policy is configured for `action_type`, OR
    /// - a token was available and consumed.
    ///
    /// Returns [`ThrottleDecision::Wait`] with a suggested poll
    /// delay when the bucket is empty. The caller yields to the
    /// scheduler; the delay is the minimum time until the next
    /// refill would unblock the request.
    pub fn try_acquire(
        &self,
        tenant_id: &str,
        action_type: &str,
        now: DateTime<Utc>,
    ) -> ThrottleDecision {
        if !self.config.enabled {
            return ThrottleDecision::Proceed;
        }
        let Some(policy) = self.config.policies.get(action_type).copied() else {
            return ThrottleDecision::Proceed;
        };
        let key = (tenant_id.to_string(), action_type.to_string());
        let Ok(mut buckets) = self.buckets.lock() else {
            return ThrottleDecision::Proceed;
        };
        let bucket = buckets
            .entry(key)
            .or_insert_with(|| TokenBucket::new(policy, now));
        bucket.try_consume(now)
    }

    /// Snapshot every live bucket for `argo doctor throttles`. The
    /// output is sorted by `(tenant_id, action_type)` for stable
    /// diffing in JSON consumers.
    #[must_use]
    pub fn snapshot(&self, now: DateTime<Utc>) -> Vec<BucketSnapshot> {
        let Ok(mut buckets) = self.buckets.lock() else {
            return Vec::new();
        };
        let mut rows: Vec<BucketSnapshot> = buckets
            .iter_mut()
            .map(|((tenant, action), b)| {
                b.refill(now);
                BucketSnapshot {
                    tenant_id: tenant.clone(),
                    action_type: action.clone(),
                    tokens_available: b.tokens,
                    burst: b.policy.burst,
                    tokens_per_sec: b.policy.tokens_per_sec,
                    last_refill: b.last_refill,
                }
            })
            .collect();
        rows.sort_by(|a, b| {
            a.tenant_id
                .cmp(&b.tenant_id)
                .then_with(|| a.action_type.cmp(&b.action_type))
        });
        rows
    }
}

/// Build the JSON payload for one `runtime.action_throttled` audit
/// row. Callers pass the cumulative wait observed across one
/// dispatch's poll loop and emit when `queued_for >=
/// ThrottleConfig::audit_threshold`.
#[must_use]
pub fn build_audit_payload(
    tenant_id: &str,
    action_type: &str,
    queued_for: Duration,
) -> serde_json::Value {
    serde_json::json!({
        "action_type": action_type,
        "queued_for_secs": queued_for.as_secs_f64(),
        "tenant_id": tenant_id,
    })
}

/// Convenience: classify whether a cumulative-wait observation
/// crosses the operator-tuned audit threshold. Keeps the comparison
/// logic in one place so the dispatcher does not duplicate it.
#[must_use]
pub fn should_emit_audit(config: &ThrottleConfig, queued_for: Duration) -> bool {
    queued_for >= config.audit_threshold
}

#[cfg(test)]
#[path = "action_throttle_tests.rs"]
mod action_throttle_tests;
