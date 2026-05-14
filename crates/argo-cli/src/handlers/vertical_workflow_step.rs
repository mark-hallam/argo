//! Per-step runner extracted from `vertical_workflow.rs` to keep
//! that file under the 500-LOC HARD RULE 6 ceiling.
//!
//! Loaded via `#[path = "vertical_workflow_step.rs"] mod step;`
//! from `vertical_workflow.rs`, mirroring the
//! `turn.rs` / `turn_helpers.rs` sibling pattern.
//!
//! No public API changes: items are `pub(super)` so only the
//! parent module sees them.
use argo_audit::{AuditKind, AuditSink};
use argo_capability::CapabilityEngine;
use argo_store::SessionStore;
use argo_tool::{ToolContext, ToolRegistry};
use argo_types::{ArgoError, ArgoResult, Principal, SessionId, ToolCallId};
use argo_vertical::WorkflowStep;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Per-step audit context. When the runner is invoked with a
/// session id (#668), every dispatched / blocked / paused step
/// emits the same `ToolCall` / `ToolResult` / `RbacDeny` /
/// `EscalationRequired` audit rows that the agent runner emits,
/// so `argo audit show --session <id>` shows workflow activity
/// alongside agent turns.
pub(super) struct WorkflowAuditCtx {
    audit: AuditSink,
    session_id: SessionId,
}

pub(super) struct StepOutcome {
    pub(super) status: String,
    pub(super) message: String,
}

// 8 args is intentional here — workflow steps need access to
// the registry, engine, principal, cwd, both ACL sets, and the
// audit ctx. Bundling them into a struct would just push the
// same arity into a struct definition without removing the
// coupling, so we accept the lint locally.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_one_step(
    step: &WorkflowStep,
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
    cwd: &Path,
    denied: &std::collections::HashSet<String>,
    needs_approval: &std::collections::HashSet<String>,
    audit_ctx: Option<&WorkflowAuditCtx>,
) -> StepOutcome {
    match step.kind.as_str() {
        "tool_call" => {
            run_tool_call_step(
                step,
                registry,
                engine,
                principal,
                cwd,
                denied,
                needs_approval,
                audit_ctx,
            )
            .await
        }
        "skill_invoke" => StepOutcome {
            status: "deferred_no_llm".into(),
            message: format!(
                "skill_invoke '{}' deferred — needs the agent runner's LLM transport (PR #659 ships tool_call only)",
                step.skill.as_deref().unwrap_or("")
            ),
        },
        "prompt" => StepOutcome {
            status: "deferred_no_llm".into(),
            message: "prompt step deferred — needs the agent runner's LLM transport (PR #659 ships tool_call only)".into(),
        },
        other => StepOutcome {
            status: "failed".into(),
            message: format!("unknown step kind '{other}'"),
        },
    }
}

// 8 args mirrors `run_one_step` for the same reason.
#[allow(clippy::too_many_arguments)]
async fn run_tool_call_step(
    step: &WorkflowStep,
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
    cwd: &Path,
    denied: &std::collections::HashSet<String>,
    needs_approval: &std::collections::HashSet<String>,
    audit_ctx: Option<&WorkflowAuditCtx>,
) -> StepOutcome {
    let Some(tool_name) = step.tool.as_deref() else {
        return StepOutcome {
            status: "failed".into(),
            message: "tool_call step missing `tool` field".into(),
        };
    };
    let call_id = ToolCallId::new();

    // Deny-wins (#655) — emit RbacDeny when bound to a session
    // so the chain is identical to the agent runner's deny path.
    if denied.contains(tool_name) {
        emit_audit(
            audit_ctx,
            AuditKind::RbacDeny,
            serde_json::json!({
                "tool": tool_name,
                "call_id": call_id,
                "denied_by": "vertical_manifest",
                "source": "workflow_runner",
            }),
        )
        .await;
        return StepOutcome {
            status: "denied_by_vertical".into(),
            message: format!(
                "tool '{tool_name}' is in the bound vertical's [tools].denied list"
            ),
        };
    }
    // Approval gate (#660) — emit EscalationRequired when bound
    // to a session so an operator can correlate with the
    // approve/revoke audit kinds.
    if needs_approval.contains(tool_name) {
        emit_audit(
            audit_ctx,
            AuditKind::EscalationRequired,
            serde_json::json!({
                "tool": tool_name,
                "call_id": call_id,
                "reason": "human_approval_required",
                "source": "workflow_runner",
            }),
        )
        .await;
        return StepOutcome {
            status: "escalation_required".into(),
            message: format!(
                "tool '{tool_name}' is in the bound vertical's [escalation].human_approval_required list — workflow paused for human approval"
            ),
        };
    }
    // Emit ToolCall before dispatch + ToolResult after, mirroring
    // the agent runner's per-call audit shape.
    emit_audit(
        audit_ctx,
        AuditKind::ToolCall,
        serde_json::json!({
            "tool": tool_name,
            "call_id": call_id,
            "source": "workflow_runner",
        }),
    )
    .await;
    let args = step.args.clone().unwrap_or(serde_json::Value::Null);
    let mut ctx = ToolContext::new(principal.clone(), cwd.to_path_buf());
    ctx.call_id = call_id;
    let result = registry.dispatch(tool_name, args, &ctx, engine).await;
    let (is_error, duration_ms) = match &result {
        Ok(r) => (r.is_error, r.duration_ms),
        Err(_) => (true, 0),
    };
    emit_audit(
        audit_ctx,
        AuditKind::ToolResult,
        serde_json::json!({
            "tool": tool_name,
            "call_id": call_id,
            "is_error": is_error,
            "duration_ms": duration_ms,
            "source": "workflow_runner",
        }),
    )
    .await;
    match result {
        Ok(r) if r.is_error => StepOutcome {
            status: "failed".into(),
            message: tool_result_message(&r),
        },
        Ok(r) => StepOutcome {
            status: "ok".into(),
            message: tool_result_message(&r),
        },
        Err(e) => StepOutcome {
            status: "failed".into(),
            message: format!("dispatch '{tool_name}' errored: {e}"),
        },
    }
}

async fn emit_audit(
    audit_ctx: Option<&WorkflowAuditCtx>,
    kind: AuditKind,
    payload: serde_json::Value,
) {
    let Some(ctx) = audit_ctx else {
        return;
    };
    // Best-effort — a failed audit write must not abort the
    // workflow. The chain verifier will flag any breaks.
    let _ = ctx.audit.write(Some(ctx.session_id), kind, payload, None).await;
}

pub(super) async fn open_audit_ctx(session_id_str: &str) -> ArgoResult<WorkflowAuditCtx> {
    use crate::home::{db_path, ensure_dir, resolve_home};
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home).to_string_lossy().as_ref()).await?;
    store.migrate().await?;
    let store = Arc::new(store);
    let uuid = Uuid::parse_str(session_id_str).map_err(|e| {
        ArgoError::Provider(format!(
            "invalid session id '{session_id_str}': {e}"
        ))
    })?;
    let sid = SessionId(uuid);
    if store.get_session(sid).await?.is_none() {
        return Err(ArgoError::Provider(format!(
            "no session with id '{session_id_str}'"
        )));
    }
    let audit = AuditSink::new(store);
    Ok(WorkflowAuditCtx {
        audit,
        session_id: sid,
    })
}

fn tool_result_message(r: &argo_types::ToolCallResult) -> String {
    // Take a small preview from the first text content block —
    // enough for the operator to confirm the tool returned what
    // they expected without dumping the full output.
    use argo_types::ContentBlock;
    for block in &r.content {
        if let ContentBlock::Text { text } = block {
            let preview: String = text.chars().take(120).collect();
            return preview.replace('\n', " ");
        }
    }
    String::new()
}
