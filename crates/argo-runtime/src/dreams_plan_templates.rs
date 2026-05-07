//! Plan-template Dreams pipeline (V2.0 cross-pollination,
//! `claude-dreams-plan-templates`).
//!
//! Today the Planner trait emits one decomposition per call.
//! Quality varies — operators see similar coarse goals get split
//! into differently-shaped DAGs run after run. The Dreams
//! pipeline groups N similar parent-goal decompositions into a
//! cohort and asks the Dreams backend to produce a polished
//! plan template (workflow YAML + reasoning) — the resulting
//! [`LearnedPlanTemplate`] is candidate-stamped `Pending` and
//! enters the TrustGrade ladder. Operators promote good ones via
//! `argo plan template promote`; the [`super::TemplatePlanner`]
//! consults promoted templates before falling through to
//! [`super::LlmPlanner`].
//!
//! Mechanism mirrors `argo-skill::dreams_distillation`:
//!  - bag-of-tokens cosine similarity over `parent_goal` strings
//!  - anchor clustering with [`MIN_PLAN_COHORT_SIZE`] floor
//!  - mechanical-fallback boundary: cohorts under the floor
//!    are dropped and the legacy in-memory templates remain
//!  - HTTP transport + persistence live in the argo-cli layer
//!    so this crate stays free of database deps
//!
//! ```ignore
//! use argo_runtime::dreams_plan_templates::{
//!     distill_plan_cohort, group_into_plan_cohorts,
//! };
//! let cohorts = group_into_plan_cohorts(history);
//! for cohort in cohorts {
//!     let learned = distill_plan_cohort(&cohort, &caller).await?;
//!     // persist via argo_store::learned_plan_template::insert
//! }
//! ```
use crate::PlannerError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minimum cohort size that triggers a Dreams call. Below this,
/// the legacy mechanical TemplatePlanner shipped-defaults win.
pub const MIN_PLAN_COHORT_SIZE: usize = 3;

/// Cosine-similarity threshold for cohort membership. Members
/// must score `>=` this against the anchor parent_goal.
pub const PLAN_COHORT_SIMILARITY_THRESHOLD: f64 = 0.7;

/// Marker stored alongside a [`LearnedPlanTemplate`] indicating
/// the body came from the Dreams pipeline.
pub const BODY_SOURCE_DREAMS: &str = "dreams_distilled";

/// One historical decomposition fed into a cohort. The runtime
/// reads these from [`super::SubtaskSink`] tail rows (typically
/// the argo-store `subtask_ledger`); the layer that walks the
/// store is in argo-cli to keep this module persistence-free.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanCohortInput {
    /// The coarse goal the Planner decomposed.
    pub parent_goal: String,
    /// Optional Session id that owned the decomposition.
    pub parent_session_id: Option<String>,
    /// The workflow YAML the Planner emitted (or a serialised
    /// representation thereof). Members of a cohort share a
    /// shape; the Dreams call distils a polished body across
    /// them.
    pub workflow_yaml: String,
    /// Whether the decomposition completed without failures.
    /// Failed runs are excluded by [`group_into_plan_cohorts`].
    pub success: bool,
}

/// Group of similar successful decompositions that warrant
/// distillation. Built by [`group_into_plan_cohorts`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanCohort {
    /// Anchor parent goal — first input in the cohort. Used as
    /// the row's `parent_goal` field when persisted.
    pub anchor_parent_goal: String,
    /// Members of the cohort. `members.len() >= MIN_PLAN_COHORT_SIZE`
    /// is invariantly true for cohorts emitted by
    /// [`group_into_plan_cohorts`].
    pub members: Vec<PlanCohortInput>,
}

/// Result of one Dreams plan-template distillation call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanDistillationResult {
    /// Stable name suggested by Dreams. The CLI may override.
    pub name: String,
    /// One-line description shown to operators.
    pub description: String,
    /// Lower-case keywords that should match the template.
    pub keywords: Vec<String>,
    /// The polished workflow YAML.
    pub yaml: String,
    /// Model identifier used.
    pub model: String,
    /// Approximate cost in USD micros — feeds the caller-site
    /// daily cap.
    pub cost_micros: i64,
}

/// Pluggable Dreams caller for plan-template distillation.
/// Production uses an HTTP impl in argo-cli; tests use stubs.
#[async_trait]
pub trait PlanDistillationCaller: Send + Sync {
    /// Submit a cohort for distillation.
    ///
    /// # Errors
    /// Implementations return [`PlannerError::Llm`] for
    /// transport / decode / API failures.
    async fn distill(&self, cohort: &PlanCohort) -> Result<PlanDistillationResult, PlannerError>;
}

/// What a [`PlanDistillationCaller`] hands back to the caller
/// after a successful Dreams call. The argo-cli persistence
/// layer turns this into the row payload for
/// `argo_store::learned_plan_template::insert`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnedPlanTemplate {
    /// Stable name used in TemplatePlanner reasoning + CLI
    /// display.
    pub name: String,
    /// One-line description shown to operators.
    pub description: String,
    /// Anchor parent goal — provenance for audit.
    pub parent_goal: String,
    /// Lower-case keywords the planner matches against.
    pub keywords: Vec<String>,
    /// Workflow YAML the Dreams call returned.
    pub yaml: String,
    /// `body_source` marker — always [`BODY_SOURCE_DREAMS`].
    pub body_source: String,
    /// Cohort size at distillation time.
    pub cohort_size: usize,
}

/// Group historical decompositions by similarity over the
/// `parent_goal` string. Failures are dropped first; remaining
/// rows are clustered around the first input by bag-of-tokens
/// cosine similarity. Cohorts smaller than
/// [`MIN_PLAN_COHORT_SIZE`] are discarded.
#[must_use]
pub fn group_into_plan_cohorts(input: Vec<PlanCohortInput>) -> Vec<PlanCohort> {
    let successes: Vec<PlanCohortInput> = input.into_iter().filter(|m| m.success).collect();
    if successes.len() < MIN_PLAN_COHORT_SIZE {
        return Vec::new();
    }
    // Single-pass anchor clustering — keeps members that cluster
    // around the first row's parent_goal. Multi-anchor passes can
    // land later if the miss rate proves problematic.
    let anchor_tokens = goal_tokens(&successes[0].parent_goal);
    let anchor_goal = successes[0].parent_goal.clone();
    let cluster: Vec<PlanCohortInput> = successes
        .into_iter()
        .filter(|m| {
            cosine_similarity(&anchor_tokens, &goal_tokens(&m.parent_goal))
                >= PLAN_COHORT_SIMILARITY_THRESHOLD
        })
        .collect();
    if cluster.len() < MIN_PLAN_COHORT_SIZE {
        return Vec::new();
    }
    vec![PlanCohort {
        anchor_parent_goal: anchor_goal,
        members: cluster,
    }]
}

/// Bag-of-tokens cosine similarity over two token vectors.
/// Each token is weighted by occurrence count. Returns 0.0 when
/// either side is empty.
#[must_use]
pub fn cosine_similarity(a: &[String], b: &[String]) -> f64 {
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

/// Lower-case alphanumeric tokens of a free-text goal. Stop
/// punctuation; collapse whitespace; drop empties. Used both by
/// the cohort grouper and by callers that want to deduplicate
/// overlapping cohort anchors.
#[must_use]
pub fn goal_tokens(goal: &str) -> Vec<String> {
    goal.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Distill one cohort into a [`LearnedPlanTemplate`] via the
/// Dreams backend.
///
/// Returns the candidate (with `body_source` set to
/// [`BODY_SOURCE_DREAMS`]) and the raw [`PlanDistillationResult`].
/// The caller is responsible for daily-cap accounting and
/// persistence via `argo_store::learned_plan_template::insert`.
///
/// # Errors
/// - [`PlannerError::Llm`] when `cohort.members.len() < MIN_PLAN_COHORT_SIZE`.
/// - Propagates [`PlanDistillationCaller::distill`] errors.
pub async fn distill_plan_cohort(
    cohort: &PlanCohort,
    caller: &dyn PlanDistillationCaller,
) -> Result<(LearnedPlanTemplate, PlanDistillationResult), PlannerError> {
    if cohort.members.len() < MIN_PLAN_COHORT_SIZE {
        return Err(PlannerError::Llm(format!(
            "plan cohort '{}' too small (size {} < min {})",
            cohort.anchor_parent_goal,
            cohort.members.len(),
            MIN_PLAN_COHORT_SIZE
        )));
    }
    let result = caller.distill(cohort).await?;
    let template = LearnedPlanTemplate {
        name: result.name.clone(),
        description: result.description.clone(),
        parent_goal: cohort.anchor_parent_goal.clone(),
        keywords: result.keywords.clone(),
        yaml: result.yaml.clone(),
        body_source: BODY_SOURCE_DREAMS.into(),
        cohort_size: cohort.members.len(),
    };
    Ok((template, result))
}

#[cfg(test)]
#[path = "dreams_plan_templates_tests.rs"]
mod tests;
