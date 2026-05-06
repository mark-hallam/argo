//! `argo skill quality <name>` — show the latest quality score +
//! inputs for one skill (`claude-skill-quality-judge`).

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{QualityScore, SkillQualityStore, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

pub async fn run(name: &str, json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(db_path(&home)).await?);
    store.migrate().await?;
    let skills = SkillStore::new(Arc::clone(&store));
    let skill = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))?;
    let qstore = SkillQualityStore::new(store);
    qstore.ensure_schema().await?;
    match qstore.latest(skill.id).await? {
        Some(score) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&score)
                        .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
                );
            } else {
                print_text(&score);
            }
        }
        None => {
            if json {
                println!("null");
            } else {
                println!("(skill '{name}' has no quality score yet)");
            }
        }
    }
    Ok(())
}

fn print_text(score: &QualityScore) {
    println!("skill_id:        {}", score.skill_id);
    println!("score:           {}", score.score);
    println!("computed_at:     {}", score.computed_at.to_rfc3339());
    println!("inputs.success_rate:    {:.3}", score.inputs.success_rate);
    println!("inputs.avg_latency_ms:  {:.1}", score.inputs.avg_latency_ms);
    println!("inputs.retry_count:     {}", score.inputs.retry_count);
    println!("inputs.p99_latency_ms:  {:.1}", score.inputs.p99_latency_ms);
}
