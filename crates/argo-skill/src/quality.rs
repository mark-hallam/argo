//! V2.0 — Per-skill 0..100 quality score.
//!
//! Operators want a single number that summarises "should I keep
//! using this skill?" — falling success rate, slow latency, or
//! lots of retries should drop the score visibly.
//!
//! The score is a weighted combination of four inputs (see
//! [`QualityInputs`]). It's deterministic + offline — recomputable
//! from the audit ledger any time. Live scores live in the
//! `skill_quality_score` table (one row per (skill, computed_at)).
//!
//! ## Score formula
//!
//! Starts from 100 and applies penalties:
//!
//! - **Base** = `success_rate * 100` (0..=100).
//! - **Retry penalty** = `min(retry_count, 25) * 2` (caps at 50).
//! - **Avg-latency penalty**:
//!   - `0` if `avg_latency_ms <= 5000`
//!   - `10` if `5000 < avg_latency_ms <= 30000`
//!   - `20` if `avg_latency_ms > 30000`
//! - **P99 penalty**:
//!   - `0` if `p99_latency_ms <= 30000`
//!   - `10` if `30000 < p99_latency_ms <= 120000`
//!   - `20` if `p99_latency_ms > 120000`
//!
//! Final score = `clamp(base - retry_penalty - latency_penalty -
//! p99_penalty, 0, 100)` cast to `u8`.

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SkillId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

const ENSURE_QUALITY_SCHEMA_SQL: &str =
    include_str!("../../argo-store/migrations/24__skill_quality_score.sql");

/// Operational inputs the quality formula consumes. Built per
/// skill from recent invocations (typically the last 30 days).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QualityInputs {
    /// Fraction of recent invocations that completed successfully
    /// (0.0..=1.0). 1.0 = every recent run finished cleanly.
    pub success_rate: f64,
    /// Mean wall-time per invocation in milliseconds.
    pub avg_latency_ms: f64,
    /// Total retries across recent invocations.
    pub retry_count: u32,
    /// 99th-percentile wall-time per invocation in milliseconds.
    pub p99_latency_ms: f64,
}

impl Default for QualityInputs {
    fn default() -> Self {
        // Empty-history default: assume best-case so a brand-new
        // skill isn't penalised before it's been used.
        Self {
            success_rate: 1.0,
            avg_latency_ms: 0.0,
            retry_count: 0,
            p99_latency_ms: 0.0,
        }
    }
}

/// Compute the quality score from inputs. See module docs for the
/// formula.
///
/// ```
/// use argo_skill::quality::{compute_score, QualityInputs};
/// // Perfect inputs → 100.
/// let perfect = QualityInputs::default();
/// assert_eq!(compute_score(&perfect), 100);
/// ```
#[must_use]
pub fn compute_score(inputs: &QualityInputs) -> u8 {
    let base = (inputs.success_rate.clamp(0.0, 1.0) * 100.0) as i32;
    let retry_penalty = inputs.retry_count.min(25) as i32 * 2;
    let avg_penalty = if inputs.avg_latency_ms > 30_000.0 {
        20
    } else if inputs.avg_latency_ms > 5_000.0 {
        10
    } else {
        0
    };
    let p99_penalty = if inputs.p99_latency_ms > 120_000.0 {
        20
    } else if inputs.p99_latency_ms > 30_000.0 {
        10
    } else {
        0
    };
    let raw = base - retry_penalty - avg_penalty - p99_penalty;
    raw.clamp(0, 100) as u8
}

/// One persisted score row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityScore {
    /// Skill being scored.
    pub skill_id: SkillId,
    /// 0..=100 quality score.
    pub score: u8,
    /// Snapshot of the inputs that produced this score.
    pub inputs: QualityInputs,
    /// When the score was computed.
    pub computed_at: DateTime<Utc>,
}

/// Persistence facade over `skill_quality_score`.
#[derive(Clone)]
pub struct SkillQualityStore {
    store: Arc<SessionStore>,
}

impl std::fmt::Debug for SkillQualityStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillQualityStore").finish()
    }
}

impl SkillQualityStore {
    /// Wrap a [`SessionStore`].
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Apply migration 24. Idempotent.
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        for stmt in ENSURE_QUALITY_SCHEMA_SQL.split_terminator(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql)
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("ensure quality schema: {e}")))?;
        }
        Ok(())
    }

    /// Compute + persist a score for one skill. Returns the new
    /// row.
    pub async fn record(
        &self,
        skill_id: SkillId,
        inputs: &QualityInputs,
        now: DateTime<Utc>,
    ) -> ArgoResult<QualityScore> {
        let score = compute_score(inputs);
        let inputs_json = serde_json::to_string(inputs)
            .map_err(|e| ArgoError::Provider(format!("serialize inputs: {e}")))?;
        sqlx::query(
            "INSERT OR REPLACE INTO skill_quality_score \
             (skill_id, score, inputs_json, computed_at) VALUES (?, ?, ?, ?)",
        )
        .bind(skill_id.to_string())
        .bind(i64::from(score))
        .bind(inputs_json)
        .bind(now.timestamp_micros())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("insert quality score: {e}")))?;
        Ok(QualityScore {
            skill_id,
            score,
            inputs: *inputs,
            computed_at: now,
        })
    }

    /// Most-recent score for a skill (or `None` if never scored).
    pub async fn latest(&self, skill_id: SkillId) -> ArgoResult<Option<QualityScore>> {
        let row = sqlx::query(
            "SELECT skill_id, score, inputs_json, computed_at \
             FROM skill_quality_score WHERE skill_id = ? \
             ORDER BY computed_at DESC LIMIT 1",
        )
        .bind(skill_id.to_string())
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("latest quality score: {e}")))?;
        row.map(score_from_row).transpose()
    }

    /// Most-recent score for every skill that has at least one
    /// score row, ordered by score ascending (worst first).
    pub async fn latest_all(&self) -> ArgoResult<Vec<QualityScore>> {
        let rows = sqlx::query(
            "SELECT q.skill_id, q.score, q.inputs_json, q.computed_at \
             FROM skill_quality_score q \
             JOIN ( \
                 SELECT skill_id, MAX(computed_at) AS mc \
                 FROM skill_quality_score GROUP BY skill_id \
             ) m ON q.skill_id = m.skill_id AND q.computed_at = m.mc \
             ORDER BY q.score ASC, q.skill_id ASC",
        )
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("latest_all quality scores: {e}")))?;
        rows.into_iter().map(score_from_row).collect()
    }

    /// Full history for a skill, newest first.
    pub async fn history(&self, skill_id: SkillId) -> ArgoResult<Vec<QualityScore>> {
        let rows = sqlx::query(
            "SELECT skill_id, score, inputs_json, computed_at \
             FROM skill_quality_score WHERE skill_id = ? \
             ORDER BY computed_at DESC",
        )
        .bind(skill_id.to_string())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("quality history: {e}")))?;
        rows.into_iter().map(score_from_row).collect()
    }

    /// Map of `skill_id → latest score`, populated for every skill
    /// that has been scored. Convenience for the `--quality-min`
    /// filter on `argo skill list`.
    pub async fn latest_by_skill(
        &self,
    ) -> ArgoResult<std::collections::HashMap<String, u8>> {
        let rows = self.latest_all().await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.skill_id.to_string(), r.score))
            .collect())
    }
}

fn score_from_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<QualityScore> {
    let skill_id_str: String = row.get("skill_id");
    let score: i64 = row.get("score");
    let inputs_json: String = row.get("inputs_json");
    let computed_at_micros: i64 = row.get("computed_at");
    let skill_id = Uuid::parse_str(&skill_id_str)
        .map(SkillId)
        .map_err(|e| ArgoError::Provider(format!("decode skill id '{skill_id_str}': {e}")))?;
    let inputs: QualityInputs = serde_json::from_str(&inputs_json)
        .map_err(|e| ArgoError::Provider(format!("decode quality inputs: {e}")))?;
    let computed_at = DateTime::<Utc>::from_timestamp_micros(computed_at_micros)
        .ok_or_else(|| ArgoError::Provider(format!("decode computed_at: {computed_at_micros}")))?;
    let score_u8 = u8::try_from(score)
        .map_err(|e| ArgoError::Provider(format!("decode score '{score}': {e}")))?;
    Ok(QualityScore {
        skill_id,
        score: score_u8,
        inputs,
        computed_at,
    })
}

#[cfg(test)]
#[path = "quality_tests.rs"]
mod tests;
