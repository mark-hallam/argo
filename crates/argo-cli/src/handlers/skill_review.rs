//! `argo skill review {list, show, approve, reject}` (#684).
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
use argo_skill::{LifecycleState, Skill, SkillStore};
use argo_store::feedback::{latest_vote_for_action, Vote};
use argo_types::{ArgoError, ArgoResult, SkillId};
use uuid::Uuid;

pub async fn run(skills: &SkillStore, cmd: SkillReviewCmd) -> ArgoResult<()> {
    match cmd {
        SkillReviewCmd::List {
            limit,
            vertical,
            json,
        } => list_pending(skills, limit, vertical.as_deref(), json).await,
        SkillReviewCmd::Show { skill_id, json } => show(skills, &skill_id, json).await,
        SkillReviewCmd::Approve {
            skill_id,
            note,
            json,
        } => {
            transition(
                skills,
                &skill_id,
                LifecycleState::Active,
                note.as_deref(),
                json,
            )
            .await
        }
        SkillReviewCmd::Reject {
            skill_id,
            note,
            json,
        } => {
            transition(
                skills,
                &skill_id,
                LifecycleState::Rejected,
                note.as_deref(),
                json,
            )
            .await
        }
        SkillReviewCmd::Promote {
            memory_id,
            feedback_action,
            json,
        } => promote(skills, &memory_id, feedback_action.as_deref(), json).await,
        SkillReviewCmd::Export {
            skill_id,
            output,
            format,
        } => super::skill_export::run(skills, &skill_id, &output, &format).await,
        SkillReviewCmd::Import { path, lock } => {
            super::skill_import::run(skills, &path, lock.as_deref()).await
        }
    }
}

/// Materialise a memory-backed candidate into a `pending_review`
/// skill row (#697). The new skill is NOT auto-active — operator
/// must run `argo skill review approve <new-skill-id>` next.
async fn promote(
    skills: &SkillStore,
    memory_id: &str,
    feedback_action: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let feedback = confirm_feedback(skills, feedback_action).await?;
    let new_skill =
        super::skill_review_candidates::promote_candidate(skills, memory_id, feedback).await?;
    if json {
        let mut body = serde_json::json!({
            "ok": true,
            "skill_id": new_skill.id.0.to_string(),
            "name": new_skill.frontmatter.name,
            "vertical_id": new_skill.vertical_id,
            "lifecycle_state": "pending_review",
            "memory_entry_id": memory_id,
        });
        if let Some(feedback) = feedback {
            body["feedback"] = serde_json::json!({
                "action_id": feedback.action_id,
                "vote": feedback.vote.as_str(),
            });
        }
        println!(
            "{}",
            serde_json::to_string(&body).unwrap_or_default()
        );
    } else {
        println!(
            "promoted memory candidate `{memory_id}` -> skill `{}` (id={}) in pending_review",
            new_skill.frontmatter.name, new_skill.id.0,
        );
        if let Some(feedback) = feedback {
            println!(
                "feedback confirmed: action `{}` latest vote `{}`",
                feedback.action_id,
                feedback.vote.as_str()
            );
        }
        println!("approve next: argo skill review approve {}", new_skill.id.0);
    }
    Ok(())
}

async fn confirm_feedback<'a>(
    skills: &SkillStore,
    action_id: Option<&'a str>,
) -> ArgoResult<Option<super::skill_review_candidates::PromotionFeedback<'a>>> {
    let Some(action_id) = action_id else {
        return Ok(None);
    };
    let store = skills.store_arc();
    super::feedback_common::ensure_feedback_schema(&store).await?;
    let vote = latest_vote_for_action(store.pool(), action_id)
        .await
        .map_err(|e| ArgoError::Provider(format!("read operator feedback: {e}")))?;
    match vote {
        Some(Vote::Up) => Ok(Some(
            super::skill_review_candidates::PromotionFeedback {
                action_id,
                vote: Vote::Up,
            },
        )),
        Some(Vote::Down) => Err(ArgoError::Provider(format!(
            "latest operator feedback for action '{action_id}' is down; promotion requires an up vote"
        ))),
        None => Err(ArgoError::Provider(format!(
            "no operator feedback found for action '{action_id}'; promotion requires an up vote"
        ))),
    }
}

async fn list_pending(
    skills: &SkillStore,
    limit: Option<usize>,
    vertical: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let mut pending = skills.list_pending_review().await?;
    if let Some(vertical) = vertical {
        pending.retain(|s| s.vertical_id.as_deref() == Some(vertical));
    }
    if let Some(limit) = limit {
        pending.truncate(limit);
    }

    // #697: also surface memory-backed candidates from every
    // loaded vertical's `learned_skill_candidates` rows.
    let mut memory_candidates =
        super::skill_review_candidates::collect_memory_candidates(skills).await?;
    if let Some(vertical) = vertical {
        memory_candidates.retain(|c| c.vertical_id == vertical);
    }
    if let Some(limit) = limit {
        memory_candidates.truncate(limit);
    }

    if json {
        let pending_arr: Vec<serde_json::Value> = pending.iter().map(skill_summary_json).collect();
        let candidate_arr: Vec<serde_json::Value> = memory_candidates
            .iter()
            .map(|c| {
                serde_json::json!({
                    "memory_entry_id": c.entry.id.to_string(),
                    "vertical_id": &c.vertical_id,
                    "skill_name": &c.candidate.skill_name,
                    "problem": c.candidate.problem.as_deref(),
                    "reusable_pattern": c.candidate.reusable_pattern.as_deref(),
                    "evidence": c.candidate.evidence.as_deref(),
                    "expected_reuse": c.candidate.expected_reuse.as_deref(),
                    "risk": c.candidate.risk.as_deref(),
                    "learned_in": c.entry.learned_in.map(|sid| sid.0.to_string()),
                    "learned_at": c.entry.learned_at.to_rfc3339(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "count": pending_arr.len(),
                "pending": pending_arr,
                "memory_candidates_count": candidate_arr.len(),
                "memory_candidates": candidate_arr,
            }))
            .unwrap_or_default()
        );
        return Ok(());
    }

    if pending.is_empty() && memory_candidates.is_empty() {
        println!("(no skills awaiting review)");
        return Ok(());
    }
    if !pending.is_empty() {
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
        println!("show:    argo skill review show <id>");
        println!("approve: argo skill review approve <id>");
        println!("reject:  argo skill review reject <id>");
    }
    if !memory_candidates.is_empty() {
        println!();
        println!(
            "memory-backed candidates — {} entry/entries (#697):",
            memory_candidates.len()
        );
        for c in &memory_candidates {
            println!(
                "  [{}] {} — vertical={}, learned_at={}",
                c.entry.id,
                c.candidate.skill_name,
                c.vertical_id,
                c.entry.learned_at.to_rfc3339()
            );
            if let Some(p) = &c.candidate.problem {
                println!("        problem: {p}");
            }
        }
        println!();
        println!("promote: argo skill review promote <memory_entry_id>");
    }
    Ok(())
}

async fn show(skills: &SkillStore, skill_id_str: &str, json: bool) -> ArgoResult<()> {
    let skill_id = parse_skill_id(skill_id_str)?;
    let skill = lookup_by_id(skills, skill_id).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&skill_detail_json(&skill)).unwrap_or_default()
        );
        return Ok(());
    }

    let learned = skill
        .learned_in
        .map(|sid| sid.0.to_string())
        .unwrap_or_else(|| "(none)".into());
    let vert = skill.vertical_id.as_deref().unwrap_or("(none)");
    println!("id: {}", skill.id.0);
    println!("name: {}", skill.frontmatter.name);
    println!("version: {}", skill.frontmatter.version);
    println!("state: {}", skill.lifecycle_state.label());
    println!("vertical: {vert}");
    println!("originating_session: {learned}");
    println!(
        "provenance: source={}, hash={}",
        skill.frontmatter.provenance.source, skill.frontmatter.provenance.hash
    );
    let trajectory = extract_trajectory(&skill.body);
    println!("trajectory:");
    if trajectory.is_empty() {
        println!("  (none)");
    } else {
        for step in trajectory {
            println!("  {step}");
        }
    }
    println!();
    println!("{}", skill.body);
    Ok(())
}

async fn transition(
    skills: &SkillStore,
    skill_id_str: &str,
    next: LifecycleState,
    note: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let skill_id = parse_skill_id(skill_id_str)?;

    // Read the current row so we can build a precise audit
    // payload (name, vertical_id, learned_in) — surfacing them
    // makes the chain row self-explanatory without a join.
    let pre = lookup_by_id(skills, skill_id).await?;
    let new_state = skills.transition_lifecycle(skill_id, next).await?;

    // Emit the appropriate audit row through the same store
    // the SkillStore wraps. The session_id we attribute to is
    // `pre.learned_in` so the chain row lands in the originating
    // session's per-session audit timeline. Skill rows have no
    // reviewed_at / reviewed_by columns today, so reviewer data
    // is kept in this append-only audit payload.
    let audit = AuditSink::new(skills.store_arc());
    let payload = serde_json::json!({
        "skill_id": skill_id.0.to_string(),
        "name": &pre.frontmatter.name,
        "version": &pre.frontmatter.version,
        "vertical_id": pre.vertical_id.as_deref(),
        "to": new_state.label(),
        "reviewed_by": "cli",
        "note": note,
    });
    let kind = match new_state {
        LifecycleState::Active => AuditKind::SkillApproved,
        LifecycleState::Rejected => AuditKind::SkillRejected,
        _ => AuditKind::SkillApproved,
    };
    audit.write(pre.learned_in, kind, payload.clone()).await?;

    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "ok": true,
                "skill_id": skill_id.0.to_string(),
                "name": &pre.frontmatter.name,
                "to": new_state.label(),
                "note": note,
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
            "{verb} skill '{}' v{} -> {}",
            pre.frontmatter.name,
            pre.frontmatter.version,
            new_state.label()
        );
    }
    Ok(())
}

fn parse_skill_id(skill_id_str: &str) -> ArgoResult<SkillId> {
    let uuid = Uuid::parse_str(skill_id_str)
        .map_err(|e| ArgoError::Provider(format!("invalid skill id '{skill_id_str}': {e}")))?;
    Ok(SkillId(uuid))
}

fn skill_summary_json(s: &Skill) -> serde_json::Value {
    serde_json::json!({
        "id": s.id.0.to_string(),
        "name": &s.frontmatter.name,
        "version": &s.frontmatter.version,
        "vertical_id": s.vertical_id.as_deref(),
        "learned_in": s.learned_in.map(|sid| sid.0.to_string()),
        "created_at": s.created_at.to_rfc3339(),
    })
}

fn skill_detail_json(s: &Skill) -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "id": s.id.0.to_string(),
        "name": &s.frontmatter.name,
        "version": &s.frontmatter.version,
        "description": &s.frontmatter.description,
        "vertical_id": s.vertical_id.as_deref(),
        "lifecycle_state": s.lifecycle_state.label(),
        "originating_session": s.learned_in.map(|sid| sid.0.to_string()),
        "created_at": s.created_at.to_rfc3339(),
        "provenance": &s.frontmatter.provenance,
        "trajectory": extract_trajectory(&s.body),
        "body": &s.body,
    })
}

fn extract_trajectory(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_section = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            in_section = trimmed.eq_ignore_ascii_case("## Trajectory");
            continue;
        }
        if in_section && !trimmed.is_empty() {
            out.push(trimmed.to_string());
        }
    }
    out
}

/// Per-id lookup helper. The SkillStore doesn't have a public
/// `get_by_id`, so we walk the pending-review list first and
/// then fall back to the non-superseded list. Bounded by table
/// size; review traffic is low-volume.
async fn lookup_by_id(skills: &SkillStore, target: SkillId) -> ArgoResult<Skill> {
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
