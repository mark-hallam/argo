//! Scheduled re-evaluation of active skills with declared eval suites.
//!
//! This module owns the cron-safe tick body. It deliberately accepts an
//! [`EvalExecutor`] so production callers can supply a real runtime
//! executor while the crate remains free of agent-runner/provider
//! dependencies. The zero-argument helper uses [`InputEchoEvalExecutor`],
//! a deterministic fallback useful for CLI smoke checks and fixtures
//! whose eval input already is the observed output.

use crate::eval_runner::{run_evals, ActualOutcome, EvalExecutor};
use crate::skill::Skill;
use crate::store::{RawSkill, SkillStore};
use argo_types::{ArgoError, ArgoResult, SkillId};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

const AUDIT_KIND: &str = "skill.regression_detected";
const ACTION_TYPE: &str = "SKILL_REEVAL";
const ENSURE_SQL: &str = include_str!("../../argo-store/migrations/121__skill_reeval_history.sql");

/// Tunables for one scheduled re-eval tick.
#[derive(Debug, Clone)]
pub struct ScheduledReevalConfig {
    /// Absolute pass-rate drop that counts as a regression.
    pub regression_threshold: f32,
}

impl Default for ScheduledReevalConfig {
    fn default() -> Self {
        Self {
            regression_threshold: env_f32("ARGO_SKILL_REEVAL_REGRESSION_THRESHOLD", 0.10),
        }
    }
}

/// Summary returned by a scheduled re-eval tick.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReevalReport {
    /// Active skills with non-empty eval suites that were executed.
    pub evaluated: usize,
    /// Evaluated skills whose pass-rate dropped beyond the threshold.
    pub regressed: usize,
    /// Regressed skills successfully moved to `lifecycle_state = 'flagged'`.
    pub flagged: usize,
    /// Active skills skipped because their frontmatter has no eval cases.
    pub skipped_no_eval: usize,
}

/// Deterministic fallback executor that returns each eval case's
/// `input` as the actual outcome.
pub struct InputEchoEvalExecutor;

#[async_trait]
impl EvalExecutor for InputEchoEvalExecutor {
    async fn run_one(&self, _skill: &Skill, case: &crate::EvalCase) -> ArgoResult<ActualOutcome> {
        Ok(ActualOutcome::new(case.input.clone()))
    }
}

/// Run one scheduled re-eval tick with the deterministic fallback
/// executor and environment/default config.
pub async fn scheduled_reeval_tick(skills: &SkillStore) -> ArgoResult<ReevalReport> {
    scheduled_reeval_tick_with_config(
        skills,
        &InputEchoEvalExecutor,
        ScheduledReevalConfig::default(),
    )
    .await
}

/// Run one scheduled re-eval tick with explicit executor and config.
pub async fn scheduled_reeval_tick_with_config(
    skills: &SkillStore,
    executor: &dyn EvalExecutor,
    config: ScheduledReevalConfig,
) -> ArgoResult<ReevalReport> {
    ensure_schema(skills).await?;
    let targets = active_skills(skills).await?;
    let mut report = ReevalReport::default();
    for skill in targets {
        evaluate_one(skills, executor, &config, &skill, &mut report).await?;
    }
    Ok(report)
}

/// Re-evaluate one active skill by id with the deterministic fallback
/// executor.
pub async fn scheduled_reeval_skill(
    skills: &SkillStore,
    skill_id: SkillId,
) -> ArgoResult<ReevalReport> {
    scheduled_reeval_skill_with_config(
        skills,
        skill_id,
        &InputEchoEvalExecutor,
        ScheduledReevalConfig::default(),
    )
    .await
}

/// Re-evaluate one active skill by id with explicit executor and config.
pub async fn scheduled_reeval_skill_with_config(
    skills: &SkillStore,
    skill_id: SkillId,
    executor: &dyn EvalExecutor,
    config: ScheduledReevalConfig,
) -> ArgoResult<ReevalReport> {
    ensure_schema(skills).await?;
    let skill = active_skill_by_id(skills, skill_id).await?;
    let mut report = ReevalReport::default();
    evaluate_one(skills, executor, &config, &skill, &mut report).await?;
    Ok(report)
}

async fn evaluate_one(
    skills: &SkillStore,
    executor: &dyn EvalExecutor,
    config: &ScheduledReevalConfig,
    skill: &Skill,
    report: &mut ReevalReport,
) -> ArgoResult<()> {
    if skill.frontmatter.evals.is_empty() {
        report.skipped_no_eval += 1;
        return Ok(());
    }
    report.evaluated += 1;
    let previous = previous_pass_rate(skills, skill).await?;
    let eval_run_id = Uuid::now_v7().to_string();
    let run_at = Utc::now().timestamp_micros();
    let pass_rate = match run_evals(skill, executor, &skill.frontmatter.evals).await {
        Ok(eval) => Some(f64::from(eval.rate)),
        Err(err) => {
            tracing::warn!(
                target: "argo_skill::scheduled_reeval",
                skill_id = %skill.id,
                error = %err,
                "skill re-eval failed; recording null pass_rate"
            );
            None
        }
    };
    let regressed = previous
        .zip(pass_rate)
        .is_some_and(|(prev, current)| prev - current > f64::from(config.regression_threshold));
    insert_history(
        skills,
        skill,
        &eval_run_id,
        run_at,
        pass_rate,
        previous,
        regressed,
    )
    .await?;
    if regressed {
        report.regressed += 1;
        if flag_skill(skills, skill).await? > 0 {
            report.flagged += 1;
        }
        emit_regression_audit(skills, skill, &eval_run_id, previous, pass_rate).await?;
    }
    Ok(())
}

async fn ensure_schema(skills: &SkillStore) -> ArgoResult<()> {
    let stripped = ENSURE_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(skills.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("skill_reeval_history ensure: {e}")))?;
    }
    Ok(())
}

async fn active_skills(skills: &SkillStore) -> ArgoResult<Vec<Skill>> {
    let rows = sqlx::query_as::<_, RawSkill>(
        "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
                s.learned_in, s.superseded_by, s.created_at, l.lock, \
                s.vertical_id, s.lifecycle_state \
         FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
         WHERE s.superseded_by IS NULL AND s.lifecycle_state = 'active' \
         ORDER BY s.name ASC",
    )
    .fetch_all(skills.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("scheduled_reeval active skills: {e}")))?;
    rows.into_iter().map(Skill::try_from).collect()
}

async fn active_skill_by_id(skills: &SkillStore, skill_id: SkillId) -> ArgoResult<Skill> {
    let row: Option<RawSkill> = sqlx::query_as::<_, RawSkill>(
        "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
                s.learned_in, s.superseded_by, s.created_at, l.lock, \
                s.vertical_id, s.lifecycle_state \
         FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
         WHERE s.id = ? AND s.superseded_by IS NULL AND s.lifecycle_state = 'active'",
    )
    .bind(skill_id.0.as_bytes().as_slice())
    .fetch_optional(skills.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("scheduled_reeval skill lookup: {e}")))?;
    row.ok_or_else(|| ArgoError::Provider(format!("no active skill '{skill_id}'")))
        .and_then(Skill::try_from)
}

async fn previous_pass_rate(skills: &SkillStore, skill: &Skill) -> ArgoResult<Option<f64>> {
    sqlx::query_scalar(
        "SELECT pass_rate FROM skill_reeval_history \
         WHERE skill_id = ? AND pass_rate IS NOT NULL \
         ORDER BY run_at DESC LIMIT 1",
    )
    .bind(skill.id.to_string())
    .fetch_optional(skills.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_reeval previous: {e}")))
}

async fn insert_history(
    skills: &SkillStore,
    skill: &Skill,
    eval_run_id: &str,
    run_at: i64,
    pass_rate: Option<f64>,
    previous_pass_rate: Option<f64>,
    regressed: bool,
) -> ArgoResult<()> {
    sqlx::query(
        "INSERT INTO skill_reeval_history \
         (eval_run_id, skill_id, skill_name, skill_version, run_at, \
          pass_rate, previous_pass_rate, regressed) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(eval_run_id)
    .bind(skill.id.to_string())
    .bind(&skill.frontmatter.name)
    .bind(&skill.frontmatter.version)
    .bind(run_at)
    .bind(pass_rate)
    .bind(previous_pass_rate)
    .bind(if regressed { 1_i64 } else { 0_i64 })
    .execute(skills.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_reeval history insert: {e}")))?;
    Ok(())
}

async fn flag_skill(skills: &SkillStore, skill: &Skill) -> ArgoResult<u64> {
    let res = sqlx::query("UPDATE skill SET lifecycle_state = 'flagged' WHERE id = ?")
        .bind(skill.id.0.as_bytes().as_slice())
        .execute(skills.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill_reeval flag skill: {e}")))?;
    Ok(res.rows_affected())
}

async fn emit_regression_audit(
    skills: &SkillStore,
    skill: &Skill,
    eval_run_id: &str,
    previous_pass_rate: Option<f64>,
    current_pass_rate: Option<f64>,
) -> ArgoResult<()> {
    let payload = serde_json::json!({
        "action_type": ACTION_TYPE,
        "outcome_status": "regressed",
        "skill_id": skill.id.to_string(),
        "name": skill.frontmatter.name,
        "previous_pass_rate": previous_pass_rate,
        "current_pass_rate": current_pass_rate,
        "eval_run_id": eval_run_id,
    });
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Schema(format!("skill_reeval audit json: {e}")))?;
    let parent = last_audit_hash(skills).await?;
    let self_hash = chain_hash(&parent, AUDIT_KIND, &canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at, principal_id, tenant_id, \
         worker_id, channel, action_type, outcome_status, \
         approver_id, cost_band, data_classification) \
         VALUES (?, NULL, ?, ?, ?, ?, ?, 'system', '__system__', \
         NULL, NULL, ?, ?, NULL, NULL, NULL)",
    )
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .bind(AUDIT_KIND)
    .bind(&canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .bind(Utc::now().timestamp_micros())
    .bind(ACTION_TYPE)
    .bind("regressed")
    .execute(skills.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_reeval audit insert: {e}")))?;
    Ok(())
}

async fn last_audit_hash(skills: &SkillStore) -> ArgoResult<[u8; 32]> {
    let row = sqlx::query("SELECT self_hash FROM audit_record ORDER BY seq DESC LIMIT 1")
        .fetch_optional(skills.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("skill_reeval audit hash: {e}")))?;
    let mut out = [0u8; 32];
    if let Some(row) = row {
        let bytes: Vec<u8> = row
            .try_get("self_hash")
            .map_err(|e| ArgoError::Provider(format!("skill_reeval audit hash decode: {e}")))?;
        if bytes.len() == 32 {
            out.copy_from_slice(&bytes);
        }
    }
    Ok(out)
}

fn chain_hash(parent: &[u8; 32], kind: &str, payload: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(parent);
    h.update(kind.as_bytes());
    h.update(b"\0");
    h.update(payload.as_bytes());
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}

fn env_f32(key: &str, fallback: f32) -> f32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| (0.0..=1.0).contains(v))
        .unwrap_or(fallback)
}

#[cfg(test)]
#[path = "scheduled_reeval_cron_tests.rs"]
mod tests;
