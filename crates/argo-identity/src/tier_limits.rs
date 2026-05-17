//! Runtime enforcement knobs derived from a [`crate::tier::Tier`].
//!
//! `tier.rs` carries the **product-facing** tier surface (display
//! name, Stripe price, worker cap, feature flags) compiled from
//! `docs/skycot/pricing/v1-tiers.toml`. `tier_limits.rs` carries the
//! **operational** knobs the gateway uses to fail-closed when a
//! tenant crosses a tier line: monthly action quota, MCP connector
//! allowlist, request-per-second sustained + burst rates.
//!
//! Splitting them keeps the customer-facing TOML schema narrow (the
//! values a sales/billing operator might tune) and reserves the
//! security-shaped operational defaults for a code-owned const table
//! that ships with the binary. A future brief that wires per-tenant
//! overrides for these knobs (e.g. a custom enterprise contract) is
//! free to layer over the const baseline.
//!
//! ```
//! use argo_identity::tier::Tier;
//! use argo_identity::tier_limits::{ConnectorPolicy, RuntimeTierLimits};
//!
//! let limits = RuntimeTierLimits::for_tier(Tier::Free);
//! assert_eq!(limits.monthly_actions, 100);
//! assert!(matches!(limits.allowed_connectors, ConnectorPolicy::Allowlist(_)));
//! assert!(limits.allows_connector("gmail"));
//! assert!(!limits.allows_connector("hubspot"));
//! ```

use crate::tier::Tier;

/// Sentinel for "unlimited" in [`RuntimeTierLimits::monthly_actions`].
/// `0` is the legacy convention used by `v1-tiers.toml`; this constant
/// gives callers a self-documenting name to compare against.
pub const UNLIMITED_ACTIONS: u32 = 0;

/// MCP-connector authorisation policy for a tier. Action paths that
/// hit a connector should consult [`RuntimeTierLimits::allows_connector`]
/// rather than matching on this enum directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectorPolicy {
    /// Only the listed connector slugs are permitted. Match is exact
    /// on the canonical lowercase slug (`"gmail"`, `"slack"`).
    Allowlist(&'static [&'static str]),
    /// All in-core tools and MCP connectors are permitted.
    AllowAll,
}

/// Per-tier runtime gates. Fields are public so the caller (gateway
/// middleware) can read them without going through accessors; the
/// values themselves come from a const per-tier table and are not
/// runtime-mutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTierLimits {
    /// Calendar-month action cap. `UNLIMITED_ACTIONS` (`0`) means no
    /// cap — fail-open is correct only for enterprise.
    pub monthly_actions: u32,
    /// MCP-connector allowlist policy.
    pub allowed_connectors: ConnectorPolicy,
    /// Sustained requests-per-second budget (token-bucket refill rate).
    pub rps_sustained: u32,
    /// Burst tolerance: the bucket capacity. Greater than or equal to
    /// `rps_sustained`.
    pub rps_burst: u32,
}

impl RuntimeTierLimits {
    /// Resolve the canonical runtime gates for `tier`. Pure: no IO,
    /// safe to call inside a hot-path middleware before the LRU lookup.
    #[must_use]
    pub fn for_tier(tier: Tier) -> Self {
        match tier {
            Tier::Free => Self {
                monthly_actions: 100,
                allowed_connectors: ConnectorPolicy::Allowlist(FREE_CONNECTOR_ALLOWLIST),
                rps_sustained: 1,
                rps_burst: 5,
            },
            Tier::Pro => Self {
                monthly_actions: 5_000,
                allowed_connectors: ConnectorPolicy::AllowAll,
                rps_sustained: 10,
                rps_burst: 50,
            },
            Tier::Enterprise => Self {
                monthly_actions: UNLIMITED_ACTIONS,
                allowed_connectors: ConnectorPolicy::AllowAll,
                rps_sustained: 100,
                rps_burst: 500,
            },
        }
    }

    /// Whether `connector_slug` is permitted under this tier. The
    /// caller is expected to normalise the slug to lowercase and to
    /// strip the `argo-mcp-connector-` crate prefix before calling.
    #[must_use]
    pub fn allows_connector(&self, connector_slug: &str) -> bool {
        match &self.allowed_connectors {
            ConnectorPolicy::AllowAll => true,
            ConnectorPolicy::Allowlist(list) => list.contains(&connector_slug),
        }
    }

    /// Whether the monthly action quota is unlimited (`monthly_actions
    /// == UNLIMITED_ACTIONS`).
    #[must_use]
    pub fn is_unlimited(&self) -> bool {
        self.monthly_actions == UNLIMITED_ACTIONS
    }
}

/// Free-tier connectors. The two lowest-risk integrations — both
/// read-mostly OAuth and widely deployed; broader connectors require
/// at least Pro.
const FREE_CONNECTOR_ALLOWLIST: &[&str] = &["gmail", "slack"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_tier_caps_at_100() {
        let limits = RuntimeTierLimits::for_tier(Tier::Free);
        assert_eq!(limits.monthly_actions, 100);
        assert_eq!(limits.rps_sustained, 1);
        assert_eq!(limits.rps_burst, 5);
        assert!(!limits.is_unlimited());
    }

    #[test]
    fn pro_tier_caps_match_brief() {
        let limits = RuntimeTierLimits::for_tier(Tier::Pro);
        assert_eq!(limits.rps_sustained, 10);
        assert_eq!(limits.rps_burst, 50);
        assert!(matches!(limits.allowed_connectors, ConnectorPolicy::AllowAll));
    }

    #[test]
    fn enterprise_unlimited() {
        let limits = RuntimeTierLimits::for_tier(Tier::Enterprise);
        assert!(limits.is_unlimited());
        assert_eq!(limits.rps_sustained, 100);
        assert_eq!(limits.rps_burst, 500);
    }

    #[test]
    fn free_allows_only_gmail_slack() {
        let limits = RuntimeTierLimits::for_tier(Tier::Free);
        assert!(limits.allows_connector("gmail"));
        assert!(limits.allows_connector("slack"));
        assert!(!limits.allows_connector("hubspot"));
        assert!(!limits.allows_connector("salesforce"));
        assert!(!limits.allows_connector(""));
    }

    #[test]
    fn pro_allows_every_connector() {
        let limits = RuntimeTierLimits::for_tier(Tier::Pro);
        for slug in ["gmail", "slack", "hubspot", "salesforce", "notion"] {
            assert!(limits.allows_connector(slug), "pro should allow {slug}");
        }
    }
}
