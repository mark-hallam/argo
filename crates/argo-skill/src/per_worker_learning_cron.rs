//! Per-worker skill-rating learning cron.
//!
//! The tick consumes positive `skill_rating` aggregates, re-proposes
//! worker-scoped overlay candidates through the normal judged proposal
//! path, associates the candidate with `worker_skill`, and records a
//! hash-chained audit row for each proposal.

use crate::skill::Skill;
use crate::store::{RawSkill, SkillStore};
use argo_judge::Judge;
use argo_store::skill_rating::{learning_candidates_since, RatingSummary};
use argo_store::WorkerId;
use argo_types::{ArgoError, ArgoResult, SessionId, SkillId, TenantId};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const AUDIT_KIND: &str = "skill.candidate.promoted";
const LEARNING_ACTION: &str = "PER_WORKER_LEARNING";

/// Tunables for one per-worker learning cron tick.
#[derive(Debug, Clone)]
pub struct PerWorkerLearningConfig {
    /// Earliest `skill_rating.rated_at` timestamp to consider.
    pub since_micros: i64,
    /// Optional tenant filter for ratings that may influence this worker.
    pub tenant_id: Option<TenantId>,
    /// Minimum rating count before a candidate can be proposed.
    pub min_ratings: u32,
    /// Minimum positive score (`up / total`) before proposal.
    pub min_score: f32,
    /// Vertical id stamped on proposed overlay candidates.
    pub vertical_id: String,
}

impl Default for PerWorkerLearningConfig {
    fn default() -> Self {
        Self {
            since_micros: 0,
            tenant_id: None,
            min_ratings: env_u32("ARGO_PER_WORKER_LEARNING_MIN_RATINGS", 3),
            min_score: env_f32("ARGO_PER_WORKER_LEARNING_MIN_SCORE", 0.75),
            vertical_id: "per-worker-learning".to_string(),
        }
    }
}

/// Summary returned by a cron tick.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerWorkerLearningReport {
    /// Rating groups evaluated after threshold filtering.
    pub evaluated: usize,
    /// Skill ids proposed through `propose_for_review_judged`.
    pub proposed: Vec<String>,
    /// Rating groups skipped because an overlay candidate already exists.
    pub skipped_existing: usize,
}

/// Run one per-worker learning tick with environment/default tunables.
pub async fn per_worker_learning_tick(
    worker_id: WorkerId,
    skills: &SkillStore,
) -> ArgoResult<PerWorkerLearningReport> {
    per_worker_learning_tick_with_config(
        worker_id,
        skills,
        PerWorkerLearningConfig::default(),
        None,
    )
    .await
}

/// Run one per-worker learning tick with explicit tunables.
pub async fn per_worker_learning_tick_with_config(
    worker_id: WorkerId,
    skills: &SkillStore,
    config: PerWorkerLearningConfig,
    judge: Option<&dyn Judge>,
) -> ArgoResult<PerWorkerLearningReport> {
    let candidates = learning_candidates_since(
        skills.pool(),
        config.since_micros,
        config.tenant_id,
        config.min_ratings,
        config.min_score,
    )
    .await?;
    let mut report = PerWorkerLearningReport {
        evaluated: candidates.len(),
        ..PerWorkerLearningReport::default()
    };
    for candidate in candidates {
        if candidate.up_count <= candidate.down_count {
            continue;
        }
        let Some(source) = source_skill(skills, &candidate).await? else {
            continue;
        };
        let overlay_name = overlay_name(&source, worker_id);
        if let Some(existing) = skill_by_name(skills, &overlay_name).await? {
            attach_worker_overlay(skills, worker_id, existing.id).await?;
            report.skipped_existing += 1;
            continue;
        }
        let raw = overlay_raw(&source, &overlay_name, worker_id)?;
        let proposed = skills
            .propose_for_review_judged(&raw, SessionId::new(), &config.vertical_id, judge)
            .await?;
        attach_worker_overlay(skills, worker_id, proposed.id).await?;
        append_learning_audit(skills, worker_id, &source, &proposed, &candidate).await?;
        report.proposed.push(proposed.id.to_string());
    }
    Ok(report)
}

/// Spawn a periodic per-worker learning cron task.
///
/// The task logs tick failures and continues; callers own the returned
/// handle and can abort it during shutdown.
pub fn spawn_per_worker_learning_cron(
    worker_id: WorkerId,
    skills: Arc<SkillStore>,
    config: PerWorkerLearningConfig,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match per_worker_learning_tick_with_config(worker_id, &skills, config.clone(), None)
                .await
            {
                Ok(report) => tracing::debug!(
                    target: "argo_skill::per_worker_learning_cron",
                    worker_id = %worker_id,
                    evaluated = report.evaluated,
                    proposed = report.proposed.len(),
                    skipped_existing = report.skipped_existing,
                    "per-worker learning cron tick completed"
                ),
                Err(err) => tracing::error!(
                    target: "argo_skill::per_worker_learning_cron",
                    worker_id = %worker_id,
                    error = %err,
                    "per-worker learning cron tick failed"
                ),
            }
        }
    })
}

fn env_u32(key: &str, fallback: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(fallback)
}

fn env_f32(key: &str, fallback: f32) -> f32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| (0.0..=1.0).contains(v))
        .unwrap_or(fallback)
}

async fn source_skill(skills: &SkillStore, candidate: &RatingSummary) -> ArgoResult<Option<Skill>> {
    let id =
        SkillId(Uuid::parse_str(&candidate.skill_id).map_err(|e| {
            ArgoError::Provider(format!("per_worker_learning skill_id parse: {e}"))
        })?);
    let row: Option<RawSkill> = sqlx::query_as::<_, RawSkill>(
        "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
                s.learned_in, s.superseded_by, s.created_at, l.lock, \
                s.vertical_id, s.lifecycle_state \
         FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
         WHERE s.id = ? AND s.version = ?",
    )
    .bind(id.0.as_bytes().as_slice())
    .bind(&candidate.skill_version)
    .fetch_optional(skills.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("per_worker_learning source skill: {e}")))?;
    row.map(Skill::try_from).transpose()
}

async fn skill_by_name(skills: &SkillStore, name: &str) -> ArgoResult<Option<Skill>> {
    let row: Option<RawSkill> = sqlx::query_as::<_, RawSkill>(
        "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
                s.learned_in, s.superseded_by, s.created_at, l.lock, \
                s.vertical_id, s.lifecycle_state \
         FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
         WHERE s.name = ? AND s.superseded_by IS NULL \
         ORDER BY s.created_at DESC LIMIT 1",
    )
    .bind(name)
    .fetch_optional(skills.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("per_worker_learning overlay lookup: {e}")))?;
    row.map(Skill::try_from).transpose()
}

fn overlay_name(source: &Skill, worker_id: WorkerId) -> String {
    let worker = worker_id.to_string();
    let short = worker.split('-').next().unwrap_or("worker");
    format!("{}__worker_overlay_{short}", source.frontmatter.name)
}

fn overlay_raw(source: &Skill, name: &str, worker_id: WorkerId) -> ArgoResult<String> {
    let mut frontmatter = source.frontmatter.clone();
    frontmatter.name = name.to_string();
    frontmatter.version = format!("{}+worker.{}", source.frontmatter.version, worker_id);
    frontmatter.description = format!(
        "Per-worker overlay candidate for {}",
        source.frontmatter.name
    );
    frontmatter.provenance.source = format!("learned:per_worker_overlay:{}", source.id);
    let yaml = serde_yaml::to_string(&frontmatter)
        .map_err(|e| ArgoError::Schema(format!("per_worker_learning yaml: {e}")))?;
    Ok(format!("---\n{yaml}---\n{}", source.body))
}

async fn attach_worker_overlay(
    skills: &SkillStore,
    worker_id: WorkerId,
    skill_id: SkillId,
) -> ArgoResult<()> {
    sqlx::query(
        "INSERT INTO worker_skill (worker_id, skill_id, proficiency) VALUES (?, ?, 5) \
         ON CONFLICT(worker_id, skill_id) DO UPDATE SET proficiency = excluded.proficiency",
    )
    .bind(worker_id.to_string())
    .bind(skill_id.to_string())
    .execute(skills.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("per_worker_learning worker overlay: {e}")))?;
    Ok(())
}

async fn append_learning_audit(
    skills: &SkillStore,
    worker_id: WorkerId,
    source: &Skill,
    proposed: &Skill,
    rating: &RatingSummary,
) -> ArgoResult<()> {
    let tenant = rating.publisher_tenant.clone();
    let payload = serde_json::json!({
        "action_type": LEARNING_ACTION,
        "outcome_status": "proposed",
        "tenant_id": tenant,
        "worker_id": worker_id.to_string(),
        "source_skill_id": source.id.to_string(),
        "proposed_skill_id": proposed.id.to_string(),
        "proposed_name": proposed.frontmatter.name,
        "up_count": rating.up_count,
        "down_count": rating.down_count,
        "score": rating.score,
    });
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Schema(format!("per_worker_learning audit json: {e}")))?;
    let parent = last_audit_hash(skills).await?;
    let self_hash = chain_hash(&parent, AUDIT_KIND, &canonical);
    let audit_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at, principal_id, tenant_id, \
         worker_id, channel, action_type, outcome_status, \
         approver_id, cost_band, data_classification) \
         VALUES (?, NULL, ?, ?, ?, ?, ?, NULL, ?, ?, NULL, ?, ?, NULL, NULL, NULL)",
    )
    .bind(audit_id.as_bytes().as_slice())
    .bind(AUDIT_KIND)
    .bind(&canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .bind(Utc::now().timestamp_micros())
    .bind(&tenant)
    .bind(worker_id.to_string())
    .bind(LEARNING_ACTION)
    .bind("proposed")
    .execute(skills.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("per_worker_learning audit insert: {e}")))?;
    Ok(())
}

async fn last_audit_hash(skills: &SkillStore) -> ArgoResult<[u8; 32]> {
    let row: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(skills.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("per_worker_learning audit hash: {e}")))?;
    let mut out = [0u8; 32];
    if let Some(bytes) = row {
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

#[cfg(test)]
#[path = "per_worker_learning_cron_tests.rs"]
mod tests;
