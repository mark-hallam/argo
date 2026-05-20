//! Per-step runner for `skill_invoke` vertical-workflow steps
//! (mission-pm-vertical-workflow-skill-invoke, P1 #8).
//!
//! Loaded as a sibling of `vertical_workflow_step.rs` from
//! `vertical_workflow.rs` (`#[path] mod skill_step;`). Kept in its
//! own file so the `tool_call` / `prompt` paths in
//! `vertical_workflow_step.rs` stay byte-stable and so the agent-
//! runner invocation logic doesn't push that file toward the
//! 500-LOC ceiling.
//!
//! Mechanic: when a `skill_invoke` step fires, build a one-shot
//! [`argo_agent_runner::run_turn_with_events`] invocation under the
//! workflow's bound session (`--session`), with the named skill
//! attached so it lands in the system prompt for that turn. Hard
//! cap of [`SKILL_INVOKE_MAX_STEPS`] tool-dispatch rounds; the
//! agent runner returns `stopped_on_tool_budget = true` when the
//! cap is hit, which becomes a `"failed"` step. Final-content text
//! is captured into `StepOutcome::message`; an empty turn (the
//! model exited after a tool result rather than emitting assistant
//! text) is also surfaced as `"failed"` per the brief.
use super::step::{emit_workflow_audit, StepOutcome, WorkflowAuditCtx};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_agent_runner::run_turn_with_events;
use argo_audit::AuditKind;
use argo_skill::{LayeredLookup, Skill, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use argo_vertical::WorkflowStep;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;

/// Hard cap on tool-dispatch rounds within a single `skill_invoke`
/// step. Matches [`argo_agent_runner::MAX_TOOL_ROUNDS`] so the
/// hard "max steps" envelope an operator sees in a workflow run
/// is the same as the one a free-standing agent turn lives under.
pub(super) const SKILL_INVOKE_MAX_STEPS: usize = 8;

/// Drive one `skill_invoke` step.
///
/// `audit_ctx` carries the session id + tenant id that the
/// agent-runner invocation runs under; without it the step has no
/// session to bind to and we surface a clear failure rather than
/// silently mint an ephemeral session (operators wire workflows
/// through `argo vertical run`, which always provides a session).
pub(super) async fn run_skill_invoke_step(
    step: &WorkflowStep,
    workflow_name: &str,
    cwd: &Path,
    audit_ctx: Option<&WorkflowAuditCtx>,
) -> StepOutcome {
    let step_name = step.name.as_str();
    let skill_name = step.skill.as_deref().unwrap_or("");

    emit_workflow_audit(
        audit_ctx,
        AuditKind::WorkflowStepStarted,
        serde_json::json!({
            "workflow_name": workflow_name,
            "step_name": step_name,
            "skill_name": skill_name,
        }),
    )
    .await;

    if step.skill.is_none() || skill_name.is_empty() {
        return finalize_step(
            audit_ctx,
            workflow_name,
            step_name,
            skill_name,
            None,
            "failed".into(),
            "skill_invoke step missing `skill` field".into(),
        )
        .await;
    }

    let Some(ctx) = audit_ctx else {
        return finalize_step(
            audit_ctx,
            workflow_name,
            step_name,
            skill_name,
            None,
            "failed".into(),
            "skill_invoke requires a session (run via `argo vertical run` or pass --session)".into(),
        )
        .await;
    };

    let store = match open_session_store(cwd).await {
        Ok(s) => s,
        Err(e) => {
            return finalize_step(
                audit_ctx,
                workflow_name,
                step_name,
                skill_name,
                None,
                "failed".into(),
                format!("open store for skill_invoke: {e}"),
            )
            .await;
        }
    };

    let session_id = ctx.session_id;

    let resolved = match resolve_skill(&store, session_id, skill_name).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return finalize_step(
                audit_ctx,
                workflow_name,
                step_name,
                skill_name,
                None,
                "failed".into(),
                format!("skill_invoke '{skill_name}' not found"),
            )
            .await;
        }
        Err(e) => {
            return finalize_step(
                audit_ctx,
                workflow_name,
                step_name,
                skill_name,
                None,
                "failed".into(),
                format!("skill_invoke lookup '{skill_name}' errored: {e}"),
            )
            .await;
        }
    };

    let skills = SkillStore::new(Arc::clone(&store));
    if let Err(e) = skills.attach_to_session(session_id, resolved.id).await {
        return finalize_step(
            audit_ctx,
            workflow_name,
            step_name,
            skill_name,
            None,
            "failed".into(),
            format!("attach skill '{skill_name}': {e}"),
        )
        .await;
    }

    let turn = run_turn_with_events(
        &store,
        cwd,
        session_id,
        None,
        false,
        SKILL_INVOKE_MAX_STEPS,
        None,
    )
    .await;

    let (status, message) = match turn {
        Ok(r) if r.stopped_on_tool_budget => (
            "failed".to_string(),
            format!("skill_invoke step exceeded max_steps={SKILL_INVOKE_MAX_STEPS}"),
        ),
        Ok(r) if r.text.is_empty() => (
            "failed".to_string(),
            "skill_invoke ended without assistant text".to_string(),
        ),
        Ok(r) => ("success".to_string(), r.text),
        Err(e) => (
            "failed".to_string(),
            format!("skill_invoke '{skill_name}' turn errored: {e}"),
        ),
    };

    let sha = if status == "success" {
        Some(sha256_hex(&message))
    } else {
        None
    };
    finalize_step(
        audit_ctx,
        workflow_name,
        step_name,
        skill_name,
        sha,
        status,
        message,
    )
    .await
}

async fn finalize_step(
    audit_ctx: Option<&WorkflowAuditCtx>,
    workflow_name: &str,
    step_name: &str,
    skill_name: &str,
    final_text_sha: Option<String>,
    status: String,
    message: String,
) -> StepOutcome {
    emit_workflow_audit(
        audit_ctx,
        AuditKind::WorkflowStepCompleted,
        serde_json::json!({
            "workflow_name": workflow_name,
            "step_name": step_name,
            "skill_name": skill_name,
            "status": status,
            "final_text_sha": final_text_sha,
        }),
    )
    .await;
    StepOutcome { status, message }
}

async fn open_session_store(cwd: &Path) -> ArgoResult<Arc<SessionStore>> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home).to_string_lossy().as_ref()).await?;
    store.migrate().await?;
    Ok(Arc::new(store))
}

/// Resolve a skill name across the active set: workspace + library
/// (via [`LayeredLookup`]) plus a per-worker overlay fallback for
/// skills that exist only as `worker_skill` attachments on the
/// session's bound worker. Returns `Ok(None)` when no layer
/// matches; that's the `"skill_invoke '<name>' not found"` path.
async fn resolve_skill(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    name: &str,
) -> ArgoResult<Option<Skill>> {
    let skills = SkillStore::new(Arc::clone(store));
    let lookup = LayeredLookup::workspace_only(&skills);
    if let Some(layered) = lookup.find(name).await? {
        return Ok(Some(layered.skill));
    }
    let Some(session) = store
        .get_session(session_id)
        .await
        .map_err(|e| ArgoError::Provider(format!("get_session: {e}")))?
    else {
        return Ok(None);
    };
    if let Some(worker_id) = session.worker_id {
        for s in skills.list_for_worker(worker_id).await? {
            if s.frontmatter.name == name {
                return Ok(Some(s));
            }
        }
    }
    Ok(None)
}

fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let bytes = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn sha256_hex_is_lowercase_64_chars() {
        let h = sha256_hex("hello");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(
            h,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
