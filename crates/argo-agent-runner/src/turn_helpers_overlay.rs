//! Per-worker skill overlay merge for [`load_prompt_skills_with_library`]
//! (mission-pm-skill-overlay-load).
//!
//! Extracted as a sibling of `turn_helpers.rs` so that file stays under
//! the 500-LOC HARD RULE 6 ceiling. Loaded via
//! `#[path = "turn_helpers_overlay.rs"] mod turn_helpers_overlay;` from
//! `turn.rs`, mirroring the `turn_helpers_context` sibling pattern.
//!
//! Overlay skills come from `worker_skill` (migration 101). The
//! per-worker learning cron promotes positive-rated skills into that
//! table; until this brief, the read-side prompt loader did not
//! consult it, so promotion was invisible to the worker. These
//! helpers close the loop.
//!
//! [`load_prompt_skills_with_library`]: super::turn_helpers
use argo_skill::{LayeredSkill, Skill, SkillLayer, SkillStore};
use argo_store::WorkerId;
use argo_types::ArgoResult;

/// Append per-worker overlay skills from `worker_skill` into a
/// [`LayeredSkill`] row set as the [`SkillLayer::Workspace`] tier,
/// deduped by name against rows already present.
///
/// Tagged Workspace (not Library) so the existing
/// [`argo_skill::SkillRelevanceSelector`] passes them through with
/// the workspace `vertical_id` rule rather than the library
/// `published:` source filter — overlays are personalised, not
/// marketplace content. Names already represented by a workspace or
/// library row are skipped so the established row wins (no
/// precedence bug against workspace skills).
pub(super) async fn append_worker_overlays_as_workspace(
    workspace: &SkillStore,
    rows: &mut Vec<LayeredSkill>,
    worker_id: Option<WorkerId>,
) -> ArgoResult<()> {
    let Some(worker_id) = worker_id else {
        return Ok(());
    };
    let seen: std::collections::HashSet<String> = rows
        .iter()
        .map(|r| r.skill.frontmatter.name.clone())
        .collect();
    for skill in workspace.list_for_worker(worker_id).await? {
        if seen.contains(&skill.frontmatter.name) {
            continue;
        }
        rows.push(LayeredSkill {
            skill,
            layer: SkillLayer::Workspace,
        });
    }
    Ok(())
}

/// Library-less fallback: extend a workspace-only result vec with
/// per-worker overlays, deduped by name. Used when no skill library
/// is configured (`ARGO_SKILL_LIBRARY_DB` unset).
pub(super) async fn merge_worker_overlays(
    workspace: &SkillStore,
    base: Vec<Skill>,
    worker_id: Option<WorkerId>,
) -> ArgoResult<Vec<Skill>> {
    let Some(worker_id) = worker_id else {
        return Ok(base);
    };
    let mut out = base;
    let seen: std::collections::HashSet<String> =
        out.iter().map(|s| s.frontmatter.name.clone()).collect();
    for skill in workspace.list_for_worker(worker_id).await? {
        if seen.contains(&skill.frontmatter.name) {
            continue;
        }
        out.push(skill);
    }
    Ok(out)
}
