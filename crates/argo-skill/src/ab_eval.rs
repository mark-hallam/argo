//! A/B-by-eval — the closed self-improvement loop (DEC-023 / P4).
//!
//! The pieces beneath this module were already real but disconnected:
//! [`crate::eval_runner::run_evals`] genuinely runs a skill's declared
//! [`crate::EvalCase`]s through a pluggable [`EvalExecutor`] and produces
//! a measured pass-rate; [`crate::ab_test`] holds the statistics + the
//! `skill_ab_test_*` tables; [`crate::scheduled_reeval_cron`] writes
//! `skill_reeval_history`. What was missing was a single call that, given
//! TWO versions of one skill, RUNS BOTH through the real scorer and
//! reports a measured pass-rate delta — the "skill improved from X% → Y%"
//! proof.
//!
//! [`run_ab_eval`] closes that gap:
//!
//! 1. Load both versions of `skill_name` (any version string) from the
//!    `skill` table via [`crate::SkillStore::versions_for_name`].
//! 2. Run BOTH bodies through [`run_evals`] against the SAME eval set
//!    (version B's declared `evals`, falling back to A's). The executor
//!    is **body-aware**: it returns the skill body as the candidate
//!    output, so a `Contains`/`Equals`/`NonEmpty` predicate genuinely
//!    passes only when THAT body contains the required guidance. An older
//!    body that lacks a fix fails the case; an improved body that adds it
//!    passes. The delta is therefore a real measurement, not a rig.
//! 3. Return a [`SkillAbEvalReport`] with the two real pass rates + the
//!    signed delta + per-case outcomes for each variant.
//!
//! The executor is deterministic by default (no model, no network) so the
//! loop is reproducible in CI and the demo. A model-graded executor can be
//! supplied by the caller via [`run_ab_eval_with`] — the deterministic
//! [`BodyContainsExecutor`] is the default + the fallback, mirroring how
//! Genesis gates its LLM classifier behind a deterministic keyword path.

use crate::eval_runner::{run_evals, ActualOutcome, EvalExecutor};
use crate::skill::{EvalCase, Skill};
use crate::store::SkillStore;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Idempotent mirror of `crates/argo-store/migrations/121__skill_reeval_history.sql`.
/// The table is lazily applied by the reeval cron (it is NOT in the
/// migration registry), so a store that has never run a reeval tick would
/// otherwise hit "no such table" on [`record_candidate_reeval`].
const REEVAL_SCHEMA_SQL: &str =
    include_str!("../../argo-store/migrations/121__skill_reeval_history.sql");

/// Per-variant measured result inside an A/B-by-eval run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantEvalResult {
    /// Version string of this variant (e.g. `"v1"`).
    pub version: String,
    /// Cases that passed.
    pub pass: u32,
    /// Cases that failed (wrong output OR executor error).
    pub fail: u32,
    /// `pass / (pass + fail)`, in `[0.0, 1.0]`.
    pub pass_rate: f32,
}

/// Full A/B-by-eval comparison: two measured variants + the delta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillAbEvalReport {
    /// Skill family name both variants belong to.
    pub skill_name: String,
    /// Number of eval cases run against BOTH variants (the shared set).
    pub case_count: usize,
    /// Baseline (the "before" / old version).
    pub baseline: VariantEvalResult,
    /// Candidate (the "after" / new version).
    pub candidate: VariantEvalResult,
    /// `candidate.pass_rate - baseline.pass_rate`. Positive = improved.
    pub pass_rate_delta: f32,
    /// `true` when the candidate's pass rate strictly exceeds the
    /// baseline's — a real, measured improvement.
    pub improved: bool,
}

/// Deterministic, body-aware [`EvalExecutor`].
///
/// Returns the skill body as the candidate output. Paired with
/// `Contains` / `Equals` / `NonEmpty` predicates this makes the eval a
/// genuine test of the body's content: a body that lacks the required
/// guidance fails the case, a body that adds it passes. No model, no
/// network — fully reproducible.
#[derive(Debug, Default, Clone, Copy)]
pub struct BodyContainsExecutor;

#[async_trait]
impl EvalExecutor for BodyContainsExecutor {
    async fn run_one(&self, skill: &Skill, _case: &EvalCase) -> ArgoResult<ActualOutcome> {
        Ok(ActualOutcome::new(Value::String(skill.body.clone())))
    }
}

/// Run an A/B-by-eval comparison between two versions of one skill with
/// the deterministic [`BodyContainsExecutor`].
///
/// `baseline_version` / `candidate_version` are version strings that must
/// both exist as `skill` rows for `skill_name`.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when either version is missing, when
/// the two versions are the same string, or when neither version declares
/// any eval cases (there is nothing to measure).
pub async fn run_ab_eval(
    skills: &SkillStore,
    skill_name: &str,
    baseline_version: &str,
    candidate_version: &str,
) -> ArgoResult<SkillAbEvalReport> {
    run_ab_eval_with(
        skills,
        skill_name,
        baseline_version,
        candidate_version,
        &BodyContainsExecutor,
    )
    .await
}

/// Run an A/B-by-eval comparison with a caller-supplied executor (e.g. a
/// model-graded one). Every other code path is identical to
/// [`run_ab_eval`]; this is the seam the CLI's `--llm` flag wires.
///
/// # Errors
/// Same conditions as [`run_ab_eval`].
pub async fn run_ab_eval_with(
    skills: &SkillStore,
    skill_name: &str,
    baseline_version: &str,
    candidate_version: &str,
    executor: &dyn EvalExecutor,
) -> ArgoResult<SkillAbEvalReport> {
    if baseline_version == candidate_version {
        return Err(ArgoError::Provider(
            "ab-eval: baseline and candidate versions must differ".into(),
        ));
    }
    let versions = skills.versions_for_name(skill_name).await?;
    let baseline = find_version(&versions, skill_name, baseline_version)?;
    let candidate = find_version(&versions, skill_name, candidate_version)?;

    // The shared eval set: prefer the candidate's declared cases (the
    // contract the new version is held to), fall back to the baseline's.
    // Both bodies are scored against the SAME set so the delta is
    // apples-to-apples.
    let cases = shared_eval_set(&candidate, &baseline)?;

    let base_report = run_evals(&baseline, executor, &cases).await?;
    let cand_report = run_evals(&candidate, executor, &cases).await?;

    let pass_rate_delta = cand_report.rate - base_report.rate;
    Ok(SkillAbEvalReport {
        skill_name: skill_name.to_string(),
        case_count: cases.len(),
        baseline: VariantEvalResult {
            version: baseline_version.to_string(),
            pass: base_report.pass,
            fail: base_report.fail,
            pass_rate: base_report.rate,
        },
        candidate: VariantEvalResult {
            version: candidate_version.to_string(),
            pass: cand_report.pass,
            fail: cand_report.fail,
            pass_rate: cand_report.rate,
        },
        pass_rate_delta,
        improved: cand_report.rate > base_report.rate,
    })
}

fn find_version(versions: &[Skill], skill_name: &str, version: &str) -> ArgoResult<Skill> {
    versions
        .iter()
        .find(|s| s.frontmatter.version == version)
        .cloned()
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "ab-eval: skill '{skill_name}' has no version '{version}'"
            ))
        })
}

fn shared_eval_set(candidate: &Skill, baseline: &Skill) -> ArgoResult<Vec<EvalCase>> {
    if !candidate.frontmatter.evals.is_empty() {
        return Ok(candidate.frontmatter.evals.clone());
    }
    if !baseline.frontmatter.evals.is_empty() {
        return Ok(baseline.frontmatter.evals.clone());
    }
    Err(ArgoError::Provider(format!(
        "ab-eval: neither version of skill '{}' declares eval cases; \
         add an `evals:` block to the SKILL.md frontmatter to measure it",
        candidate.frontmatter.name
    )))
}

/// Persist the candidate's measured pass rate from a [`SkillAbEvalReport`]
/// into `skill_reeval_history`, keyed on the candidate version's
/// `skill.id`, with the baseline's rate as `previous_pass_rate`. This is
/// what makes the A/B result visible on the worker Evals tab
/// (`GET /api/v1/workers/<id>/evals` joins `worker_skill.skill_id` →
/// `skill_reeval_history.skill_id`), so any worker that has learned the
/// candidate skill sees the real before/after run.
///
/// Reuses the same lazily-applied `skill_reeval_history` schema + insert
/// shape the scheduled-reeval cron uses — no new table, no migration.
/// `regressed` is set when the candidate's rate trailed the baseline's.
/// Returns the `eval_run_id` of the inserted row.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when the candidate version can't be
/// resolved to a `skill.id`, or the schema/insert query fails.
pub async fn record_candidate_reeval(
    skills: &SkillStore,
    report: &SkillAbEvalReport,
) -> ArgoResult<String> {
    ensure_reeval_schema(skills).await?;
    let versions = skills.versions_for_name(&report.skill_name).await?;
    let candidate = find_version(&versions, &report.skill_name, &report.candidate.version)?;
    let eval_run_id = Uuid::now_v7().to_string();
    let run_at = Utc::now().timestamp_micros();
    let regressed = i64::from(report.candidate.pass_rate < report.baseline.pass_rate);
    sqlx::query(
        "INSERT INTO skill_reeval_history \
         (eval_run_id, skill_id, skill_name, skill_version, run_at, \
          pass_rate, previous_pass_rate, regressed) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&eval_run_id)
    .bind(candidate.id.to_string())
    .bind(&report.skill_name)
    .bind(&report.candidate.version)
    .bind(run_at)
    .bind(f64::from(report.candidate.pass_rate))
    .bind(f64::from(report.baseline.pass_rate))
    .bind(regressed)
    .execute(skills.store_arc().pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("ab-eval reeval-history insert: {e}")))?;
    Ok(eval_run_id)
}

async fn ensure_reeval_schema(skills: &SkillStore) -> ArgoResult<()> {
    let stripped = REEVAL_SCHEMA_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(skills.store_arc().pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("ab-eval reeval schema: {e}")))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "ab_eval_tests.rs"]
mod ab_eval_tests;
