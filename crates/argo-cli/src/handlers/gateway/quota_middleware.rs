//! Per-action quota gate + tier-aware usage metering for the gateway.
//!
//! This module owns the single metered-action seam. Every metered
//! action (`POST /sessions`, forge runs, channel messages) flows through
//! [`check_quota_or_reject`], which:
//!   1. resolves the cwd-derived workspace tenant,
//!   2. clears the operator-configurable [`TenantQuotaStore`] caps
//!      (the override layer), then
//!   3. clears the **tier baseline** via [`check_tier_quota_or_reject`],
//!      which bumps the period-to-date usage counter on success and
//!      rejects with a structured upgrade error at the cap.
//!
//! "One seam, no parallel counter": the tier usage counter
//! (`tenant_usage`) is written in exactly one place —
//! `tenant_query::increment_tenant_usage`, called from
//! [`check_tier_quota_or_reject`] — so the `/billing` meter and
//! `GET /api/v1/tenant/usage` read a true number under every pricing
//! model. The counter is metered on Pro too (not just at the Free cap)
//! because it feeds whichever pricing model the operator picks.
//!
//! [`check_tier_quota_or_reject`] reads the tenant's pricing tier
//! (`workspace.tier`), resolves it to a [`RuntimeTierLimits`] via the
//! const operational table, and short-circuits dispatch with a
//! `429 Too Many Requests` JSON payload when the period-to-date counter
//! has reached the tier cap. On success the counter is bumped by one and
//! the request proceeds.
//!
//! # Quota-exceeded payload shape
//!
//! ```json
//! {
//!   "error": "quota_exceeded",
//!   "tier": "free",
//!   "limit": 100,
//!   "used": 100,
//!   "upgrade_to": "pro",
//!   "upgrade_url": "/billing",
//!   "retry_after_ts": 1735689600
//! }
//! ```
//!
//! `retry_after_ts` is the Unix-epoch-seconds boundary of the next
//! calendar month (UTC), matching Stripe invoice periods. `upgrade_to`
//! names the next tier up and `upgrade_url` the in-product page that
//! lifts the cap. Each rejection also emits a best-effort
//! `quota.exceeded` audit row so the over-quota event is forensically
//! visible.
//!
//! # Tier cache
//!
//! Tier reads hit SQLite, but tier doesn't change per request. A
//! best-effort 60-second LRU keeps the hot path off the database.
//! Cache miss falls back to a live `tenant_query::tenant_tier` read.

use argo_audit::{AuditKind, AuditSink};
use argo_identity::tier::Tier;
use argo_identity::tier_limits::RuntimeTierLimits;
use argo_store::tenant_quota::TenantQuotaStore;
use argo_store::tenant_quota_kinds::{QuotaActionKind, QuotaError};
use argo_store::tenant_query;
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use dashmap::DashMap;
use serde_json::json;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

const TIER_CACHE_TTL: Duration = Duration::from_secs(60);

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

/// The single metered-action seam: clear the operator-configurable
/// [`TenantQuotaStore`] override caps, then the tier baseline, metering
/// one action against the tenant's monthly usage. Returns
/// `Some((status, body))` to short-circuit dispatch (HTTP 429) when
/// either gate trips; `None` when the action is allowed (and the tier
/// usage counter has been bumped by one).
///
/// Tenant is resolved from the REQUEST via the canonical
/// [`super::resolve_tenant::resolve_tenant_for_request`] (WS1,
/// `pricing-quota-seam-resolve-request-tenant`, from the #2871 review):
/// an authenticated caller meters their OWN tenant's row; a pre-auth /
/// anonymous caller falls through that resolver to the cwd/demo tenant,
/// so the anonymous lane shares ONE bounded bucket and can never consume
/// a real tenant's allowance (nor a real tenant another's). The previous
/// cwd-only resolution lumped EVERY caller into the gateway workspace's
/// bucket — cross-tenant cap exhaustion + per-tenant under-reporting.
/// On a resolve failure the call fails open (returns `None`) — the gate
/// is not authoritative when the store is unreachable, and the
/// underlying failure is still observable via the usage read path.
pub(super) async fn check_quota_or_reject(
    req: &str,
    store: &Arc<SessionStore>,
    kind: QuotaActionKind,
) -> Option<(&'static str, String)> {
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .ok()?;
    let tenant_id = tenant.to_string();
    // 1. Operator-configurable override caps (independent of tier).
    let quotas = TenantQuotaStore::new(store.pool());
    match quotas
        .check_or_reject(&tenant_id, kind, 1, Utc::now())
        .await
    {
        Ok(()) => {}
        Err(QuotaError::Exceeded {
            kind,
            current,
            limit,
        }) => {
            return Some((
                "429 Too Many Requests",
                format!(
                    "{{\"error\":\"quota_exceeded\",\"kind\":\"{kind}\",\"current\":{current},\"limit\":{limit}}}"
                ),
            ));
        }
        // Fail-open on internal error — the gate is not authoritative
        // when the store is unreachable.
        Err(QuotaError::Internal(_)) => return None,
    }
    // 2. Tier baseline + usage metering (the one true counter).
    check_tier_quota_or_reject(store, tenant).await
}

/// Apply the tier-baseline monthly action quota to `tenant`. On
/// success increments the counter and returns `None`; on quota
/// exhaustion emits a `quota.exceeded` audit row and returns
/// `Some((status_line, body))` for the caller to frame as the gateway
/// response.
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
        let upgrade_to = next_tier_slug(tier);
        let body = format!(
            "{{\"error\":\"quota_exceeded\",\"tier\":\"{tier}\",\"limit\":{cap},\"used\":{used},\"upgrade_to\":\"{upgrade_to}\",\"upgrade_url\":\"/billing\",\"retry_after_ts\":{retry_after_ts}}}",
            tier = tier.as_str(),
        );
        emit_quota_exceeded_audit(store, tenant, tier, cap, used, upgrade_to).await;
        return Some(("429 Too Many Requests", body));
    }
    let _ = tenant_query::increment_tenant_usage(store, tenant, 1, now).await;
    None
}

/// The tier a quota-capped tenant upgrades INTO to lift the cap. Free →
/// Pro, Pro → Enterprise. Enterprise is unlimited and never reaches this
/// path, but maps to itself for totality.
fn next_tier_slug(tier: Tier) -> &'static str {
    match tier {
        Tier::Free => "pro",
        Tier::Pro => "enterprise",
        Tier::Enterprise => "enterprise",
    }
}

/// Best-effort `quota.exceeded` audit row. Truth-bounded forensic
/// breadcrumb of an over-quota rejection; a write failure never blocks
/// the response (the 429 is the authoritative caller-facing signal).
async fn emit_quota_exceeded_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    tier: Tier,
    limit: u64,
    used: u64,
    upgrade_to: &str,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    let payload = json!({
        "action_type": "quota.exceeded",
        "verb": "hit the monthly action quota",
        "endpoint": "quota.tier",
        "tier": tier.as_str(),
        "limit": limit,
        "used": used,
        "upgrade_to": upgrade_to,
        "upgrade_url": "/billing",
    });
    let _ = sink.write(None, AuditKind::HttpRequest, payload, None).await;
}

/// Calendar-month boundary (UTC) AFTER `now`, returned as Unix seconds.
/// December rolls over to January of the next year. Saturates to
/// `now.timestamp()` if year arithmetic overflows — which would only
/// happen near `i32::MAX` years from now and is safe to ignore.
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
