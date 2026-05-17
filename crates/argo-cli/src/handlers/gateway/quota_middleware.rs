//! Tier-aware monthly-action quota check for the gateway.
//!
//! Note: `check_tier_quota_or_reject` is `pub(super)` and tested in
//! isolation, but the dispatch wiring (calling it at the top of
//! `routes_dispatch_post::dispatch_post`) is intentionally out of
//! this brief's may-touch list — `routes_dispatch_post.rs` is
//! reserved for a follow-up "wire-up" brief. The `#[allow(dead_code)]`
//! markers below acknowledge the temporary state; remove them in the
//! follow-up that wires the call.
//!
//! Reads the tenant's pricing tier (`workspace.tier`), resolves it to
//! a [`RuntimeTierLimits`] via the const operational table, and
//! short-circuits dispatch with a `429 Too Many Requests` JSON payload
//! when the period-to-date counter has reached the tier cap. On
//! success the counter is bumped by one and the request proceeds.
//!
//! Pattern mirrors the existing `check_quota_or_reject` helper in
//! `routes_dispatch_post.rs` (which guards the `TenantQuotaStore`
//! operator-configurable caps). The two paths are independent: this
//! one enforces the **tier baseline**, the other enforces
//! **operator-overrides on top of the baseline**. Both must clear
//! before the request runs.
//!
//! # 429 payload shape
//!
//! ```json
//! {
//!   "error": "quota_exceeded",
//!   "tier": "free",
//!   "limit": 100,
//!   "used": 100,
//!   "retry_after_ts": 1735689600
//! }
//! ```
//!
//! `retry_after_ts` is the Unix-epoch-seconds boundary of the next
//! calendar month (UTC), matching Stripe invoice periods.
//!
//! # Tier cache
//!
//! Tier reads hit SQLite, but tier doesn't change per request. A
//! best-effort 60-second LRU keeps the hot path off the database.
//! Cache miss falls back to a live `tenant_query::tenant_tier` read.

use argo_identity::tier::Tier;
use argo_identity::tier_limits::RuntimeTierLimits;
use argo_store::tenant_query;
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use dashmap::DashMap;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

#[allow(dead_code, reason = "wired by follow-up brief that touches routes_dispatch_post.rs")]
const TIER_CACHE_TTL: Duration = Duration::from_secs(60);

#[allow(dead_code, reason = "wired by follow-up brief that touches routes_dispatch_post.rs")]
#[derive(Debug, Clone)]
struct CachedTier {
    tier: Tier,
    inserted_at: Instant,
}

static TIER_CACHE: OnceLock<Arc<DashMap<String, CachedTier>>> = OnceLock::new();

fn cache() -> Arc<DashMap<String, CachedTier>> {
    TIER_CACHE
        .get_or_init(|| Arc::new(DashMap::new()))
        .clone()
}

/// One-shot cache initialisation. Idempotent — safe to call multiple
/// times from boot.
pub(super) fn init_tier_cache() {
    let _ = cache();
}

/// Resolve the cached or freshly-read [`Tier`] for `tenant`. Returns
/// [`Tier::Free`] (fail-closed) when the tier value on disk is
/// unrecognised — a misconfigured workspace shouldn't quietly inherit
/// the unlimited enterprise budget.
#[allow(dead_code, reason = "wired by follow-up brief that touches routes_dispatch_post.rs")]
async fn resolve_tier(store: &SessionStore, tenant: TenantId) -> Tier {
    let key = tenant.to_string();
    let now = Instant::now();
    if let Some(entry) = cache().get(&key) {
        if now.saturating_duration_since(entry.inserted_at) < TIER_CACHE_TTL {
            return entry.tier;
        }
    }
    let raw = tenant_query::tenant_tier(store, tenant)
        .await
        .unwrap_or_else(|_| "free".to_string());
    let tier = Tier::from_str(&raw).unwrap_or(Tier::Free);
    cache().insert(
        key,
        CachedTier {
            tier,
            inserted_at: now,
        },
    );
    tier
}

/// Apply the tier-baseline monthly action quota to `tenant`. On
/// success increments the counter and returns `None`; on quota
/// exhaustion returns `Some((status_line, body))` for the caller to
/// frame as the gateway response.
#[allow(dead_code, reason = "wired by follow-up brief that touches routes_dispatch_post.rs")]
pub(super) async fn check_tier_quota_or_reject(
    store: &Arc<SessionStore>,
    tenant: TenantId,
) -> Option<(&'static str, String)> {
    let now = Utc::now();
    let tier = resolve_tier(store, tenant).await;
    let limits = RuntimeTierLimits::for_tier(tier);
    if limits.is_unlimited() {
        let _ = tenant_query::increment_tenant_usage(store, tenant, 1, now).await;
        return None;
    }
    let (used, _period_start) = tenant_query::tenant_usage_for_period(store, tenant, now)
        .await
        .unwrap_or((0, now.timestamp()));
    let cap = u64::from(limits.monthly_actions);
    if used >= cap {
        let retry_after_ts = next_month_boundary_ts(now);
        let body = format!(
            "{{\"error\":\"quota_exceeded\",\"tier\":\"{tier}\",\"limit\":{cap},\"used\":{used},\"retry_after_ts\":{retry_after_ts}}}",
            tier = tier.as_str(),
        );
        return Some(("429 Too Many Requests", body));
    }
    let _ = tenant_query::increment_tenant_usage(store, tenant, 1, now).await;
    None
}

/// Calendar-month boundary (UTC) AFTER `now`, returned as Unix seconds.
/// December rolls over to January of the next year. Saturates to
/// `now.timestamp()` if year arithmetic overflows — which would only
/// happen near `i32::MAX` years from now and is safe to ignore.
#[allow(dead_code, reason = "wired by follow-up brief that touches routes_dispatch_post.rs")]
fn next_month_boundary_ts(now: DateTime<Utc>) -> i64 {
    let (year, month) = if now.month() == 12 {
        (now.year().saturating_add(1), 1)
    } else {
        (now.year(), now.month() + 1)
    };
    Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .map(|dt| dt.timestamp())
        .unwrap_or_else(|| now.timestamp())
}

#[cfg(test)]
#[path = "quota_middleware_tests.rs"]
mod tests;

#[cfg(test)]
pub(super) fn clear_cache_for_test() {
    cache().clear();
}
