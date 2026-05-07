//! Stream H-4c — skill-learning loop primitives.
//!
//! When a `ComputerUseExecutor` (Stream B) completes a novel
//! sequence with `success_metric > threshold`, the runtime
//! serialises the trajectory + a Claude-distilled summary into
//! the existing `pending_review` skill lifecycle so a human can
//! approve it before it becomes part of the live skill library.
//!
//! This module ships the **wiring primitives** the runtime
//! consumes:
//!
//! - [`should_propose`] — pure threshold check.
//! - [`SkillCandidate`] — the data shape a CU completion turns
//!   into. Has a `to_skill_md` serialiser so the existing
//!   [`crate::SkillStore::propose_for_review`] entry point
//!   accepts the candidate without churn.
//! - [`SkillCandidate::from_cu_trajectory`] — builds a candidate
//!   from a CU outcome shape (final_text + actions_taken +
//!   tokens_used + success_metric).
//!
//! The Claude-distillation pass (turning the raw trajectory into
//! a polished SKILL.md body) lands as a follow-up — it requires
//! an LLM call, so the offline path here ships a deterministic
//! placeholder body the operator can refine before approving.
//! That keeps the loop testable without `ANTHROPIC_API_KEY` set.

use serde::{Deserialize, Serialize};

/// Threshold above which a successful CU trajectory is worth
/// proposing as a learned skill. Caller can override for
/// tenant-specific tuning; default is conservative
/// (high-success-only).
pub const DEFAULT_SUCCESS_THRESHOLD: f64 = 0.8;

/// Pure threshold check. Returns `true` when the CU
/// completion's `success_metric` clears `threshold`. Used by
/// the dispatch shim's post-execution hook to decide whether to
/// even build a candidate.
///
/// ```
/// use argo_skill::learn::{should_propose, DEFAULT_SUCCESS_THRESHOLD};
/// assert!(should_propose(0.95, DEFAULT_SUCCESS_THRESHOLD));
/// assert!(!should_propose(0.5, DEFAULT_SUCCESS_THRESHOLD));
/// // Boundary: equal-to-threshold does NOT propose (strictly >).
/// assert!(!should_propose(DEFAULT_SUCCESS_THRESHOLD, DEFAULT_SUCCESS_THRESHOLD));
/// ```
#[must_use]
pub fn should_propose(success_metric: f64, threshold: f64) -> bool {
    success_metric > threshold
}

/// Draft skill candidate distilled from a CU trajectory. Lives
/// in this module so the dispatch shim can build one without
/// pulling in the `propose_for_review` SqlitePool path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillCandidate {
    /// Skill name — must be unique per workspace. Caller picks;
    /// recommended convention is `<action_type>_<seq_short>`.
    pub name: String,
    /// SemVer-shaped version. Caller picks; the `propose_for_review`
    /// step refuses if a row with the same `(name, version)` is
    /// already installed.
    pub version: String,
    /// One-line description visible to the model when the skill
    /// is loaded into a future session's system prompt.
    pub description: String,
    /// CU action trajectory — one entry per step the model took.
    /// Persisted in the SKILL.md body so reviewers see the
    /// sequence that earned the proposal.
    pub trajectory: Vec<String>,
    /// Final text the CU loop captured (URL, confirmation
    /// message, etc.). Surfaces in the SKILL.md as the success
    /// signature.
    pub final_text: Option<String>,
    /// Tokens used during the CU loop. Audit-only.
    pub tokens_used: u32,
    /// Vertical the candidate belongs to. Required because the
    /// `pending_review` lifecycle is vertical-scoped.
    pub vertical_id: String,
    /// Stable threshold-clearing success metric. Stored so the
    /// reviewer sees what motivated the proposal.
    pub success_metric: f64,
    /// Source of the candidate body. `None` for the legacy
    /// mechanical path (`from_cu_trajectory`); `Some("dreams_distilled")`
    /// when produced by the Dreams cohort distillation pipeline
    /// in [`crate::dreams_distillation`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_source: Option<String>,
}

impl SkillCandidate {
    /// Build a candidate from a CU trajectory + metadata. Used
    /// by the dispatch shim's post-execution hook when a CU
    /// completion clears [`should_propose`].
    ///
    /// `name` should be unique per workspace; the convention is
    /// `<action_type>_<short_session_id>` so reviews are
    /// reproducible.
    #[must_use]
    pub fn from_cu_trajectory(
        name: impl Into<String>,
        action_type: &str,
        vertical_id: &str,
        trajectory: Vec<String>,
        final_text: Option<String>,
        tokens_used: u32,
        success_metric: f64,
    ) -> Self {
        let name = name.into();
        Self {
            description: format!(
                "Learned from a successful `{action_type}` run \
                 (success {:.2}, {} steps)",
                success_metric,
                trajectory.len()
            ),
            name,
            version: "0.1.0".into(),
            trajectory,
            final_text,
            tokens_used,
            vertical_id: vertical_id.to_string(),
            success_metric,
            body_source: None,
        }
    }

    /// Serialise to a SKILL.md document the existing
    /// [`crate::SkillStore::propose_for_review`] entry point
    /// accepts. Source string is `learned:<session>` so
    /// [`crate::TrustGrade::from_source`] grades it as
    /// `LearnedUnreviewed` until the human approves.
    #[must_use]
    pub fn to_skill_md(&self, session_short: &str) -> String {
        let mut body = String::new();
        body.push_str("---\n");
        body.push_str(&format!("name: {}\n", self.name));
        body.push_str(&format!("version: {}\n", self.version));
        body.push_str(&format!(
            "description: {}\n",
            json_string(&self.description)
        ));
        body.push_str("capabilities:\n  - cu\n");
        body.push_str("provenance:\n");
        body.push_str(&format!("  source: learned:{session_short}\n"));
        body.push_str("  hash: sha256-pending\n");
        body.push_str("lock: none\n");
        body.push_str("---\n\n");
        body.push_str(&format!(
            "# {}\n\nThis skill was learned by a computer-use trajectory \
             with success metric **{:.2}**. The reviewer should refine the \
             body before approving.\n\n## Trajectory\n\n",
            self.name, self.success_metric
        ));
        for (i, step) in self.trajectory.iter().enumerate() {
            body.push_str(&format!("{}. {step}\n", i + 1));
        }
        body.push_str(&format!(
            "\n**Tokens used**: {}\n",
            self.tokens_used
        ));
        if let Some(ft) = self.final_text.as_deref() {
            body.push_str(&format!("\n**Final text**: {}\n", json_string(ft)));
        }
        body
    }
}

/// Minimal JSON-style escape for embedding free-form text in a
/// YAML scalar. Quotes the string and escapes `"` + `\` +
/// newlines so the SKILL.md frontmatter parses cleanly.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SkillCandidate {
        SkillCandidate::from_cu_trajectory(
            "post_listing_abc123",
            "post_listing",
            "gtm-engineer",
            vec![
                "step 1: navigate marketplace.fb.com".into(),
                "step 2: click Sell".into(),
                "step 3: fill title field".into(),
            ],
            Some("https://fb.com/marketplace/123".into()),
            42_000,
            0.92,
        )
    }

    #[test]
    fn should_propose_uses_strict_inequality() {
        assert!(should_propose(0.81, 0.80));
        assert!(!should_propose(0.80, 0.80));
        assert!(!should_propose(0.79, 0.80));
        // Negative values never propose.
        assert!(!should_propose(-0.1, 0.0));
    }

    #[test]
    fn from_cu_trajectory_populates_description_with_metadata() {
        let c = sample();
        assert!(c.description.contains("post_listing"));
        assert!(c.description.contains("0.92"));
        assert!(c.description.contains("3 steps"));
        assert_eq!(c.trajectory.len(), 3);
        assert_eq!(c.tokens_used, 42_000);
        assert_eq!(c.vertical_id, "gtm-engineer");
    }

    #[test]
    fn to_skill_md_emits_yaml_frontmatter_with_learned_source() {
        let c = sample();
        let md = c.to_skill_md("session-xyz");
        assert!(md.starts_with("---\n"));
        assert!(md.contains("name: post_listing_abc123"));
        assert!(md.contains("source: learned:session-xyz"));
        // body section
        assert!(md.contains("# post_listing_abc123"));
        assert!(md.contains("Trajectory"));
        assert!(md.contains("step 1: navigate marketplace.fb.com"));
        assert!(md.contains("**Tokens used**: 42000"));
        assert!(md.contains("https://fb.com/marketplace/123"));
    }

    #[test]
    fn to_skill_md_handles_missing_final_text() {
        let mut c = sample();
        c.final_text = None;
        let md = c.to_skill_md("session-1");
        assert!(!md.contains("Final text"));
    }

    #[test]
    fn to_skill_md_escapes_quotes_in_description() {
        let mut c = sample();
        c.description = r#"Action with "quotes" and \backslash\"#.into();
        let md = c.to_skill_md("s1");
        // Description rides in a YAML-safe quoted string.
        assert!(md.contains("\\\"quotes\\\""), "got: {md}");
        assert!(md.contains("\\\\backslash\\\\"), "got: {md}");
    }

    #[test]
    fn skill_candidate_round_trips_via_serde() {
        let c = sample();
        let json = serde_json::to_string(&c).unwrap();
        let parsed: SkillCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, c);
    }

    #[test]
    fn default_threshold_is_conservative_8_decile() {
        assert!((DEFAULT_SUCCESS_THRESHOLD - 0.8).abs() < f64::EPSILON);
    }
}
