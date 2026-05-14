//! `argo skill quality <name>` — show the latest quality score
//! (and optional history) + inputs for one skill
//! (`claude-skill-quality-judge` /
//! `claude-skill-quality-score-vis`).
//!
//! Default render is the legacy text view (latest only) so the
//! older `argo skill quality <name>` callers see no output drift.
//! `--json` (or `--format json`) emits the historical pipeline-
//! friendly shape; `--format markdown` emits a single-skill
//! markdown report; `--limit N` widens any of the three modes
//! into a "last N scores" history block.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{QualityScore, SkillQualityStore, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SkillId};
use std::sync::Arc;

pub async fn run(name: &str, json: bool, format: &str, limit: u32) -> ArgoResult<()> {
    let chosen = if json { "json" } else { format };
    if !matches!(chosen, "text" | "markdown" | "json") {
        return Err(ArgoError::Provider(format!(
            "unknown --format '{format}' — use 'text', 'markdown', or 'json'"
        )));
    }
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
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
    let scores = load_scores(&qstore, skill.id, limit).await?;
    print_view(name, &scores, chosen, limit)?;
    Ok(())
}

/// Pull the latest score (limit==1) or the trailing history
/// (limit>1) for a skill. Always returns most-recent-first.
pub(super) async fn load_scores(
    qstore: &SkillQualityStore,
    skill_id: SkillId,
    limit: u32,
) -> ArgoResult<Vec<QualityScore>> {
    if limit <= 1 {
        return Ok(qstore.latest(skill_id).await?.into_iter().collect());
    }
    let mut hist = qstore.history(skill_id).await?;
    hist.sort_by_key(|b| std::cmp::Reverse(b.computed_at));
    let n = limit as usize;
    if hist.len() > n {
        hist.truncate(n);
    }
    Ok(hist)
}

pub(super) fn print_view(
    name: &str,
    scores: &[QualityScore],
    format: &str,
    limit: u32,
) -> ArgoResult<()> {
    match format {
        "json" => print_json(name, scores, limit),
        "markdown" => {
            print!("{}", render_markdown(name, scores, limit));
            Ok(())
        }
        _ => {
            print_text_block(name, scores, limit);
            Ok(())
        }
    }
}

fn print_json(name: &str, scores: &[QualityScore], limit: u32) -> ArgoResult<()> {
    if limit <= 1 {
        match scores.first() {
            Some(score) => {
                let s = serde_json::to_string_pretty(score)
                    .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?;
                println!("{s}");
            }
            None => println!("null"),
        }
        return Ok(());
    }
    let payload = serde_json::json!({
        "skill_name": name,
        "latest": scores.first(),
        "history": scores,
        "history_len": scores.len(),
    });
    let s = serde_json::to_string_pretty(&payload)
        .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?;
    println!("{s}");
    Ok(())
}

fn print_text_block(name: &str, scores: &[QualityScore], limit: u32) {
    if scores.is_empty() {
        println!("(skill '{name}' has no quality score yet)");
        return;
    }
    let latest = &scores[0];
    print_text_one(latest);
    if limit > 1 && scores.len() > 1 {
        println!();
        println!("history ({} prior score{}):", scores.len() - 1, if scores.len() == 2 { "" } else { "s" });
        for score in scores.iter().skip(1) {
            println!(
                "  {} → score={} success_rate={:.3} avg_latency_ms={:.1}",
                score.computed_at.to_rfc3339(),
                score.score,
                score.inputs.success_rate,
                score.inputs.avg_latency_ms,
            );
        }
    }
}

fn print_text_one(score: &QualityScore) {
    println!("skill_id:        {}", score.skill_id);
    println!("score:           {}", score.score);
    println!("computed_at:     {}", score.computed_at.to_rfc3339());
    println!("inputs.success_rate:    {:.3}", score.inputs.success_rate);
    println!("inputs.avg_latency_ms:  {:.1}", score.inputs.avg_latency_ms);
    println!("inputs.retry_count:     {}", score.inputs.retry_count);
    println!("inputs.p99_latency_ms:  {:.1}", score.inputs.p99_latency_ms);
}

pub(super) fn render_markdown(name: &str, scores: &[QualityScore], limit: u32) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Skill quality — `{name}`\n\n"));
    if scores.is_empty() {
        out.push_str("(no quality score recorded yet)\n");
        return out;
    }
    let latest = &scores[0];
    out.push_str("## Latest\n\n");
    out.push_str(&format!("- **score**: `{}`\n", latest.score));
    out.push_str(&format!(
        "- **computed_at**: `{}`\n",
        latest.computed_at.to_rfc3339()
    ));
    out.push_str(&format!("- **skill_id**: `{}`\n\n", latest.skill_id));
    out.push_str("### Inputs\n\n");
    out.push_str(&format!(
        "- success_rate: `{:.3}`\n",
        latest.inputs.success_rate
    ));
    out.push_str(&format!(
        "- avg_latency_ms: `{:.1}`\n",
        latest.inputs.avg_latency_ms
    ));
    out.push_str(&format!(
        "- retry_count: `{}`\n",
        latest.inputs.retry_count
    ));
    out.push_str(&format!(
        "- p99_latency_ms: `{:.1}`\n",
        latest.inputs.p99_latency_ms
    ));
    if limit > 1 && scores.len() > 1 {
        out.push_str(&format!(
            "\n## History ({} prior score{})\n\n",
            scores.len() - 1,
            if scores.len() == 2 { "" } else { "s" }
        ));
        out.push_str("| computed_at | score | success_rate | avg_latency_ms |\n");
        out.push_str("| --- | ---: | ---: | ---: |\n");
        for score in scores.iter().skip(1) {
            out.push_str(&format!(
                "| `{}` | {} | {:.3} | {:.1} |\n",
                score.computed_at.to_rfc3339(),
                score.score,
                score.inputs.success_rate,
                score.inputs.avg_latency_ms,
            ));
        }
    }
    out
}

#[cfg(test)]
#[path = "skill_quality_tests.rs"]
mod tests;
