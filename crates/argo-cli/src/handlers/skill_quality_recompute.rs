//! `argo skill quality-recompute` — manually recompute the quality
//! score from operator-supplied inputs (or defaults). Stand-in for
//! the background tick deferred to a follow-up brief.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{QualityInputs, SkillQualityStore, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
struct RecomputedOut {
    skill_id: String,
    name: Option<String>,
    score: u8,
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    name: Option<&str>,
    all: bool,
    success_rate: Option<f64>,
    avg_latency_ms: Option<f64>,
    retry_count: Option<u32>,
    p99_latency_ms: Option<f64>,
    json: bool,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(db_path(&home)).await?);
    store.migrate().await?;
    let skills = SkillStore::new(Arc::clone(&store));
    let qstore = SkillQualityStore::new(store);
    qstore.ensure_schema().await?;
    let inputs = build_inputs(success_rate, avg_latency_ms, retry_count, p99_latency_ms);
    let now = Utc::now();
    let mut outs: Vec<RecomputedOut> = Vec::new();
    if all {
        for skill in skills.list_active().await? {
            let score = qstore.record(skill.id, &inputs, now).await?;
            outs.push(RecomputedOut {
                skill_id: skill.id.to_string(),
                name: Some(skill.frontmatter.name.clone()),
                score: score.score,
            });
        }
    } else {
        let n = name.ok_or_else(|| {
            ArgoError::Provider("recompute: pass --all or a skill name".into())
        })?;
        let skill = skills
            .find_active_by_name(n)
            .await?
            .ok_or_else(|| ArgoError::Provider(format!("no active skill '{n}'")))?;
        let recorded = qstore.record(skill.id, &inputs, now).await?;
        outs.push(RecomputedOut {
            skill_id: recorded.skill_id.to_string(),
            name: Some(skill.frontmatter.name),
            score: recorded.score,
        });
    }
    render(&outs, &inputs, json)?;
    Ok(())
}

fn build_inputs(
    success_rate: Option<f64>,
    avg_latency_ms: Option<f64>,
    retry_count: Option<u32>,
    p99_latency_ms: Option<f64>,
) -> QualityInputs {
    let defaults = QualityInputs::default();
    QualityInputs {
        success_rate: success_rate.unwrap_or(defaults.success_rate),
        avg_latency_ms: avg_latency_ms.unwrap_or(defaults.avg_latency_ms),
        retry_count: retry_count.unwrap_or(defaults.retry_count),
        p99_latency_ms: p99_latency_ms.unwrap_or(defaults.p99_latency_ms),
    }
}

fn render(outs: &[RecomputedOut], inputs: &QualityInputs, json: bool) -> ArgoResult<()> {
    if json {
        let body = serde_json::json!({
            "inputs": inputs,
            "recomputed": outs,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&body)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else if outs.is_empty() {
        println!("(no skills to recompute)");
    } else {
        for o in outs {
            let label = o.name.as_deref().unwrap_or(&o.skill_id);
            println!("recomputed '{label}' → score {}", o.score);
        }
    }
    Ok(())
}

