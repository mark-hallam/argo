//! Plan provenance annotation
//! (`mission-planner-bias-from-template`).
//!
//! Records HOW a planner produced a workflow so operators reading
//! the audit chain (or the `argo plan generate --json` envelope)
//! can distinguish a learned-template bias from a cold-generate
//! pass. The runtime stamps one of these on every
//! `DecomposeResult` it emits.
//!
//! Defined in `argo-types` (zero-dep) so every consumer — runtime,
//! CLI, audit, gateway — can deserialise it without taking a heavy
//! crate dependency. Lives behind serde so it travels cleanly in
//! the audit-chain `payload_json`.

use serde::{Deserialize, Serialize};

/// One of three sources for a freshly emitted plan. Carried on the
/// `DecomposeResult` and stamped into the audit chain so operators
/// can answer "why this plan?" without re-running the planner.
///
/// Variants are flat (no nested struct) so the JSON shape is easy
/// to query from operator tooling (`jq '.provenance.kind'`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanProvenance {
    /// The planner produced a fresh decomposition (`TemplatePlanner`
    /// or `LlmPlanner` cold path). Default when no learned template
    /// matched the task signature.
    #[default]
    ColdGenerated,

    /// One of the shipped-default templates matched. Distinct from
    /// `ColdGenerated` because the body comes from a hardcoded
    /// template, not a fresh LLM call. Useful for operators trying
    /// to spot when a built-in template is doing the work.
    BuiltIn {
        /// Template name (matches `PlanTemplate::name`).
        template_name: String,
    },

    /// A Dreams-curated learned template biased the planner. The
    /// `template_id` ties back to a `learned_plan_template` row so
    /// the operator can pull provenance via
    /// `argo plan template show <id>`.
    FromLearnedTemplate {
        /// Stable UUIDv7 id of the `learned_plan_template` row.
        template_id: String,
        /// Stable task signature that matched.
        signature: String,
    },
}

impl PlanProvenance {
    /// Short label used in CLI summary lines and audit JSON
    /// envelopes. Matches the serde `kind` value.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::ColdGenerated => "cold_generated",
            Self::BuiltIn { .. } => "built_in",
            Self::FromLearnedTemplate { .. } => "from_learned_template",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_generated_round_trips() {
        let p = PlanProvenance::ColdGenerated;
        let j = serde_json::to_string(&p).expect("ser");
        assert_eq!(j, r#"{"kind":"cold_generated"}"#);
        let back: PlanProvenance = serde_json::from_str(&j).expect("de");
        assert_eq!(back, p);
        assert_eq!(p.kind(), "cold_generated");
    }

    #[test]
    fn built_in_carries_template_name() {
        let p = PlanProvenance::BuiltIn {
            template_name: "approval-gate".into(),
        };
        let j = serde_json::to_string(&p).expect("ser");
        let back: PlanProvenance = serde_json::from_str(&j).expect("de");
        assert_eq!(back, p);
        assert_eq!(p.kind(), "built_in");
    }

    #[test]
    fn from_learned_template_round_trips() {
        let p = PlanProvenance::FromLearnedTemplate {
            template_id: "01HQK9N7VV8X4S0M8E0A6R8H3W".into(),
            signature: "vertical=ebay-recon|skill=research|shape=goal".into(),
        };
        let j = serde_json::to_string(&p).expect("ser");
        let back: PlanProvenance = serde_json::from_str(&j).expect("de");
        assert_eq!(back, p);
        assert_eq!(p.kind(), "from_learned_template");
    }

    #[test]
    fn default_is_cold_generated() {
        assert_eq!(PlanProvenance::default(), PlanProvenance::ColdGenerated);
    }
}
