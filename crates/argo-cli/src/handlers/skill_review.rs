//! `argo skill review {list, approve, reject}` (#684).
//!
//! Drives the learned-skill review gate from #681. Without
//! this surface the lifecycle gate is theoretical — agents
//! can propose skills via `propose_for_review` (#681) but
//! operators have no way to act on the queue.
//!
//! Approve / reject emit dedicated audit kinds
//! (`skill.approved` / `skill.rejected`) so the chain shows
//! who promoted what + when. State changes flow through the
//! existing `transition_lifecycle` graph (PendingReview →
//! Active | Rejected) — illegal transitions return
//! `ArgoError::Schema` and the CLI surfaces them as exit-1.
use crate::cli::SkillReviewCmd;
use argo_audit::{AuditKind, AuditSink};
use argo_skill::{LifecycleState, SkillStore};
use argo_types::{ArgoError, ArgoResult, SkillId};
use uuid::Uuid;

pub async fn run(skills: &SkillStore, cmd: SkillReviewCmd) -> ArgoResult<()> {
    match cmd {
        SkillReviewCmd::List { json } => list_pending(skills, json).await,
        SkillReviewCmd::Approve { skill_id, json } => {
            transition(skills, &skill_id, LifecycleState::Active, json).await
        }
        SkillReviewCmd::Reject { skill_id, json } => {
            transition(skills, &skill_id, LifecycleState::Rejected, json).await
        }
    }
}

async fn list_pending(skills: &SkillStore, json: bool) -> ArgoResult<()> {
    let pending = skills.list_pending_review().await?;
    if json {
        let arr: Vec<serde_json::Value> = pending
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id.0.to_string(),
                    "name": s.frontmatter.name,
                    "version": s.frontmatter.version,
                    "vertical_id": s.vertical_id,
                    "learned_in": s.learned_in.map(|sid| sid.0.to_string()),
                    "created_at": s.created_at.to_rfc3339(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "count": arr.len(),
                "pending": arr,
            }))
            .unwrap_or_default()
        );
    } else if pending.is_empty() {
        println!("(no skills awaiting review)");
    } else {
        println!("pending review queue — {} skill(s):", pending.len());
        for s in &pending {
            let vert = s.vertical_id.as_deref().unwrap_or("(none)");
            let learned = s
                .learned_in
                .map(|sid| sid.0.to_string())
                .unwrap_or_else(|| "(none)".into());
            println!(
                "  [{}] {} v{} — vertical={vert}, learned_in={learned}, at={}",
                s.id.0,
                s.frontmatter.name,
                s.frontmatter.version,
                s.created_at.to_rfc3339()
            );
        }
        println!();
        println!("approve: argo skill review approve <id>");
        println!("reject:  argo skill review reject <id>");
    }
    Ok(())
}

async fn transition(
    skills: &SkillStore,
    skill_id_str: &str,
    next: LifecycleState,
    json: bool,
) -> ArgoResult<()> {
    let uuid = Uuid::parse_str(skill_id_str).map_err(|e| {
        ArgoError::Provider(format!(
            "invalid skill id '{skill_id_str}': {e}"
        ))
    })?;
    let skill_id = SkillId(uuid);

    // Read the current row so we can build a precise audit
    // payload (name, vertical_id, learned_in) — surfacing them
    // makes the chain row self-explanatory without a join.
    let pre = lookup_by_id(skills, skill_id).await?;
    let new_state = skills.transition_lifecycle(skill_id, next).await?;

    // Emit the appropriate audit row through the same store
    // the SkillStore wraps. The session_id we attribute to is
    // `pre.learned_in` so the chain row lands in the originating
    // session's per-session audit timeline.
    let audit = AuditSink::new(skills.store_arc());
    let payload = serde_json::json!({
        "skill_id": skill_id.0.to_string(),
        "name": pre.frontmatter.name,
        "version": pre.frontmatter.version,
        "vertical_id": pre.vertical_id,
        "to": new_state.label(),
    });
    let kind = match new_state {
        LifecycleState::Active => AuditKind::SkillApproved,
        LifecycleState::Rejected => AuditKind::SkillRejected,
        // The graph forbids other transitions via the public
        // CLI; if we ever get here the audit chain just records
        // the underlying state via approval kinds.
        _ => AuditKind::SkillApproved,
    };
    audit
        .write(pre.learned_in, kind, payload.clone())
        .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "skill_id": skill_id.0.to_string(),
                "name": pre.frontmatter.name,
                "to": new_state.label(),
            }))
            .unwrap_or_default()
        );
    } else {
        let verb = if matches!(new_state, LifecycleState::Active) {
            "approved"
        } else {
            "rejected"
        };
        println!(
            "{verb} skill '{}' v{} → {}",
            pre.frontmatter.name,
            pre.frontmatter.version,
            new_state.label()
        );
    }
    Ok(())
}

/// Per-id lookup helper. The SkillStore doesn't have a public
/// `get_by_id`, so we walk the pending-review list (cheap; the
/// review queue is tiny). For approve cases (skill is no longer
/// pending) we fall back to the active-by-name path via the
/// frontmatter, which we don't have until we read SOMETHING …
/// so for now: scan all_active + pending. Bounded by table
/// size; review traffic is low-volume.
async fn lookup_by_id(skills: &SkillStore, target: SkillId) -> ArgoResult<argo_skill::Skill> {
    if let Some(found) = skills
        .list_pending_review()
        .await?
        .into_iter()
        .find(|s| s.id == target)
    {
        return Ok(found);
    }
    if let Some(found) = skills
        .list_active()
        .await?
        .into_iter()
        .find(|s| s.id == target)
    {
        return Ok(found);
    }
    Err(ArgoError::Provider(format!(
        "no skill with id '{}'",
        target.0
    )))
}

#[cfg(test)]
#[path = "skill_review_tests.rs"]
mod tests;
