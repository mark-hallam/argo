//! Operator-feedback ingestion seam for Forge (Stream
//! `mission-forge-operator-feedback-eval-signal`, MISSION_BACKLOG
//! Section 3 Claim 4 #85).
//!
//! Today Forge runs on synthesised eval cases only — operator
//! thumbs are a parallel signal that influences
//! `should_propose_with_feedback` but never feed into the loss
//! function. This module opens the ingestion seam: pulls operator
//! votes from `argo-store::feedback` for a given skill within a
//! look-back window, maps each to a [`FeedbackEvalCase`] tagged
//! `source: "operator_feedback"`, and exposes
//! [`weighted_loss_score`] so the worker plugin can aggregate
//! operator-weighted and synthesised cases into one score.
//!
//! The 1.5× weight on operator cases is a v1 starting point; see
//! [`crate::config::FEEDBACK_CASE_WEIGHT`]. The 14-day look-back
//! is the v1 default; the per-call `window` parameter lets future
//! cron-based Forge runs override.
//!
//! Per-skill opt-out: if `skill_flag.no-forge-feedback = 1` for
//! the target skill, ingestion is skipped (empty vec, no error).
//! Useful for skills where operator feedback is noisy.
//!
//! Caller flow (typical):
//!
//! ```no_run
//! # use std::time::Duration;
//! # use argo_forge::{ForgeOptions, feedback_eval};
//! # async fn demo(store: &argo_store::SessionStore) -> argo_types::ArgoResult<()> {
//! let opts = ForgeOptions::new("billing-resolver", "0.1");
//! let cases = if opts.include_operator_feedback {
//!     feedback_eval::ingest_operator_feedback(
//!         store, &opts.skill_name, Duration::from_secs(14 * 24 * 3600),
//!     ).await?
//! } else {
//!     Vec::new()
//! };
//! // Hand `cases` to the worker plugin alongside synth examples.
//! # let _ = cases; Ok(()) }
//! ```
use crate::config::FEEDBACK_CASE_WEIGHT;
use argo_store::feedback::{list_feedback, Vote};
use argo_store::skill_flag::{get_flag_or_default, FLAG_NO_FORGE_FEEDBACK};
use argo_store::{runs_for_skill, SessionStore};
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

/// Hard cap on how many feedback rows we'll scan per ingest. Each
/// row is small; the cap exists to keep an unbounded query off the
/// hot path while still being generous enough to absorb a noisy
/// skill that gets thousands of votes in 14 days.
const MAX_FEEDBACK_ROWS: u32 = 1000;

/// Hard cap on how many `SkillRunRow`s we'll fetch when resolving
/// "which action_ids belong to this skill in the window". Mirrors
/// `MAX_FEEDBACK_ROWS`.
const MAX_SKILL_RUNS: usize = 1000;

/// Expected outcome for an eval case derived from an operator vote.
///
/// * `Up` → `Pass` (operator approved → loss-function target = good).
/// * `Down` → `Fail` (operator rejected → loss-function target = bad).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedOutcome {
    /// Operator approved the action — candidate should reproduce.
    Pass,
    /// Operator rejected the action — candidate should NOT reproduce.
    Fail,
}

impl ExpectedOutcome {
    /// Map an `argo-store::feedback::Vote` to the expected outcome.
    #[must_use]
    pub fn from_vote(vote: Vote) -> Self {
        match vote {
            Vote::Up => Self::Pass,
            Vote::Down => Self::Fail,
        }
    }
}

/// One operator-derived eval case. Folded into the suite alongside
/// synthesised cases, but carries `FEEDBACK_CASE_WEIGHT` in the
/// loss aggregation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackEvalCase {
    /// Action this vote was cast against. Used by the worker
    /// plugin to look up the action summary / context when
    /// scoring a candidate.
    pub invocation_id: String,
    /// Short human-readable label for the action — currently the
    /// operator's free-form comment if present, else the empty
    /// string. The worker plugin can resolve fuller context from
    /// the `worker_action` memory body via `invocation_id`.
    pub action_summary: String,
    /// `Pass` for Up votes, `Fail` for Down votes.
    pub expected: ExpectedOutcome,
    /// Provenance tag — always `"operator_feedback"` for cases
    /// produced by this module. Stable string so downstream JSON
    /// consumers can filter cases by origin.
    pub source: String,
}

impl FeedbackEvalCase {
    /// Convenience constructor used by [`ingest_operator_feedback`].
    #[must_use]
    pub fn new(invocation_id: String, action_summary: String, expected: ExpectedOutcome) -> Self {
        Self {
            invocation_id,
            action_summary,
            expected,
            source: "operator_feedback".to_string(),
        }
    }
}

/// Pull operator votes from `argo-store::feedback` for `skill_name`
/// within `window` and map each to a [`FeedbackEvalCase`].
///
/// Returns an empty vec (not an error) when:
/// * `skill_flag.no-forge-feedback = 1` for `skill_name`, or
/// * no votes landed within the window, or
/// * no action runs of this skill were recorded in the window.
///
/// The `tracing::info!` line surfaces empty-vec outcomes so
/// operators can tell "ingestion ran, no votes" apart from
/// "ingestion skipped entirely".
///
/// # Examples
///
/// ```no_run
/// # use std::time::Duration;
/// # async fn demo(store: &argo_store::SessionStore) -> argo_types::ArgoResult<()> {
/// let cases = argo_forge::feedback_eval::ingest_operator_feedback(
///     store, "billing-resolver", Duration::from_secs(14 * 24 * 3600),
/// ).await?;
/// // `cases` is empty when there's nothing to ingest — graceful.
/// # let _ = cases; Ok(()) }
/// ```
pub async fn ingest_operator_feedback(
    store: &SessionStore,
    skill_name: &str,
    window: Duration,
) -> ArgoResult<Vec<FeedbackEvalCase>> {
    let pool = store.pool();
    if get_flag_or_default(pool, skill_name, FLAG_NO_FORGE_FEEDBACK, 0).await? == 1 {
        tracing::info!(
            target: "argo_forge::feedback_eval",
            skill = skill_name,
            "skipping operator feedback ingestion (no-forge-feedback flag set)"
        );
        return Ok(Vec::new());
    }

    let since_micros = since_micros_from_window(window);
    let runs =
        runs_for_skill(store, skill_name, None, Some(since_micros), MAX_SKILL_RUNS, true).await?;
    if runs.is_empty() {
        tracing::info!(
            target: "argo_forge::feedback_eval",
            skill = skill_name,
            "skipping operator feedback ingestion (no action runs in window)"
        );
        return Ok(Vec::new());
    }
    let allowed_actions: HashSet<&str> =
        runs.iter().map(|row| row.action_id.as_str()).collect();

    let feedback_rows = list_feedback(pool, None, None, MAX_FEEDBACK_ROWS)
        .await
        .map_err(|e| ArgoError::Provider(format!("feedback_eval list_feedback: {e}")))?;

    let mut cases = Vec::new();
    for row in feedback_rows {
        if row.given_at_micros < since_micros {
            continue;
        }
        if !allowed_actions.contains(row.action_id.as_str()) {
            continue;
        }
        cases.push(FeedbackEvalCase::new(
            row.action_id,
            row.comment,
            ExpectedOutcome::from_vote(row.vote),
        ));
    }

    if cases.is_empty() {
        tracing::info!(
            target: "argo_forge::feedback_eval",
            skill = skill_name,
            "no operator votes within window — proceeding with synth-only cases"
        );
    } else {
        tracing::info!(
            target: "argo_forge::feedback_eval",
            skill = skill_name,
            cases = cases.len(),
            "ingested operator feedback cases"
        );
    }
    Ok(cases)
}

/// Aggregate a candidate's per-case scores into a single weighted
/// loss. Operator cases (`source == "operator_feedback"`) carry
/// `FEEDBACK_CASE_WEIGHT`× the weight of every other case. Returns
/// `(weighted_score, total_weight)` so callers can either normalize
/// (divide) or report raw — the worker plugin folds this into the
/// existing per-iteration score the GEPA loop emits.
///
/// Pure helper, no I/O.
///
/// # Examples
///
/// ```
/// use argo_forge::feedback_eval::{weighted_loss_score, ScoredCase, CaseKind};
/// let scores = [
///     ScoredCase { kind: CaseKind::Synth, score: 1.0 },
///     ScoredCase { kind: CaseKind::OperatorFeedback, score: 0.0 },
/// ];
/// let (w_score, total_weight) = weighted_loss_score(&scores);
/// // Synth (weight 1.0) → 1.0; operator-feedback (weight 1.5) → 0.0.
/// // Sum = 1.0; total weight = 2.5 → normalized = 0.4.
/// assert!((w_score - 1.0).abs() < 1e-9);
/// assert!((total_weight - 2.5).abs() < 1e-9);
/// ```
#[must_use]
pub fn weighted_loss_score(scores: &[ScoredCase]) -> (f32, f32) {
    let mut acc = 0.0_f32;
    let mut total_weight = 0.0_f32;
    for case in scores {
        let weight = case.weight();
        acc += weight * case.score;
        total_weight += weight;
    }
    (acc, total_weight)
}

/// Provenance discriminant for [`ScoredCase`] used by
/// [`weighted_loss_score`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseKind {
    /// Synthesised eval case (existing path).
    Synth,
    /// Operator-derived case from [`FeedbackEvalCase`].
    OperatorFeedback,
}

/// One scored case fed into [`weighted_loss_score`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScoredCase {
    /// Where the case came from.
    pub kind: CaseKind,
    /// Per-case score in `[0, 1]` (1.0 = candidate matched expected).
    pub score: f32,
}

impl ScoredCase {
    /// Weight multiplier: operator cases get
    /// [`FEEDBACK_CASE_WEIGHT`]; synth cases get `1.0`.
    #[must_use]
    pub fn weight(self) -> f32 {
        match self.kind {
            CaseKind::Synth => 1.0,
            CaseKind::OperatorFeedback => FEEDBACK_CASE_WEIGHT,
        }
    }
}

fn since_micros_from_window(window: Duration) -> i64 {
    let now = Utc::now().timestamp_micros();
    let window_micros = i64::try_from(window.as_micros()).unwrap_or(i64::MAX);
    now.saturating_sub(window_micros)
}

#[cfg(test)]
#[path = "feedback_eval_tests.rs"]
mod tests;
