//! Shared fixture for `skill_loop_closure_smoke` — synthetic CU
//! trajectory that clears `argo_skill::learn::DEFAULT_SUCCESS_THRESHOLD`
//! so the closure test can drive the same `SkillCandidate` shape the
//! dispatch shim would build at runtime.
//!
//! Imported via `#[path = "fixtures/skill_trajectory_seed.rs"] mod …;`
//! — Cargo treats files under `tests/<subdir>/` as plain modules
//! rather than separate test binaries, so this fixture costs nothing
//! at build time when the closure test doesn't pull it.

#![allow(dead_code)]

use argo_skill::learn::{should_propose, SkillCandidate, DEFAULT_SUCCESS_THRESHOLD};

/// Trajectory metadata for one CU completion. Stays minimal — the
/// runtime captures more (timings, screenshot ids) but the proposal
/// pipeline only consumes the fields below.
pub struct SeededTrajectory {
    pub action_type: &'static str,
    pub vertical_id: &'static str,
    pub steps: Vec<String>,
    pub final_text: Option<String>,
    pub tokens_used: u32,
    pub success_metric: f64,
}

/// Default "successful action" — 5 steps, success 0.92, well above
/// the 0.8 threshold. Returns a deterministic shape so two test runs
/// produce identical SkillCandidate names.
#[must_use]
pub fn high_confidence_post_listing(vertical_id: &'static str) -> SeededTrajectory {
    SeededTrajectory {
        action_type: "post_listing",
        vertical_id,
        steps: vec![
            "navigate marketplace.fb.com".to_string(),
            "click Sell".to_string(),
            "fill title field".to_string(),
            "upload first photo".to_string(),
            "confirm preview + submit".to_string(),
        ],
        final_text: Some("https://fb.com/marketplace/abc123".to_string()),
        tokens_used: 42_000,
        success_metric: 0.92,
    }
}

/// Below-threshold trajectory — drives the negative half of
/// `should_propose`'s gate so the test asserts the propose call is
/// guarded, not unconditional.
#[must_use]
pub fn low_confidence_trajectory(vertical_id: &'static str) -> SeededTrajectory {
    SeededTrajectory {
        action_type: "post_listing",
        vertical_id,
        steps: vec!["click Sell".to_string(), "fail captcha".to_string()],
        final_text: None,
        tokens_used: 5_000,
        success_metric: 0.4,
    }
}

/// Returns `Some(candidate)` iff the trajectory clears
/// [`DEFAULT_SUCCESS_THRESHOLD`] — mirrors the dispatch shim's
/// runtime guard so the integration test exercises the real branch.
#[must_use]
pub fn maybe_propose(name: &str, traj: &SeededTrajectory) -> Option<SkillCandidate> {
    if !should_propose(traj.success_metric, DEFAULT_SUCCESS_THRESHOLD) {
        return None;
    }
    Some(SkillCandidate::from_cu_trajectory(
        name.to_string(),
        traj.action_type,
        traj.vertical_id,
        traj.steps.clone(),
        traj.final_text.clone(),
        traj.tokens_used,
        traj.success_metric,
    ))
}
