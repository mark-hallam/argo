//! `argo skill quality-list` — list every scored skill, worst
//! first. Optional `--threshold` and `--worst` filters.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{QualityScore, SkillQualityStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
struct ListRow {
    skill_id: String,
    score: u8,
    success_rate: f64,
    avg_latency_ms: f64,
    retry_count: u32,
    p99_latency_ms: f64,
    computed_at_rfc3339: String,
}

pub async fn run(threshold: Option<u8>, worst: Option<u32>, json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(db_path(&home)).await?);
    store.migrate().await?;
    let qstore = SkillQualityStore::new(store);
    qstore.ensure_schema().await?;
    let mut scores: Vec<QualityScore> = qstore
        .latest_all()
        .await?
        .into_iter()
        .filter(|s| match threshold {
            Some(t) => s.score <= t,
            None => true,
        })
        .collect();
    if let Some(n) = worst {
        scores.truncate(n as usize);
    }
    let rows: Vec<ListRow> = scores
        .into_iter()
        .map(|s| ListRow {
            skill_id: s.skill_id.to_string(),
            score: s.score,
            success_rate: s.inputs.success_rate,
            avg_latency_ms: s.inputs.avg_latency_ms,
            retry_count: s.inputs.retry_count,
            p99_latency_ms: s.inputs.p99_latency_ms,
            computed_at_rfc3339: s.computed_at.to_rfc3339(),
        })
        .collect();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&rows)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else if rows.is_empty() {
        println!("(no scored skills)");
    } else {
        println!(
            "{:<36}  {:>5}  {:>12}  {:>12}  {:>7}  {:>12}  computed_at",
            "skill_id", "score", "success", "avg_latency", "retries", "p99_latency"
        );
        for r in &rows {
            println!(
                "{:<36}  {:>5}  {:>12.3}  {:>12.1}  {:>7}  {:>12.1}  {}",
                r.skill_id,
                r.score,
                r.success_rate,
                r.avg_latency_ms,
                r.retry_count,
                r.p99_latency_ms,
                r.computed_at_rfc3339
            );
        }
    }
    Ok(())
}
