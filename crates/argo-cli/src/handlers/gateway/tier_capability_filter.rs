//! Per-tier MCP-connector allowlist.
//!
//! Note: `check_tier_connector_or_reject` and its helpers are
//! `pub(super)` and tested in isolation, but the dispatch wiring is
//! intentionally out of this brief's may-touch list — see the
//! sibling note in `quota_middleware.rs`. `#[allow(dead_code)]`
//! markers are temporary.
//!
//! Free tier customers can only dispatch through the two lowest-risk
//! integrations (`gmail`, `slack`). Pro and Enterprise are
//! unrestricted. This filter is layered AFTER the role/capability
//! check (`Capability::ToolDispatch`) — the question it answers is
//! *"does the tier this tenant pays for unlock this connector?"*, not
//! *"does this principal have authority to call this connector?"*
//!
//! Returns `403 Forbidden` with a `tier_upsell_url` pointing the
//! caller at the upgrade page when the tier does not include the
//! requested connector.

use argo_identity::tier::Tier;
use argo_identity::tier_limits::RuntimeTierLimits;
use argo_store::{tenant_query, SessionStore};
use argo_types::TenantId;
use std::str::FromStr;
use std::sync::Arc;

#[allow(dead_code, reason = "wired by follow-up brief that touches routes_dispatch_post.rs")]
const TIER_UPSELL_URL: &str = "https://skycot.com/upgrade";

/// Canonical lowercase slug for an MCP connector — strips the
/// `argo-mcp-connector-` crate-name prefix when present. Public so
/// callers that already have a slug (e.g. parsed from a tool name
/// like `gmail.send_message`) can normalise consistently.
#[allow(dead_code, reason = "wired by follow-up brief that touches routes_dispatch_post.rs")]
#[must_use]
pub(super) fn normalise_connector_slug(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    lower
        .strip_prefix("argo-mcp-connector-")
        .unwrap_or(&lower)
        .to_string()
}

/// Pure decision: is `connector_slug` permitted under `tier`? Wraps
/// [`RuntimeTierLimits::allows_connector`] after slug normalisation
/// so callers don't need to remember to lowercase.
#[allow(dead_code, reason = "wired by follow-up brief that touches routes_dispatch_post.rs")]
#[must_use]
pub(super) fn tier_allows_connector(tier: Tier, connector_slug: &str) -> bool {
    let slug = normalise_connector_slug(connector_slug);
    RuntimeTierLimits::for_tier(tier).allows_connector(&slug)
}

/// Gateway-shaped wrapper around [`tier_allows_connector`]. Resolves
/// the tenant's tier from `workspace.tier` and returns `Some((status,
/// body))` on denial, `None` on pass. Fail-closed: unknown tier
/// strings collapse to [`Tier::Free`] (the most restrictive policy)
/// rather than silently widening to enterprise.
#[allow(dead_code, reason = "wired by follow-up brief that touches routes_dispatch_post.rs")]
pub(super) async fn check_tier_connector_or_reject(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    connector_slug: &str,
) -> Option<(&'static str, String)> {
    let raw = tenant_query::tenant_tier(store, tenant)
        .await
        .unwrap_or_else(|_| "free".to_string());
    let tier = Tier::from_str(&raw).unwrap_or(Tier::Free);
    if tier_allows_connector(tier, connector_slug) {
        return None;
    }
    let normalised = normalise_connector_slug(connector_slug);
    let body = format!(
        "{{\"error\":\"tier_forbidden\",\"tier\":\"{tier}\",\"connector\":\"{slug}\",\"tier_upsell_url\":\"{url}\"}}",
        tier = tier.as_str(),
        slug = normalised,
        url = TIER_UPSELL_URL,
    );
    Some(("403 Forbidden", body))
}

#[cfg(test)]
#[path = "tier_capability_filter_tests.rs"]
mod tests;
