//! Per-tenant token-bucket rate limiter for the gateway
//! (`mvp-rate-limit`).
//!
//! Layered at the top of [`super::routes::dispatch_route`], just
//! before the per-route gate (`claude-rate-limit-per-route`). Each
//! resolved `tenant_id` gets a token bucket with capacity = the
//! per-minute budget (default 60), so a cold-start tenant can burst
//! its full minute. Refill is `cap / 60.0` tokens per second. An
//! empty bucket → `429 Too Many Requests` whose JSON body carries
//! `"retry_after": <secs>` (also surfaced in `retry_after_secs` for
//! callers that want both shapes).
//!
//! This is **independent + additive** to:
//!
//! - per-IP limiter [`super::rate_limit_ip`] (#1051) — rejects floods
//!   from a single source IP at the listener.
//! - per-route limiter [`super::rate_limit`] (#1077) — rejects
//!   per-`(route, tenant)` bursts.
//!
//! The per-tenant layer rejects abuse of **shared infrastructure**
//! across all routes: a single tenant burning their per-minute budget
//! across many endpoints can DoS shared queues even when no individual
//! per-route bucket trips.
//!
//! # Defaults
//!
//! - 60 requests / minute / tenant.
//!
//! # Env knobs
//!
//! - `ARGO_RATE_LIMIT_TENANT_PER_MIN=<n>` — overrides the per-minute
//!   budget. `0` disables the limiter. Invalid values fall back to
//!   the default rather than failing gateway start.
//!
//! # Bypass
//!
//! - `ARGO_GATEWAY_AUTH_BYPASS=1` short-circuits the limiter. Tests
//!   carry no resolved principal, and per-tenant limiting collapses
//!   onto a single shared `"anonymous"` bucket without bypass.
//!
//! # Sensible deviation from brief
//!
//! The brief asked for an `HTTP Retry-After` header on the 429. The
//! gateway frames responses in `boot.rs` (outside this brief's
//! may-touch list) with a fixed header set, and `dispatch_route`
//! returns only `(status_line, body)` — same constraint that produced
//! the per-route limiter's deviation note. The retry interval is
//! therefore surfaced inside the JSON body (`retry_after`,
//! `retry_after_secs`) only. Adding a true header requires a separate
//! brief that touches `boot.rs`.
//!
//! # Examples
//!
//! ```ignore
//! let limiter = TenantRateLimiter::new(60, true);
//! assert_eq!(limiter.check("acme"), CheckResult::Allow);
//! ```

use dashmap::DashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

#[cfg(test)]
#[path = "rate_limit_tenant_tests.rs"]
mod tests;

/// Default per-tenant request budget per minute.
pub(super) const DEFAULT_PER_MIN: u32 = 60;
/// Soft cap on the bucket map; eviction kicks in past this size.
pub(super) const MAX_ENTRIES: usize = 100_000;
/// Buckets idle this long are eligible for eviction.
pub(super) const EVICT_AGE_SECS: u64 = 5 * 60;

/// One tenant's token bucket.
#[derive(Debug)]
pub(super) struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

impl Bucket {
    fn new(capacity: u32, now: Instant) -> Self {
        Self {
            tokens: f64::from(capacity),
            last_refill: now,
        }
    }

    /// Refill lazily based on elapsed wall-time, then try to consume
    /// one token. Returns `true` when consumed.
    fn take(&mut self, now: Instant, capacity: u32, refill_per_sec: f64) -> bool {
        let elapsed = now.saturating_duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * refill_per_sec).min(f64::from(capacity));
        self.last_refill = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Outcome of a rate-limit check.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum CheckResult {
    /// Token consumed; request may proceed.
    Allow,
    /// Bucket empty; reject with this `Retry-After` value (seconds).
    Deny { retry_after_secs: u32 },
}

/// In-memory per-tenant token-bucket store. Cheap to clone (`Arc`-backed).
pub struct TenantRateLimiter {
    buckets: Arc<DashMap<String, Bucket>>,
    cap: u32,
    refill: f64,
    enabled: bool,
}

impl TenantRateLimiter {
    /// Construct with explicit settings. Used by tests.
    pub fn new(per_min: u32, enabled: bool) -> Self {
        let cap = per_min.max(1);
        Self {
            buckets: Arc::new(DashMap::new()),
            cap,
            refill: f64::from(cap) / 60.0,
            enabled,
        }
    }

    /// Construct from `ARGO_RATE_LIMIT_TENANT_PER_MIN`. `0` disables;
    /// invalid → default.
    pub fn from_env() -> Self {
        match std::env::var("ARGO_RATE_LIMIT_TENANT_PER_MIN")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
        {
            Some(0) => Self::new(DEFAULT_PER_MIN, false),
            Some(n) => Self::new(n, true),
            None => Self::new(DEFAULT_PER_MIN, true),
        }
    }

    /// Try to consume one token for `tenant` at the current wall-clock.
    pub(super) fn check(&self, tenant: &str) -> CheckResult {
        self.check_with_now(tenant, Instant::now())
    }

    /// Test-friendly variant that takes an explicit `Instant`.
    pub(super) fn check_with_now(&self, tenant: &str, now: Instant) -> CheckResult {
        if !self.enabled {
            return CheckResult::Allow;
        }
        let allowed = {
            let mut entry = self
                .buckets
                .entry(tenant.to_string())
                .or_insert_with(|| Bucket::new(self.cap, now));
            entry.take(now, self.cap, self.refill)
        };
        if !allowed && self.buckets.len() > MAX_ENTRIES {
            self.evict_old(now);
        }
        if allowed {
            CheckResult::Allow
        } else {
            // Time until one token refills, rounded up. Always at
            // least 1.
            let secs = (1.0 / self.refill).ceil();
            let retry = if secs >= f64::from(u32::MAX) {
                u32::MAX
            } else {
                (secs as u32).max(1)
            };
            CheckResult::Deny {
                retry_after_secs: retry,
            }
        }
    }

    /// Evict buckets whose `last_refill` is older than
    /// [`EVICT_AGE_SECS`]. Called when the map exceeds [`MAX_ENTRIES`].
    pub(super) fn evict_old(&self, now: Instant) {
        let cutoff = Duration::from_secs(EVICT_AGE_SECS);
        self.buckets
            .retain(|_, bucket| now.saturating_duration_since(bucket.last_refill) < cutoff);
    }

    /// Number of tracked tenants. Used by tests.
    #[cfg(test)]
    pub(super) fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
}

static LIMITER: OnceLock<Arc<TenantRateLimiter>> = OnceLock::new();

/// Process-wide limiter handle, lazily constructed from env on first
/// access. Idempotent across threads — `OnceLock` guarantees a single
/// builder run.
pub fn global() -> Arc<TenantRateLimiter> {
    LIMITER
        .get_or_init(|| Arc::new(TenantRateLimiter::from_env()))
        .clone()
}

/// Apply the per-tenant rate limit to `req`. Returns `Some((status,
/// body))` when the bucket is empty and the caller should
/// short-circuit, `None` when the request may proceed.
///
/// Honours `ARGO_GATEWAY_AUTH_BYPASS=1` (skip the limiter — used by
/// integration-test mode where there is no resolved principal). The
/// 429 body keeps the `error: rate_limited` shape distinguishable
/// from `error: quota_exceeded` (the `claude-tenant-quota-enforcement-stricter`
/// brief).
pub(super) fn check_request(req: &str) -> Option<(&'static str, String)> {
    if std::env::var("ARGO_GATEWAY_AUTH_BYPASS").ok().as_deref() == Some("1") {
        return None;
    }
    let limiter = global();
    let tenant = super::rate_limit::extract_tenant(req);
    match limiter.check(&tenant) {
        CheckResult::Allow => None,
        CheckResult::Deny { retry_after_secs } => {
            tracing::warn!(
                target: "gateway::rate_limit_tenant",
                tenant = %tenant,
                retry_after_secs,
                "per-tenant rate limit triggered"
            );
            let body = format!(
                "{{\"error\":\"rate_limited\",\"scope\":\"tenant\",\
                 \"retry_after\":{retry_after_secs},\
                 \"retry_after_secs\":{retry_after_secs}}}"
            );
            Some(("429 Too Many Requests", body))
        }
    }
}
