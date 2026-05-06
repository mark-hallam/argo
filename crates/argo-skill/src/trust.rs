//! Trust grades for skill provenance (H-4a).
//!
//! The brief introduces a five-rung trust ladder so the
//! `list_skills_for_action` lookup can refuse to surface a
//! `learned(unreviewed)` skill when the action's risk demands a
//! signed `published(verified)` one.
//!
//! ```
//! use argo_skill::trust::{TrustGrade, min_trust_for_risk};
//! // The derived Ord goes from MOST trusted (smallest variant
//! // index = Builtin) to LEAST. So Builtin < Tenant under `<`,
//! // meaning Builtin is MORE trusted.
//! assert!(TrustGrade::Builtin < TrustGrade::Tenant);
//! assert_eq!(min_trust_for_risk("high"), TrustGrade::PublishedVerified);
//! ```
//!
//! H-4a (this slice) ships the **enum + risk-to-min-trust map +
//! filter helper**. The two-tier workspace→library walk and the
//! skill-learning loop (B's success-metric promotion) land in
//! follow-up slices that depend on this type.

use serde::{Deserialize, Serialize};

use crate::skill::Provenance;

/// Five-rung ladder of skill trust.
///
/// Ordered from MOST trusted (Builtin) to LEAST (LearnedUnreviewed).
/// Comparison reflects the ordering — a higher variant is more
/// trusted than a lower one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustGrade {
    /// Skill ships in the binary; never modified at runtime.
    /// HIGHEST trust — code review at compile time.
    Builtin,
    /// Skill is signed by a verified marketplace publisher.
    /// Trusted for high-risk actions.
    PublishedVerified,
    /// Tenant-installed skill (mutable per-tenant). Trusted for
    /// the tenant's own work but not cross-tenant promotion.
    Tenant,
    /// Skill from the marketplace community without verification.
    /// Acceptable for medium-risk actions.
    PublishedCommunity,
    /// Skill the runtime learned from a successful trajectory but
    /// no human has reviewed yet. LOWEST trust — only safe for
    /// low-risk dry-run work until reviewed.
    LearnedUnreviewed,
}

impl TrustGrade {
    /// Map an existing [`Provenance::source`] string to a
    /// [`TrustGrade`]. The mapping is conservative: anything
    /// unrecognised falls to `LearnedUnreviewed` (the strictest
    /// gate).
    ///
    /// Recognised prefixes:
    /// - `"builtin"` → `Builtin`
    /// - `"bundle:..."` → `Builtin` (vertical-bundled skills are
    ///   treated as code-review-equivalent to builtins because
    ///   the vertical itself is signed).
    /// - `"user"` → `Tenant`
    /// - `"forge"` → `Tenant` (workspace-grown skills the operator
    ///   wrote themselves).
    /// - `"published:verified:..."` → `PublishedVerified`
    /// - `"published:community:..."` → `PublishedCommunity`
    /// - `"learned"` → `LearnedUnreviewed`
    /// - anything else → `LearnedUnreviewed`
    ///
    /// ```
    /// use argo_skill::trust::TrustGrade;
    /// assert_eq!(TrustGrade::from_source("builtin"), TrustGrade::Builtin);
    /// assert_eq!(TrustGrade::from_source("bundle:fix"), TrustGrade::Builtin);
    /// assert_eq!(TrustGrade::from_source("user"), TrustGrade::Tenant);
    /// assert_eq!(
    ///     TrustGrade::from_source("published:verified:acme"),
    ///     TrustGrade::PublishedVerified,
    /// );
    /// assert_eq!(
    ///     TrustGrade::from_source("totally-unknown"),
    ///     TrustGrade::LearnedUnreviewed,
    /// );
    /// ```
    #[must_use]
    pub fn from_source(source: &str) -> Self {
        if source == "builtin" {
            return Self::Builtin;
        }
        if source.starts_with("bundle:") {
            return Self::Builtin;
        }
        if source == "user" || source == "forge" {
            return Self::Tenant;
        }
        if source.starts_with("published:verified:") {
            return Self::PublishedVerified;
        }
        if source.starts_with("published:community:") {
            return Self::PublishedCommunity;
        }
        if source == "learned" || source.starts_with("learned:") {
            return Self::LearnedUnreviewed;
        }
        Self::LearnedUnreviewed
    }
}

/// Minimum trust grade required to surface a skill for an action
/// of the given risk level. Risk levels are the same strings the
/// vertical manifest emits: `"low"` | `"medium"` | `"high"`.
/// Unknown labels default to the strictest gate (high).
///
/// Mapping rationale:
/// - **low**: any skill is fine, including learned/unreviewed.
///   The action manifest already guarantees the action is
///   reversible / draft-only.
/// - **medium**: published community is the floor — we want at
///   least *some* social-trust signal before doing reversible
///   public-facing work.
/// - **high**: only verified-publisher or builtin skills surface
///   for genuinely irreversible, money-touching, or
///   human-touching work.
///
/// ```
/// use argo_skill::trust::{TrustGrade, min_trust_for_risk};
/// assert_eq!(min_trust_for_risk("low"), TrustGrade::LearnedUnreviewed);
/// assert_eq!(min_trust_for_risk("medium"), TrustGrade::PublishedCommunity);
/// assert_eq!(min_trust_for_risk("high"), TrustGrade::PublishedVerified);
/// // Unknown label → strictest gate.
/// assert_eq!(min_trust_for_risk("zomg"), TrustGrade::PublishedVerified);
/// ```
#[must_use]
pub fn min_trust_for_risk(risk: &str) -> TrustGrade {
    match risk {
        "low" => TrustGrade::LearnedUnreviewed,
        "medium" => TrustGrade::PublishedCommunity,
        // "high" or anything we don't recognise → strictest gate.
        _ => TrustGrade::PublishedVerified,
    }
}

/// `true` iff a skill with `provenance` may be surfaced for an
/// action of the given `risk` level. The check uses
/// [`TrustGrade::from_source`] + [`min_trust_for_risk`] and the
/// derived `Ord` on [`TrustGrade`].
///
/// ```
/// use argo_skill::trust::is_skill_eligible;
/// use argo_skill::Provenance;
///
/// let builtin = Provenance { source: "builtin".into(), hash: "h".into() };
/// assert!(is_skill_eligible(&builtin, "high"));
///
/// let learned = Provenance { source: "learned".into(), hash: "h".into() };
/// assert!(is_skill_eligible(&learned, "low"));
/// assert!(!is_skill_eligible(&learned, "high"));
/// ```
#[must_use]
pub fn is_skill_eligible(provenance: &Provenance, risk: &str) -> bool {
    let skill_grade = TrustGrade::from_source(&provenance.source);
    let required = min_trust_for_risk(risk);
    // Ord is from MOST trusted (smallest variant index = Builtin)
    // to LEAST. So `skill_grade <= required` means the skill is
    // AT LEAST as trusted as the required floor.
    skill_grade <= required
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(source: &str) -> Provenance {
        Provenance {
            source: source.into(),
            hash: "sha256-x".into(),
        }
    }

    #[test]
    fn ordering_is_most_to_least_trusted() {
        assert!(TrustGrade::Builtin < TrustGrade::Tenant);
        assert!(TrustGrade::PublishedVerified < TrustGrade::PublishedCommunity);
        assert!(TrustGrade::PublishedCommunity < TrustGrade::LearnedUnreviewed);
    }

    #[test]
    fn from_source_recognises_known_prefixes() {
        assert_eq!(TrustGrade::from_source("builtin"), TrustGrade::Builtin);
        assert_eq!(TrustGrade::from_source("bundle:gtm"), TrustGrade::Builtin);
        assert_eq!(TrustGrade::from_source("user"), TrustGrade::Tenant);
        assert_eq!(TrustGrade::from_source("forge"), TrustGrade::Tenant);
        assert_eq!(
            TrustGrade::from_source("published:verified:acme"),
            TrustGrade::PublishedVerified,
        );
        assert_eq!(
            TrustGrade::from_source("published:community:bob"),
            TrustGrade::PublishedCommunity,
        );
        assert_eq!(
            TrustGrade::from_source("learned"),
            TrustGrade::LearnedUnreviewed,
        );
        assert_eq!(
            TrustGrade::from_source("learned:session-42"),
            TrustGrade::LearnedUnreviewed,
        );
    }

    #[test]
    fn from_source_unknown_falls_to_learned_unreviewed() {
        // The strictest gate is the safest default for unrecognised
        // sources — better to refuse to surface than to surface
        // something we can't grade.
        assert_eq!(
            TrustGrade::from_source("nonsense"),
            TrustGrade::LearnedUnreviewed,
        );
        assert_eq!(TrustGrade::from_source(""), TrustGrade::LearnedUnreviewed);
    }

    #[test]
    fn risk_tier_map_matches_expected_floors() {
        assert_eq!(min_trust_for_risk("low"), TrustGrade::LearnedUnreviewed);
        assert_eq!(min_trust_for_risk("medium"), TrustGrade::PublishedCommunity);
        assert_eq!(min_trust_for_risk("high"), TrustGrade::PublishedVerified);
        // Unknown defaults to strictest.
        assert_eq!(min_trust_for_risk(""), TrustGrade::PublishedVerified);
        assert_eq!(min_trust_for_risk("ULTRA"), TrustGrade::PublishedVerified);
    }

    #[test]
    fn builtin_is_eligible_for_every_risk() {
        let pv = p("builtin");
        for risk in ["low", "medium", "high"] {
            assert!(is_skill_eligible(&pv, risk), "builtin should pass {risk}");
        }
    }

    #[test]
    fn learned_unreviewed_only_low_risk() {
        let pv = p("learned");
        assert!(is_skill_eligible(&pv, "low"));
        assert!(!is_skill_eligible(&pv, "medium"));
        assert!(!is_skill_eligible(&pv, "high"));
    }

    #[test]
    fn tenant_eligible_for_low_and_medium_only() {
        // Tenant trust is ABOVE PublishedCommunity (i.e. MORE
        // trusted than the medium floor) but BELOW
        // PublishedVerified (the high floor). So tenant skills
        // surface for low + medium, NOT high.
        let pv = p("user");
        assert!(is_skill_eligible(&pv, "low"));
        assert!(is_skill_eligible(&pv, "medium"));
        assert!(!is_skill_eligible(&pv, "high"));
    }

    #[test]
    fn published_verified_eligible_for_every_risk() {
        let pv = p("published:verified:big-vendor");
        for risk in ["low", "medium", "high"] {
            assert!(
                is_skill_eligible(&pv, risk),
                "published:verified should pass {risk}"
            );
        }
    }

    #[test]
    fn published_community_eligible_for_low_and_medium_only() {
        let pv = p("published:community:cool-dev");
        assert!(is_skill_eligible(&pv, "low"));
        assert!(is_skill_eligible(&pv, "medium"));
        assert!(!is_skill_eligible(&pv, "high"));
    }

    #[test]
    fn unknown_risk_defaults_to_strictest() {
        // Even a tenant skill must be REJECTED for unknown risk
        // — the unknown maps to the high-risk gate
        // (PublishedVerified), and Tenant is below that.
        let pv = p("user");
        assert!(!is_skill_eligible(&pv, "asteroid"));
    }

    #[test]
    fn serialize_round_trip_uses_snake_case() {
        let grade = TrustGrade::PublishedVerified;
        let s = serde_json::to_string(&grade).expect("serialize");
        assert_eq!(s, "\"published_verified\"");
        let back: TrustGrade = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back, grade);
    }
}
