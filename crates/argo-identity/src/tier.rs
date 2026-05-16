//! Pricing-tier definitions and feature gates for v1 commercial
//! launch (`claude-pricing-plans-config`).
//!
//! The canonical source of truth is
//! `docs/skycot/pricing/v1-tiers.toml`, compiled into the binary via
//! `include_str!`. Three tiers are defined (`free`, `pro`,
//! `enterprise`); each carries a worker cap, a monthly action cap,
//! and an opaque feature-flag list. `Tier::allow_action` returns a
//! `AllowDecision` enum (not a bool) so callers can surface a
//! user-readable denial reason directly.
//!
//! ```
//! use argo_identity::tier::{AllowDecision, Tier, TierConfig};
//! let cfg = TierConfig::default_config();
//! let limits = cfg.limits(Tier::Free).expect("free tier present");
//! assert!(matches!(
//!     Tier::Free.allow_action("agent.run", 0, limits),
//!     AllowDecision::Allow
//! ));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

const DEFAULT_TOML: &str = include_str!("../../../docs/skycot/pricing/v1-tiers.toml");

/// One of the three v1 pricing tiers. Lowercase serialization matches
/// the Stripe product slug and the `workspace.tier` SQL column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Free tier — mock provider only, single worker, 100 actions/mo.
    Free,
    /// Pro tier — $29/mo, real providers, 5 workers, 5k actions/mo.
    Pro,
    /// Enterprise tier — custom-quoted, unlimited workers/actions.
    Enterprise,
}

impl Tier {
    /// Stable lowercase label used for serialization, SQL, and Stripe.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Enterprise => "enterprise",
        }
    }

    /// Default tier for a freshly-provisioned tenant.
    #[must_use]
    pub fn default_tier() -> Self {
        Self::Free
    }

    /// Apply the tier's feature + quota gates to an action attempt.
    /// Returns [`AllowDecision::Allow`] when the action passes, or
    /// [`AllowDecision::Deny`] with a human-readable reason otherwise.
    #[must_use]
    pub fn allow_action(self, action_kind: &str, used_actions: u64, limits: &TierLimits) -> AllowDecision {
        let cap = limits.max_actions_per_month;
        if cap > 0 && used_actions >= cap {
            return AllowDecision::Deny(format!(
                "tier {tier} monthly action cap reached: {used_actions}/{cap}",
                tier = self.as_str()
            ));
        }
        if !limits.allows(action_kind) {
            return AllowDecision::Deny(format!(
                "tier {tier} does not include feature '{action_kind}'",
                tier = self.as_str()
            ));
        }
        AllowDecision::Allow
    }
}

impl FromStr for Tier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "free" => Ok(Self::Free),
            "pro" => Ok(Self::Pro),
            "enterprise" => Ok(Self::Enterprise),
            other => Err(format!("unknown tier '{other}'")),
        }
    }
}

/// Per-tier quotas and feature-flag set. `max_workers == 0` and
/// `max_actions_per_month == 0` both encode "unlimited".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierLimits {
    /// Human-facing label (e.g. `"Pro"`).
    pub display_name: String,
    /// Stripe-billed monthly price in whole USD. `0` for `free` and for
    /// custom-quoted enterprise.
    #[serde(default)]
    pub price_monthly_usd: u32,
    /// Maximum concurrent workers; `0` = unlimited.
    #[serde(default)]
    pub max_workers: u32,
    /// Calendar-month action budget; `0` = unlimited.
    #[serde(default)]
    pub max_actions_per_month: u64,
    /// Opaque feature flags the tier unlocks (e.g. `"channel_voice"`).
    #[serde(default)]
    pub features: Vec<String>,
}

impl TierLimits {
    /// Whether the tier unlocks a named feature. Action kinds that are
    /// not explicit feature flags (e.g. generic `"agent.run"`) are
    /// always permitted; only listed gates apply.
    #[must_use]
    pub fn allows(&self, feature_or_action: &str) -> bool {
        if !feature_or_action.starts_with("feature:") {
            return true;
        }
        let needle = feature_or_action.trim_start_matches("feature:");
        self.features.iter().any(|f| f == needle)
    }
}

/// Decoded `v1-tiers.toml` — a map keyed by tier slug.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TierConfig {
    tiers: BTreeMap<String, TierLimits>,
}

impl TierConfig {
    /// Parse a TOML document into a `TierConfig`.
    pub fn from_toml_str(s: &str) -> Result<Self, String> {
        toml::from_str::<BTreeMap<String, TierLimits>>(s)
            .map(|tiers| Self { tiers })
            .map_err(|e| format!("tier toml parse: {e}"))
    }

    /// Compiled-in defaults from `docs/skycot/pricing/v1-tiers.toml`.
    /// Panics only if the bundled file is malformed — which would also
    /// fail `cargo test -p argo-identity`, so a runtime panic here
    /// signals a build-time regression.
    #[must_use]
    pub fn default_config() -> Self {
        Self::from_toml_str(DEFAULT_TOML)
            .unwrap_or_else(|e| panic!("bundled v1-tiers.toml is invalid: {e}"))
    }

    /// Limits for a tier, or `None` when the tier slug is unknown.
    #[must_use]
    pub fn limits(&self, tier: Tier) -> Option<&TierLimits> {
        self.tiers.get(tier.as_str())
    }
}

/// Result of a tier feature-gate check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowDecision {
    /// Action proceeds.
    Allow,
    /// Action is rejected; payload carries the user-facing reason.
    Deny(String),
}

#[cfg(test)]
#[path = "tier_tests.rs"]
mod tests;
