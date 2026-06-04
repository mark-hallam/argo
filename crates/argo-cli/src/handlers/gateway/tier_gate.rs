//! Canonical pricing-tier capability gate for the gateway
//! (`sprint-ws4-3-enforce-tier-capability-gates`).
//!
//! Wires the tier feature flags + quotas from
//! `docs/skycot/pricing/v1-tiers.toml` (loaded via
//! [`argo_identity::tier::TierConfig`]) into the runtime so the Free
//! tier is actually bounded — closing the WS5-4 finding that
//! `provider_real_keys` had **no runtime reader** (only the toml +
//! tests referenced it). After this brief, `rg provider_real_keys
//! crates/` shows [`tier_allows`] being called at the provider-key
//! save path.
//!
//! ONE helper family lives here so callers never scatter ad-hoc tier
//! string comparisons (`tier == "free"` etc.):
//!
//! * [`tier_allows`] — does the tenant's tier unlock a named feature
//!   flag (`provider_real_keys`, `channel_voice`, …)? This is the
//!   single capability predicate the gateway consults.
//! * [`check_worker_cap_or_reject`] — is the tenant under its tier's
//!   `max_workers` cap?
//! * [`tier_forbidden_response`] — the canonical `403` upsell body.
//!
//! Fail-closed: an unknown / unreadable tier collapses to
//! [`Tier::Free`] (most restrictive), never silently widening to
//! enterprise — same posture as `tier_capability_filter`.

use argo_identity::tier::{Tier, TierConfig};
use argo_store::tenant_query::{tenant_tier, tenant_worker_count};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::str::FromStr;
use std::sync::Arc;

/// Upgrade page the `403` bodies point the caller at. Mirrors
/// `tier_capability_filter::TIER_UPSELL_URL`.
const TIER_UPSELL_URL: &str = "https://skycot.com/upgrade";

/// Resolve the tenant's pricing tier, failing **closed** to
/// [`Tier::Free`] on any read or parse error.
pub(super) async fn tenant_tier_or_free(store: &Arc<SessionStore>, tenant: TenantId) -> Tier {
    let raw = tenant_tier(store, tenant)
        .await
        .unwrap_or_else(|_| "free".to_string());
    Tier::from_str(&raw).unwrap_or(Tier::Free)
}

/// `true` when the tenant's tier unlocks `feature` (a bare flag name
/// from `v1-tiers.toml`, e.g. `"provider_real_keys"` or
/// `"channel_slack"`). The single canonical capability check — callers
/// consult this instead of comparing tier strings inline. Fail-closed
/// via [`tenant_tier_or_free`].
pub(super) async fn tier_allows(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    feature: &str,
) -> bool {
    let tier = tenant_tier_or_free(store, tenant).await;
    TierConfig::default_config()
        .limits(tier)
        .is_some_and(|limits| limits.has_feature(feature))
}

/// Canonical `403 Forbidden` upsell body emitted when a tier lacks a
/// requested feature. Shape mirrors `tier_capability_filter`'s
/// connector denial so the SPA renders one upsell affordance.
pub(super) fn tier_forbidden_response(tier: Tier, feature: &str) -> (&'static str, String) {
    let body = format!(
        "{{\"error\":\"tier_forbidden\",\"tier\":\"{tier}\",\"feature\":\"{feature}\",\"tier_upsell_url\":\"{url}\"}}",
        tier = tier.as_str(),
        url = TIER_UPSELL_URL,
    );
    ("403 Forbidden", body)
}

/// Hire-time `max_workers` gate. Returns `Some(403)` when the tenant
/// already sits at/over its tier's worker cap, `None` to let the hire
/// proceed. `max_workers == 0` in the tier config encodes "unlimited"
/// (enterprise) and always passes.
///
/// Fail-**open** on a count read error: the worker cap is a commercial
/// quota, not a security boundary, so a transient store fault must
/// never block a paying tenant's hire (the tenant→tier read is already
/// fail-closed to Free, which is the stricter side).
pub(super) async fn check_worker_cap_or_reject(
    store: &Arc<SessionStore>,
    tenant: TenantId,
) -> Option<(&'static str, String)> {
    let tier = tenant_tier_or_free(store, tenant).await;
    let cap = TierConfig::default_config()
        .limits(tier)
        .map_or(0, |limits| limits.max_workers);
    if cap == 0 {
        return None;
    }
    let used = tenant_worker_count(store, tenant).await.unwrap_or(0);
    if used >= u64::from(cap) {
        return Some(worker_cap_response(tier, used, cap));
    }
    None
}

/// `403 Forbidden` body for a refused hire that would exceed the tier's
/// `max_workers`. Distinct `error` code from [`tier_forbidden_response`]
/// so the SPA can render "you've hit your worker limit" vs "this
/// feature needs an upgrade".
fn worker_cap_response(tier: Tier, used: u64, cap: u32) -> (&'static str, String) {
    let body = format!(
        "{{\"error\":\"tier_worker_cap\",\"tier\":\"{tier}\",\"used\":{used},\"max_workers\":{cap},\"tier_upsell_url\":\"{url}\"}}",
        tier = tier.as_str(),
        url = TIER_UPSELL_URL,
    );
    ("403 Forbidden", body)
}

#[cfg(test)]
#[path = "tier_gate_tests.rs"]
mod tests;
