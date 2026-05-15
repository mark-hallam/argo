//! `argo skill explain` rendering.

use argo_skill::{explain_skill, SkillExplanation, SkillStore};
use argo_types::ArgoResult;

pub async fn run(
    skills: &SkillStore,
    name: &str,
    history_depth: usize,
    json: bool,
) -> ArgoResult<()> {
    let explanation = explain_skill(skills, name, history_depth).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&explanation).unwrap_or_default()
        );
    } else {
        render_text(&explanation);
    }
    Ok(())
}

fn render_text(explanation: &SkillExplanation) {
    println!(
        "{} v{} ({:?})",
        explanation.name, explanation.current_version, explanation.current_trust_grade
    );
    println!("provenance: {:?}", explanation.provenance);
    println!("versions:");
    for version in &explanation.version_history {
        let reason = version.change_reason.as_deref().unwrap_or("-");
        let author = version.author.as_deref().unwrap_or("-");
        println!(
            "  - {} reason={} author={} breaking={}",
            version.version, reason, author, version.breaking_change
        );
    }
    println!("forge runs: {}", explanation.forge_runs.len());
    println!("trust observations: {}", explanation.trust_history.len());
    println!(
        "usage: {} invocations/30d, {} successes/7d, {:.1}% error rate",
        explanation.usage_summary.invocations_last_30d,
        explanation.usage_summary.invocations_last_7d,
        explanation.usage_summary.error_rate_pct
    );
}
