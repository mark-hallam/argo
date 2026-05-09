//! Public input/output types for `ComputerUseExecutor` plus the
//! default bundle-dir helper. Split out of `executor.rs` to give
//! the orchestrator headroom against the 500-LOC ceiling. All
//! items are re-exported from `executor.rs` so call sites remain
//! unchanged.

use crate::budget::BudgetSnapshot;
use argo_types::SessionId;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Public input to `ComputerUseExecutor::run`. Carries everything
/// the executor needs to fully drive one CU dispatch.
#[derive(Debug, Clone)]
pub struct RunRequest {
    pub action_id: String,
    pub job_id: String,
    pub tenant_id: String,
    pub persona_id: String,
    pub profile_name: String,
    /// Operator-friendly system prompt. Should describe the
    /// task, the URL whitelist, and any photo paths mounted in
    /// the sandbox.
    pub system_prompt: String,
    /// Optional per-action USD cap, sourced from the manifest's
    /// `executor.budget_usd` tagged field.
    pub budget_usd: Option<f64>,
    /// Override the bundle dir (tests use this to pin to a
    /// tempdir). When `None`, defaults to
    /// `.skycot/outputs/<job_id>/actions/<action_id>/`.
    pub bundle_dir: Option<PathBuf>,
    /// Optional session id used by the skill-learning hook
    /// (Stream `claude-cu-skill-learn-wire`). Plumbed in at
    /// dispatch time so a learned-skill proposal carries the
    /// originating session in its `learned:<short>` provenance.
    /// When `None`, the learning hook still runs but skips
    /// the propose call (no session = no audit chain).
    pub session_id: Option<SessionId>,
}

impl RunRequest {
    /// Demo helper — populates required fields with placeholder
    /// values. Used in the lib.rs doctest; not for production.
    #[must_use]
    pub fn demo(action_id: &str, profile: &str) -> Self {
        Self {
            action_id: action_id.into(),
            job_id: "demo-job".into(),
            tenant_id: "demo-tenant".into(),
            persona_id: "demo-persona".into(),
            profile_name: profile.into(),
            system_prompt: "Navigate to example.com and find the first link.".into(),
            budget_usd: Some(0.10),
            bundle_dir: None,
            session_id: None,
        }
    }
}

/// Outcome of one dispatch. Mirrors Stream A's `ActionOutcome`
/// enum (HumanHandoff / Completed) — the cli shim maps from
/// this type into that.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    Completed {
        result: RunResult,
        bundle_dir: PathBuf,
        budget: BudgetSnapshot,
    },
    HumanHandoff {
        reason: String,
        bundle_dir: PathBuf,
        budget: BudgetSnapshot,
    },
}

/// Successful-completion result. Shipped as the `result_json`
/// stamped into the auto-completed action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub final_text: Option<String>,
    pub actions_taken: Vec<String>,
    pub tokens_used: u32,
}

pub(crate) fn default_bundle_dir(req: &RunRequest) -> PathBuf {
    PathBuf::from(".skycot")
        .join("outputs")
        .join(&req.job_id)
        .join("actions")
        .join(&req.action_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_serialises_with_outcome_tag() {
        let v = Outcome::HumanHandoff {
            reason: "test".into(),
            bundle_dir: PathBuf::from("/tmp/test"),
            budget: BudgetSnapshot {
                tokens_used: 0,
                tokens_cap: 100,
                steps_used: 0,
                steps_cap: 5,
                wallclock_seconds_used: 0,
                wallclock_seconds_cap: 60,
            },
        };
        let serialised = serde_json::to_value(&v).unwrap();
        assert_eq!(serialised["outcome"], "human_handoff");
        assert_eq!(serialised["reason"], "test");
    }

    #[test]
    fn run_request_demo_populates_defaults() {
        let r = RunRequest::demo("act-1", "marketplace_browser");
        assert_eq!(r.action_id, "act-1");
        assert_eq!(r.profile_name, "marketplace_browser");
        assert_eq!(r.budget_usd, Some(0.10));
        assert!(r.system_prompt.contains("example.com"));
    }

    #[test]
    fn default_bundle_dir_uses_skycot_outputs_layout() {
        let r = RunRequest {
            action_id: "act-7".into(),
            job_id: "job-9".into(),
            tenant_id: "t".into(),
            persona_id: "p".into(),
            profile_name: "marketplace_browser".into(),
            system_prompt: "x".into(),
            budget_usd: None,
            bundle_dir: None,
            session_id: None,
        };
        let dir = default_bundle_dir(&r);
        assert!(dir.ends_with(".skycot/outputs/job-9/actions/act-7"));
    }
}
