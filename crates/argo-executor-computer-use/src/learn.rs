//! Stream `claude-cu-skill-learn-wire` — bridge between a
//! successful CU completion and the `argo-skill` learning loop.
//!
//! The brief: when the executor's CU loop emits `stop_success`,
//! score the trajectory and (above threshold) propose a
//! `SkillCandidate` through `SkillStore::propose_for_review`
//! so a human can review + approve before the skill goes live.
//!
//! ## Why this lives in its own module
//!
//! Stream B kept `executor.rs` close to the per-file 500-LOC
//! budget. Adding the learning hook there pushes it over. This
//! file holds:
//!
//! - [`success_metric_for_outcome`] — scoring heuristic (1.0 for
//!   explicit success, 0.0 otherwise)
//! - [`build_candidate`] — `RunResult` + request metadata →
//!   `SkillCandidate`
//! - [`propose_skill_from_run`] — fire the proposal through a
//!   caller-supplied `SkillStore`; logs + swallows errors so the
//!   action's outcome is unaffected
//!
//! Tests in `learn_tests.rs` cover the pure pieces; the
//! `SkillStore`-backed flow lives in
//! `tests/skill_learning.rs`.

use crate::executor::{Outcome, RunRequest, RunResult};
use argo_skill::{
    learn::{should_propose, SkillCandidate, DEFAULT_SUCCESS_THRESHOLD},
    SkillStore,
};
use argo_types::SessionId;

/// Scoring heuristic: `1.0` when the executor returned
/// `Outcome::Completed`, `0.0` otherwise. v1 placeholder per the
/// brief — nuanced metrics (operator-rated, trajectory-length,
/// model-confidence) land in a follow-up.
#[must_use]
pub fn success_metric_for_outcome(outcome: &Outcome) -> f64 {
    match outcome {
        Outcome::Completed { .. } => 1.0,
        Outcome::HumanHandoff { .. } => 0.0,
    }
}

/// Whether this outcome should produce a proposal at all. Wraps
/// the [`success_metric_for_outcome`] heuristic + the
/// [`should_propose`] threshold check from H-4c.
#[must_use]
pub fn should_propose_outcome(outcome: &Outcome, threshold: f64) -> bool {
    should_propose(success_metric_for_outcome(outcome), threshold)
}

/// Build a `SkillCandidate` from a successful run. The skill
/// name follows the convention `<action_type>_<session_short>`
/// where `session_short` is the first 8 chars of the session id
/// (matches the brief's note).
#[must_use]
pub fn build_candidate(
    request: &RunRequest,
    result: &RunResult,
    session: SessionId,
    success_metric: f64,
) -> SkillCandidate {
    let session_short = session_short(session);
    let action_type = action_type_from_request(request);
    let name = format!("{}_{}", action_type, session_short);
    SkillCandidate::from_cu_trajectory(
        name,
        action_type,
        // Fall back to "default" when the request has no
        // explicit profile — propose_for_review will refuse
        // empty vertical_id, so the caller must supply a
        // sensible default upstream.
        if request.profile_name.is_empty() {
            "default"
        } else {
            request.profile_name.as_str()
        },
        result.actions_taken.clone(),
        result.final_text.clone(),
        result.tokens_used,
        success_metric,
    )
}

/// First 8 chars of a session id. Used as the provenance
/// short-form so reviewers can correlate the proposal back to
/// the originating CU run.
#[must_use]
pub fn session_short(session: SessionId) -> String {
    let s = session.0.to_string();
    s.chars().take(8).collect()
}

/// Best-effort extraction of an action_type from the request.
/// The CU `RunRequest` carries the action_id (which the
/// dispatch shim plumbs through) and the profile_name; the
/// brief uses the action_type string from the manifest as the
/// skill-name prefix. We approximate by stripping any trailing
/// numeric suffix from `action_id` ("act-123" → "act").
fn action_type_from_request(request: &RunRequest) -> &str {
    // Strip the "act-<n>" / "act_<n>" pattern that the dispatch
    // shim emits; otherwise pass through.
    if let Some(pos) = request.action_id.find('-') {
        return &request.action_id[..pos];
    }
    if let Some(pos) = request.action_id.find('_') {
        return &request.action_id[..pos];
    }
    request.action_id.as_str()
}

/// Fire the proposal through `propose_for_review`. Errors are
/// logged + swallowed per the brief: a learning-loop failure
/// MUST NOT fail the action.
///
/// `vertical_id` is sourced from the candidate (so the caller
/// has already picked a sensible default if the request didn't
/// carry one). `session` powers the `learned_in` audit chain.
pub async fn propose_skill_from_run(
    store: &SkillStore,
    candidate: &SkillCandidate,
    session: SessionId,
) {
    let session_short = session_short(session);
    let raw = candidate.to_skill_md(&session_short);
    match store
        .propose_for_review(&raw, session, &candidate.vertical_id)
        .await
    {
        Ok(skill) => {
            tracing::info!(
                target: "argo_executor_computer_use::learn",
                skill_id = %skill.id,
                name = %candidate.name,
                "proposed learned skill for review",
            );
        }
        Err(err) => {
            tracing::warn!(
                target: "argo_executor_computer_use::learn",
                error = %err,
                name = %candidate.name,
                "skill proposal failed; continuing without learning",
            );
        }
    }
}

/// One-shot wrapper combining the success check + candidate
/// build + propose. Caller supplies the success threshold (use
/// [`DEFAULT_SUCCESS_THRESHOLD`] from H-4c by default).
///
/// Returns `Some(candidate)` when a proposal was attempted (the
/// caller can inspect the candidate post-hoc); `None` when the
/// outcome didn't clear the threshold.
pub async fn maybe_propose(
    store: Option<&SkillStore>,
    request: &RunRequest,
    outcome: &Outcome,
    session: SessionId,
    threshold: f64,
) -> Option<SkillCandidate> {
    let metric = success_metric_for_outcome(outcome);
    if !should_propose(metric, threshold) {
        return None;
    }
    let result = match outcome {
        Outcome::Completed { result, .. } => result.clone(),
        Outcome::HumanHandoff { .. } => return None,
    };
    let candidate = build_candidate(request, &result, session, metric);
    if let Some(s) = store {
        propose_skill_from_run(s, &candidate, session).await;
    }
    Some(candidate)
}

/// Re-export the H-4c threshold so callers can spell
/// `argo_executor_computer_use::learn::DEFAULT_THRESHOLD` rather
/// than reach into argo-skill directly. Keeps the dispatch shim
/// + tests focused on this crate's surface.
pub const DEFAULT_THRESHOLD: f64 = DEFAULT_SUCCESS_THRESHOLD;

#[cfg(test)]
#[path = "learn_tests.rs"]
mod tests;
