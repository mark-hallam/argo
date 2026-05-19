//! `argo skill reeval` — on-demand wrapper for scheduled skill
//! re-evaluation.

use argo_skill::{scheduled_reeval_skill, scheduled_reeval_tick, SkillStore};
use argo_types::{ArgoError, ArgoResult, SkillId};
use uuid::Uuid;

/// Run scheduled re-evaluation on one active skill or every active
/// skill with declared evals.
pub async fn run(
    skills: &SkillStore,
    skill: Option<&str>,
    all: bool,
    json: bool,
) -> ArgoResult<()> {
    let report = if all {
        scheduled_reeval_tick(skills).await?
    } else {
        let raw = skill
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                ArgoError::Provider("reeval: pass --skill <name-or-id> or --all".into())
            })?;
        let id = resolve_skill_id(skills, raw).await?;
        scheduled_reeval_skill(skills, id).await?
    };
    if json {
        println!(
            "{}",
            serde_json::to_string(&report)
                .map_err(|e| ArgoError::Schema(format!("reeval report json: {e}")))?
        );
    } else {
        println!(
            "evaluated={} regressed={} flagged={} skipped_no_eval={}",
            report.evaluated, report.regressed, report.flagged, report.skipped_no_eval
        );
    }
    Ok(())
}

async fn resolve_skill_id(skills: &SkillStore, raw: &str) -> ArgoResult<SkillId> {
    if let Ok(uuid) = Uuid::parse_str(raw) {
        return Ok(SkillId(uuid));
    }
    let skill = skills
        .find_active_by_name(raw)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill '{raw}'")))?;
    Ok(skill.id)
}
