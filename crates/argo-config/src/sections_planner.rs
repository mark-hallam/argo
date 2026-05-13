//! `[planner.*]` config sections.
//!
//! Hosts the operator-tunable knobs for the planner-bias path
//! (`mission-planner-bias-from-template`). Lives in its own
//! sibling file (re-exported via [`crate::sections`]) so
//! [`crate::sections`] stays a barrel and every concern stays
//! ≤ 250 LOC.
//!
//! Currently holds:
//!
//! - `[planner.bias]` — knobs for the
//!   `argo_runtime::planner_bias::PlannerBias` wrapper. Decides
//!   whether the planner consults the Dreams-curated
//!   `learned_plan_template` store before falling through to
//!   cold-generate, and the trade-offs (per-vertical preference,
//!   recency window).

use serde::{Deserialize, Serialize};

/// Default master enable for `[planner.bias]`. Bias defaults on
/// because the "skills get better over time" promise depends on
/// the wire being live; the gate on `Promoted` status keeps the
/// blast radius bounded.
pub const DEFAULT_PLANNER_BIAS_ENABLED: bool = true;
/// Default preference for per-vertical templates when multiple
/// learned templates match a signature.
pub const DEFAULT_PLANNER_BIAS_PREFER_PER_VERTICAL: bool = true;
/// Default recency window (days). Templates older than this fall
/// through to cold-generate even when matched. `0` disables the
/// recency gate entirely.
pub const DEFAULT_PLANNER_BIAS_REQUIRE_RECENT_WITHIN_DAYS: u32 = 30;

/// `[planner.bias]` — operator knobs for the
/// `argo_runtime::planner_bias::PlannerBias` wrapper.
///
/// Defaults map to the v1 "experimentation-friendly" posture:
/// bias enabled, per-vertical preference on, recency cap at 30
/// days. Operators on stable Dreams curation tighten by raising
/// `require_recent_within_days = 0` (ignore recency) once the
/// curation cadence is well-understood.
///
/// **Trust-grade gating note** (`mission-planner-bias-from-template`
/// drift-fix amendment 2026-05-13): v1 gates on
/// `LearnedPlanTemplate.trust_grade == Promoted` only. The
/// `Pending / Promoted / Rejected` lifecycle (see
/// `argo_store::learned_plan_template::TrustGrade`) does not yet
/// expose finer-grained tiers, so a `min_trust_grade` knob is a
/// follow-up brief. The brief's earlier `Bronze/Silver/Gold`
/// vocabulary referred to the older skill TrustGrade ladder and
/// is misaligned with the live `LearnedPlanTemplate` lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlannerBiasConfig {
    /// Master enable. When `false`, the planner ALWAYS
    /// cold-generates regardless of which learned templates would
    /// have matched. The provenance the caller stamps degrades to
    /// `PlanProvenance::ColdGenerated` whenever this is `false`.
    pub enabled: bool,
    /// When multiple learned templates match a signature, prefer
    /// the one scoped to the calling worker's `vertical_id` over
    /// the all-verticals fallback. Mirrors the
    /// operator_risk_overrides scope-precedence pattern (#1361).
    pub prefer_per_vertical: bool,
    /// Templates whose newest activity is older than this window
    /// (days) fall through to cold-generate. `0` disables the
    /// recency gate (stale templates still match). 30 is the v1
    /// default — strict enough that abandoned templates don't
    /// bias a worker, loose enough that a healthy curation cadence
    /// keeps templates in scope.
    pub require_recent_within_days: u32,
}

impl Default for PlannerBiasConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_PLANNER_BIAS_ENABLED,
            prefer_per_vertical: DEFAULT_PLANNER_BIAS_PREFER_PER_VERTICAL,
            require_recent_within_days: DEFAULT_PLANNER_BIAS_REQUIRE_RECENT_WITHIN_DAYS,
        }
    }
}

/// `[planner]` parent for planner-side TOML knobs. Currently only
/// wraps `[planner.bias]`; future sibling sub-tables (e.g.
/// `[planner.llm_fallback]`) extend here.
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlannerConfig {
    /// `[planner.bias]`.
    pub bias: PlannerBiasConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_bias_defaults_match_constants() {
        let cfg = PlannerBiasConfig::default();
        assert!(cfg.enabled);
        assert!(cfg.prefer_per_vertical);
        assert_eq!(cfg.require_recent_within_days, 30);
    }

    #[test]
    fn planner_bias_parses_from_toml() {
        let body = r#"
[bias]
enabled = false
prefer_per_vertical = false
require_recent_within_days = 7
"#;
        let cfg: PlannerConfig = toml::from_str(body).expect("parse");
        assert!(!cfg.bias.enabled);
        assert!(!cfg.bias.prefer_per_vertical);
        assert_eq!(cfg.bias.require_recent_within_days, 7);
    }

    #[test]
    fn empty_toml_yields_planner_defaults() {
        let cfg: PlannerConfig = toml::from_str("").expect("parse");
        assert_eq!(cfg.bias, PlannerBiasConfig::default());
    }

    #[test]
    fn zero_recency_window_parses() {
        let body = "[bias]\nrequire_recent_within_days = 0\n";
        let cfg: PlannerConfig = toml::from_str(body).expect("parse");
        assert_eq!(cfg.bias.require_recent_within_days, 0);
        assert!(cfg.bias.enabled, "other fields stay at default");
    }
}
