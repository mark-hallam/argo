//! Periodic Forge queue feeder for low-quality skills.
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

const DEFAULT_MAX_SKILLS_PER_TICK: usize = 3;
const DEFAULT_SCORE_THRESHOLD: f64 = 70.0;
const DEFAULT_INTERVAL_SECS: u64 = 3_600;
const DEFAULT_BUDGET_USD: f64 = 0.50;

/// Controls how many low-quality skills Forge may enqueue per tick.
#[derive(Debug, Clone, PartialEq)]
pub struct ForgeCronConfig {
    /// Maximum number of skills to enqueue in one tick.
    pub max_skills_per_tick: usize,
    /// Enqueue only latest quality scores strictly below this value.
    pub score_threshold: f64,
    /// Delay between ticks.
    pub interval_secs: u64,
    /// Log selected candidates without enqueueing rows.
    pub dry_run: bool,
    /// Per-run budget stamped into the queued payload.
    pub budget_usd: f64,
}

impl Default for ForgeCronConfig {
    fn default() -> Self {
        Self {
            max_skills_per_tick: DEFAULT_MAX_SKILLS_PER_TICK,
            score_threshold: DEFAULT_SCORE_THRESHOLD,
            interval_secs: DEFAULT_INTERVAL_SECS,
            dry_run: false,
            budget_usd: DEFAULT_BUDGET_USD,
        }
    }
}

impl ForgeCronConfig {
    /// Build config from `ARGO_FORGE_CRON_*` environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// Build config from a supplied lookup function.
    #[must_use]
    pub(crate) fn from_vars(mut get: impl FnMut(&str) -> Option<String>) -> Self {
        let default = Self::default();
        Self {
            max_skills_per_tick: parse_usize(
                get("ARGO_FORGE_CRON_MAX_SKILLS"),
                default.max_skills_per_tick,
            ),
            score_threshold: parse_f64(get("ARGO_FORGE_CRON_THRESHOLD"), default.score_threshold),
            interval_secs: parse_u64(get("ARGO_FORGE_CRON_INTERVAL_SECS"), default.interval_secs),
            dry_run: parse_bool(get("ARGO_FORGE_CRON_DRY_RUN"), default.dry_run),
            budget_usd: parse_f64(get("ARGO_FORGE_CRON_BUDGET_USD"), default.budget_usd),
        }
    }
}

/// Spawn the periodic Forge cron loop. The first tick fires immediately,
/// then repeats every `cfg.interval_secs`.
pub fn spawn_forge_cron(store: Arc<SessionStore>, cfg: ForgeCronConfig) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(cfg.interval_secs.max(1)));
        loop {
            ticker.tick().await;
            if let Err(e) = tick(Arc::clone(&store), &cfg).await {
                tracing::warn!(target: "forge::cron", error = %e, "forge cron tick failed");
            }
        }
    })
}

pub(crate) async fn tick(store: Arc<SessionStore>, cfg: &ForgeCronConfig) -> ArgoResult<usize> {
    let candidates = candidates(&store, cfg).await?;
    if candidates.is_empty() {
        tracing::warn!(
            target: "forge::cron",
            threshold = cfg.score_threshold,
            "forge cron found no low-quality skill candidates"
        );
        return Ok(0);
    }
    let mut enqueued = 0_usize;
    for c in candidates {
        if has_in_flight(&store, &c.skill_name).await? {
            tracing::info!(
                target: "forge::cron",
                skill_id = %c.skill_id,
                skill = %c.skill_name,
                "forge cron skipped skill with in-flight run"
            );
            continue;
        }
        if cfg.dry_run {
            tracing::info!(
                target: "forge::cron",
                skill_id = %c.skill_id,
                skill = %c.skill_name,
                score = c.score,
                "forge cron dry-run candidate"
            );
            continue;
        }
        let payload = json!({
            "skill": c.skill_name,
            "examples": examples_path(&c.skill_name),
            "iterations": 12,
            "budget_usd": cfg.budget_usd,
            "source": "forge_cron",
            "quality_score": c.score,
        });
        let payload = serde_json::to_string(&payload)
            .map_err(|e| ArgoError::Provider(format!("serialize forge cron payload: {e}")))?;
        store
            .enqueue_forge_run(&c.skill_name, &c.base_version, 0, &payload)
            .await?;
        enqueued += 1;
        tracing::info!(
            target: "forge::cron",
            skill_id = %c.skill_id,
            skill = %c.skill_name,
            score = c.score,
            "forge cron enqueued run"
        );
    }
    Ok(enqueued)
}

fn examples_path(skill_name: &str) -> String {
    format!("evals/forge/{skill_name}.jsonl")
}

#[derive(Debug)]
struct Candidate {
    skill_id: String,
    skill_name: String,
    base_version: String,
    score: i64,
}

async fn candidates(store: &SessionStore, cfg: &ForgeCronConfig) -> ArgoResult<Vec<Candidate>> {
    if cfg.max_skills_per_tick == 0 {
        return Ok(Vec::new());
    }
    let rows = sqlx::query(
        "SELECT q.skill_id, COALESCE(s.name, q.skill_id) AS skill_name, \
                COALESCE(s.version, '0.0.0') AS base_version, q.score \
         FROM skill_quality_score q \
         JOIN (SELECT skill_id, MAX(computed_at) AS mc \
               FROM skill_quality_score GROUP BY skill_id) latest \
           ON q.skill_id = latest.skill_id AND q.computed_at = latest.mc \
         LEFT JOIN skill s ON lower( \
             substr(hex(s.id),1,8) || '-' || substr(hex(s.id),9,4) || '-' || \
             substr(hex(s.id),13,4) || '-' || substr(hex(s.id),17,4) || '-' || \
             substr(hex(s.id),21,12)) = q.skill_id \
           AND s.superseded_by IS NULL \
         WHERE q.score < ? \
         ORDER BY q.score ASC, q.skill_id ASC \
         LIMIT ?",
    )
    .bind(cfg.score_threshold)
    .bind(i64::try_from(cfg.max_skills_per_tick).unwrap_or(i64::MAX))
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("forge cron candidates: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| Candidate {
            skill_id: row.get("skill_id"),
            skill_name: row.get("skill_name"),
            base_version: row.get("base_version"),
            score: row.get("score"),
        })
        .collect())
}

async fn has_in_flight(store: &SessionStore, skill_name: &str) -> ArgoResult<bool> {
    let found: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM forge_run \
         WHERE skill_name = ? AND status IN ('queued', 'running', 'pending') LIMIT 1",
    )
    .bind(skill_name)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("forge cron in-flight check: {e}")))?;
    Ok(found.is_some())
}

fn parse_usize(v: Option<String>, default: usize) -> usize {
    v.and_then(|s| s.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(default)
}

fn parse_u64(v: Option<String>, default: u64) -> u64 {
    v.and_then(|s| s.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(default)
}

fn parse_f64(v: Option<String>, default: f64) -> f64 {
    v.and_then(|s| s.parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .unwrap_or(default)
}

fn parse_bool(v: Option<String>, default: bool) -> bool {
    v.map(|s| matches!(s.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(default)
}

#[cfg(test)]
#[path = "cron_tests.rs"]
mod tests;
