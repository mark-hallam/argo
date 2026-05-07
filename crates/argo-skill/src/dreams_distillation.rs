//! Skill-distillation Dreams pipeline (V2.0 Integration B,
//! `claude-dreams-skill-distillation`).
//!
//! Today `maybe_propose` produces a SkillCandidate body
//! mechanically from a single CU trajectory. Quality varies —
//! some candidates are great, some are noisy. The Dreams
//! pipeline groups N similar trajectories into a cohort and
//! asks the Dreams backend to produce a polished body — the
//! resulting candidate is tagged
//! [`SkillCandidate::body_source`]`= Some("dreams_distilled")`.
//!
//! Mechanical fallback survives: cohorts smaller than
//! [`MIN_COHORT_SIZE`] are skipped by [`group_into_cohorts`],
//! so the existing [`SkillCandidate::from_cu_trajectory`] path
//! continues to fire on isolated successes.
//!
//! HTTP transport, daily-cap accounting, and persistence live
//! in argo-cli's `skill_distill_now` handler — this module
//! defines the trait + pure pipeline only, mirroring the
//! `argo-memory::dreams_backend` split (caller trait here,
//! `EnvDreamsCaller` HTTP impl in argo-cli for memory's
//! analogue handler).
//!
//! ```ignore
//! use argo_skill::dreams_distillation::{distill_cohort, group_into_cohorts};
//! let cohorts = group_into_cohorts(trajectories);
//! for cohort in cohorts {
//!     let (candidate, _) = distill_cohort(&cohort, &caller).await?;
//!     // persist via SkillStore::propose_for_review
//! }
//! ```

use crate::learn::SkillCandidate;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minimum cohort size that triggers a Dreams call. Below this,
/// the legacy mechanical body wins.
pub const MIN_COHORT_SIZE: usize = 3;

/// Cosine-similarity threshold for cohort membership. Members
/// must score `>=` this against the anchor trajectory.
pub const COHORT_SIMILARITY_THRESHOLD: f64 = 0.7;

/// Marker stored in [`SkillCandidate::body_source`] when the
/// Dreams pipeline produced the body.
pub const BODY_SOURCE_DREAMS: &str = "dreams_distilled";

/// One trajectory candidate fed into a cohort.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrajectoryInput {
    /// Session identifier. The first 8 chars become part of the
    /// distilled candidate's name.
    pub session_id: String,
    /// Action kind (e.g. `post_listing`, `escalate`). Cohort
    /// grouping happens within `(vertical_id, action_type)`.
    pub action_type: String,
    /// Step strings — bag-of-tokens cosine similarity input.
    pub trajectory: Vec<String>,
    /// Final captured text (URL, confirmation message, …).
    pub final_text: Option<String>,
    /// Threshold-clearing metric for the run.
    pub success_metric: f64,
    /// Vertical the run belongs to.
    pub vertical_id: String,
}

/// Group of similar trajectories that warrant distillation.
#[derive(Debug, Clone, PartialEq)]
pub struct Cohort {
    /// Action kind shared by every member.
    pub action_type: String,
    /// Vertical shared by every member.
    pub vertical_id: String,
    /// Members of the cohort. `members.len() >= MIN_COHORT_SIZE`
    /// is invariantly true for cohorts emitted by
    /// [`group_into_cohorts`].
    pub members: Vec<TrajectoryInput>,
}

/// Result of one Dreams distillation call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistillationResult {
    /// Polished SKILL.md body returned by Dreams.
    pub body: String,
    /// Model identifier used.
    pub model: String,
    /// Approximate cost in USD micros — feeds the caller-site
    /// daily cap.
    pub cost_micros: i64,
}

/// Pluggable Dreams caller for skill-cohort distillation.
/// Production uses an HTTP impl in argo-cli; tests use stubs.
#[async_trait]
pub trait SkillDistillationCaller: Send + Sync {
    /// Submit a cohort for distillation.
    ///
    /// # Errors
    /// Implementations return [`ArgoError::Provider`] for
    /// transport / decode / API failures.
    async fn distill(&self, cohort: &Cohort) -> ArgoResult<DistillationResult>;
}

/// Group trajectories by `(vertical_id, action_type)` first,
/// then keep clusters whose members are mutually similar to an
/// anchor (cosine sim ≥ [`COHORT_SIMILARITY_THRESHOLD`]).
///
/// Cohorts smaller than [`MIN_COHORT_SIZE`] are dropped — that's
/// the mechanical-fallback boundary; the legacy
/// [`SkillCandidate::from_cu_trajectory`] path remains the
/// proposal mechanism for those isolated trajectories.
#[must_use]
pub fn group_into_cohorts(input: Vec<TrajectoryInput>) -> Vec<Cohort> {
    let mut by_key: HashMap<(String, String), Vec<TrajectoryInput>> = HashMap::new();
    for t in input {
        by_key
            .entry((t.vertical_id.clone(), t.action_type.clone()))
            .or_default()
            .push(t);
    }
    let mut out = Vec::new();
    for ((vertical_id, action_type), members) in by_key {
        if members.len() < MIN_COHORT_SIZE {
            continue;
        }
        // Single-pass anchor clustering — keeps members that
        // cluster around the first trajectory. Good enough for a
        // nightly tick over a worker's recent successes; a more
        // sophisticated multi-anchor pass can land later if the
        // miss rate proves problematic.
        let anchor = members[0].trajectory.clone();
        let cluster: Vec<TrajectoryInput> = members
            .into_iter()
            .filter(|m| {
                trajectory_similarity(&anchor, &m.trajectory)
                    >= COHORT_SIMILARITY_THRESHOLD
            })
            .collect();
        if cluster.len() >= MIN_COHORT_SIZE {
            out.push(Cohort {
                action_type,
                vertical_id,
                members: cluster,
            });
        }
    }
    out
}

/// Bag-of-tokens cosine similarity over two trajectory step
/// sequences. Each step string is a token; weight = occurrence
/// count. Returns 0.0 when either side is empty.
#[must_use]
pub fn trajectory_similarity(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let mut va: HashMap<&String, f64> = HashMap::new();
    let mut vb: HashMap<&String, f64> = HashMap::new();
    for s in a {
        *va.entry(s).or_default() += 1.0;
    }
    for s in b {
        *vb.entry(s).or_default() += 1.0;
    }
    let mut dot = 0.0_f64;
    for (k, w) in &va {
        if let Some(w2) = vb.get(k) {
            dot += w * w2;
        }
    }
    let mag_a: f64 = va.values().map(|w| w * w).sum::<f64>().sqrt();
    let mag_b: f64 = vb.values().map(|w| w * w).sum::<f64>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    dot / (mag_a * mag_b)
}

/// Distill one cohort into a polished SkillCandidate via the
/// Dreams backend.
///
/// Returns the candidate (with `body_source` set to
/// [`BODY_SOURCE_DREAMS`]) and the raw [`DistillationResult`].
/// The caller is responsible for daily-cap accounting (charge
/// `result.cost_micros` after a successful call) and
/// persistence via
/// [`crate::SkillStore::propose_for_review`].
///
/// # Errors
/// - [`ArgoError::Provider`] when `cohort.members.len() < MIN_COHORT_SIZE`.
/// - Propagates [`SkillDistillationCaller::distill`] errors.
pub async fn distill_cohort(
    cohort: &Cohort,
    caller: &dyn SkillDistillationCaller,
) -> ArgoResult<(SkillCandidate, DistillationResult)> {
    if cohort.members.len() < MIN_COHORT_SIZE {
        return Err(ArgoError::Provider(format!(
            "cohort '{}' too small (size {} < min {})",
            cohort.action_type,
            cohort.members.len(),
            MIN_COHORT_SIZE
        )));
    }
    let result = caller.distill(cohort).await?;
    let anchor = &cohort.members[0];
    let session_short: String = anchor.session_id.chars().take(8).collect();
    let avg_metric: f64 = cohort
        .members
        .iter()
        .map(|m| m.success_metric)
        .sum::<f64>()
        / (cohort.members.len() as f64);
    let mut candidate = SkillCandidate::from_cu_trajectory(
        format!("{}_{}_dreams", cohort.action_type, session_short),
        &cohort.action_type,
        &cohort.vertical_id,
        anchor.trajectory.clone(),
        anchor.final_text.clone(),
        0,
        avg_metric,
    );
    candidate.body_source = Some(BODY_SOURCE_DREAMS.into());
    candidate.description = format!(
        "Distilled from a cohort of {n} successful `{kind}` runs (mean success {avg:.2})",
        n = cohort.members.len(),
        kind = cohort.action_type,
        avg = avg_metric
    );
    Ok((candidate, result))
}

#[cfg(test)]
#[path = "dreams_distillation_tests.rs"]
mod tests;
