//! Self-improvement loop closure — bridges the two halves that were
//! built separately so the full cycle runs end to end:
//!
//! ```text
//! traces ─cluster_traces─▶ TraceCluster ──[this module]──▶ Cohort
//!        ─distill_and_propose_judged─▶ proposal (review queue)
//!        ─run_ab_eval─▶ SkillAbEvalReport ──[this module]──▶ PromotionDecision
//! ```
//!
//! Two pure seams live here:
//!
//! 1. [`cluster_to_cohort`] — turn a cross-`(vertical,action)`-key
//!    [`TraceCluster`] (from [`crate::trace_cluster`]) into a
//!    [`Cohort`] the **existing** distillation→proposal pipeline
//!    ([`crate::dreams_distillation::distill_and_propose_judged`])
//!    accepts. A `TraceCluster` holds only member *ids* (it clustered by
//!    shape, discarding per-member context), so the bridge re-joins each
//!    member's full [`TraceMember`] context by id — it cannot fabricate
//!    a cohort from a cluster alone, by construction.
//! 2. [`decide_promotion`] — the **promotion-on-performance gate**: a
//!    pure decision over a measured [`SkillAbEvalReport`] (from
//!    [`crate::ab_eval::run_ab_eval`]) that returns
//!    [`PromotionDecision::Promote`] / `Hold` / `Reject`. This is the
//!    *decision*; `ab_test::promote_with_regression_gate` is the
//!    DB-level *application* — keeping the policy pure makes it
//!    deterministic + unit-testable without a store.
//!
//! No model, no DB, no IO — deterministic, CI-safe. The cron/CLI
//! consumer that wires real traces + a real Dreams caller + the store is
//! a thin orchestration layer above this (and lives outside the
//! Windows-buildable library lane).
use crate::dreams_distillation::{Cohort, TrajectoryInput, MIN_COHORT_SIZE};
use crate::trace_cluster::TraceCluster;
use serde::{Deserialize, Serialize};

/// Full per-trace context needed to rebuild a [`Cohort`] member. A
/// [`TraceCluster`] only carries member ids; the loop driver holds the
/// original fingerprints + their `(vertical, action, trajectory)`
/// context and supplies them here so the bridge can re-join by id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceMember {
    /// Trace id — matches a [`TraceCluster::member_ids`] entry and the
    /// originating session.
    pub id: String,
    /// Vertical the trace ran under.
    pub vertical_id: String,
    /// Action kind the trace performed.
    pub action_type: String,
    /// Ordered action/step names (the cohort body).
    pub trajectory: Vec<String>,
    /// Final captured text (URL / confirmation / …), if any.
    pub final_text: Option<String>,
    /// Success metric of the trace, 0.0–1.0.
    pub success_metric: f64,
}

/// Why a [`TraceCluster`] could not be bridged into a [`Cohort`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeError {
    /// Fewer than [`MIN_COHORT_SIZE`] members resolved to a known
    /// [`TraceMember`] — too small to distil (carries the resolved
    /// count).
    TooSmall(usize),
    /// A member id in the cluster had no matching [`TraceMember`] in the
    /// supplied context (carries the missing id).
    UnknownMember(String),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooSmall(n) => write!(
                f,
                "trace cluster bridged to {n} members; need >= {MIN_COHORT_SIZE} to distil"
            ),
            Self::UnknownMember(id) => {
                write!(f, "cluster member id '{id}' has no TraceMember context")
            }
        }
    }
}

impl std::error::Error for BridgeError {}

/// Bridge a cross-key [`TraceCluster`] into a [`Cohort`] the existing
/// distillation pipeline accepts.
///
/// `members_by_id` resolves each [`TraceCluster::member_ids`] entry to its
/// full [`TraceMember`]. Because a cross-key cluster can span multiple
/// `(vertical, action)` pairs, the resulting cohort must be stamped with a
/// single `(vertical_id, action_type)`: the caller passes the canonical
/// label to use (typically the cluster's most-common pair, or an operator
/// choice). Members keep their own original vertical/action inside the
/// rebuilt [`TrajectoryInput`] only via the trajectory; the cohort header
/// carries the canonical pair so the distilled skill installs somewhere.
///
/// # Errors
/// - [`BridgeError::UnknownMember`] if any member id is absent from
///   `members_by_id`.
/// - [`BridgeError::TooSmall`] if fewer than [`MIN_COHORT_SIZE`] members
///   resolve (a cohort that can't distil).
pub fn cluster_to_cohort(
    cluster: &TraceCluster,
    members_by_id: &[TraceMember],
    canonical_vertical_id: &str,
    canonical_action_type: &str,
) -> Result<Cohort, BridgeError> {
    let mut members = Vec::with_capacity(cluster.member_ids.len());
    for id in &cluster.member_ids {
        let m = members_by_id
            .iter()
            .find(|m| &m.id == id)
            .ok_or_else(|| BridgeError::UnknownMember(id.clone()))?;
        members.push(TrajectoryInput {
            session_id: m.id.clone(),
            action_type: canonical_action_type.to_string(),
            trajectory: m.trajectory.clone(),
            final_text: m.final_text.clone(),
            success_metric: m.success_metric,
            vertical_id: canonical_vertical_id.to_string(),
        });
    }
    if members.len() < MIN_COHORT_SIZE {
        return Err(BridgeError::TooSmall(members.len()));
    }
    Ok(Cohort {
        action_type: canonical_action_type.to_string(),
        vertical_id: canonical_vertical_id.to_string(),
        members,
    })
}

/// Thresholds for the promotion-on-performance gate. A candidate skill
/// version is promoted over its baseline only when its measured eval
/// pass-rate improves by at least `min_delta` AND clears
/// `min_candidate_rate` in absolute terms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PromotionPolicy {
    /// Minimum pass-rate improvement (candidate − baseline) to promote.
    /// Default `0.05` (5 percentage points) — below this the change is
    /// noise, not a win.
    pub min_delta: f32,
    /// Minimum absolute candidate pass-rate to promote even with a
    /// positive delta — never promote a still-bad skill just because the
    /// baseline was worse. Default `0.5`.
    pub min_candidate_rate: f32,
    /// Pass-rate REGRESSION (baseline − candidate) at or beyond which the
    /// candidate is actively rejected (not merely held). Default `0.05`.
    pub max_regression: f32,
}

impl Default for PromotionPolicy {
    fn default() -> Self {
        Self {
            min_delta: 0.05,
            min_candidate_rate: 0.5,
            max_regression: 0.05,
        }
    }
}

/// Outcome of the promotion-on-performance gate over a measured A/B-eval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum PromotionDecision {
    /// Candidate cleared both the delta and absolute-rate bars — promote.
    Promote {
        /// Human-readable justification (the measured numbers).
        reason: String,
    },
    /// Candidate neither improved enough nor regressed enough — keep the
    /// baseline active, do not promote (no harm, just not a win yet).
    Hold {
        /// Why it was held.
        reason: String,
    },
    /// Candidate actively regressed past `max_regression` — reject so the
    /// loser can be superseded / pruned.
    Reject {
        /// Why it was rejected.
        reason: String,
    },
}

impl PromotionDecision {
    /// `true` only for [`PromotionDecision::Promote`].
    #[must_use]
    pub fn is_promote(&self) -> bool {
        matches!(self, Self::Promote { .. })
    }
}

/// Decide whether a measured A/B-eval justifies promoting the candidate
/// over the baseline, under `policy`. Pure + deterministic — the
/// numbers fully determine the verdict, so it is unit-testable without a
/// store and safe to call in CI.
///
/// Precedence: a regression past `max_regression` is **Reject** (checked
/// first, so a candidate that both improved-vs-floor and regressed can't
/// sneak through); otherwise Promote when the delta + absolute-rate bars
/// are met; otherwise Hold.
///
/// # Examples
///
/// ```
/// use argo_skill::ab_eval::{SkillAbEvalReport, VariantEvalResult};
/// use argo_skill::self_improve_loop::{decide_promotion, PromotionPolicy};
///
/// let report = SkillAbEvalReport {
///     skill_name: "refund-helper".into(),
///     case_count: 10,
///     baseline: VariantEvalResult { version: "v1".into(), pass: 5, fail: 5, pass_rate: 0.5 },
///     candidate: VariantEvalResult { version: "v2".into(), pass: 9, fail: 1, pass_rate: 0.9 },
///     pass_rate_delta: 0.4,
///     improved: true,
/// };
/// assert!(decide_promotion(&report, &PromotionPolicy::default()).is_promote());
/// ```
#[must_use]
pub fn decide_promotion(
    report: &crate::ab_eval::SkillAbEvalReport,
    policy: &PromotionPolicy,
) -> PromotionDecision {
    let delta = report.pass_rate_delta;
    let cand = report.candidate.pass_rate;
    let base = report.baseline.pass_rate;
    // Reject first: an actual regression past the floor.
    if base - cand >= policy.max_regression {
        return PromotionDecision::Reject {
            reason: format!(
                "candidate {} regressed {:.0}pp vs baseline {} ({:.2} -> {:.2}); >= {:.0}pp reject floor",
                report.candidate.version,
                (base - cand) * 100.0,
                report.baseline.version,
                base,
                cand,
                policy.max_regression * 100.0
            ),
        };
    }
    let delta_ok = delta >= policy.min_delta;
    let rate_ok = cand >= policy.min_candidate_rate;
    if delta_ok && rate_ok {
        PromotionDecision::Promote {
            reason: format!(
                "candidate {} improved {:.0}pp ({:.2} -> {:.2}) and clears the {:.0}% bar",
                report.candidate.version,
                delta * 100.0,
                base,
                cand,
                policy.min_candidate_rate * 100.0
            ),
        }
    } else {
        PromotionDecision::Hold {
            reason: format!(
                "candidate {} at {:.2} (delta {:+.0}pp): {}",
                report.candidate.version,
                cand,
                delta * 100.0,
                if !delta_ok {
                    format!("improvement below the {:.0}pp bar", policy.min_delta * 100.0)
                } else {
                    format!(
                        "absolute rate below the {:.0}% bar",
                        policy.min_candidate_rate * 100.0
                    )
                }
            ),
        }
    }
}

#[cfg(test)]
#[path = "self_improve_loop_tests.rs"]
mod tests;
