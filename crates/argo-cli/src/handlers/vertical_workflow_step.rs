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
use argo_types::{ArgoError, ArgoResult, Principal, SessionId, TenantId, ToolCallId};
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
    pub(super) audit: AuditSink,
    pub(super) session_id: SessionId,
    pub(super) tenant_id: TenantId,
    /// DEC-025 #2 Genesis P3 — `true` when the session's bound worker is
    /// still inside its probation window. Resolved once at
    /// [`open_audit_ctx`] off the SAME session+worker read; while it
    /// holds, the workflow-step runner holds EVERY `tool_call` step
    /// pending (mirrors the agent runner's `ProbationHold` gate so the
    /// workflow-driven path can't bypass probation).
    pub(super) probation_active: bool,
}

/// Best-effort audit emission shared with sibling step-kind
/// runners (e.g. `skill_step::run_skill_invoke_step`). A failed
/// audit write must not abort the workflow; the chain verifier
/// flags any breaks at read time.
pub(super) async fn emit_workflow_audit(
    audit_ctx: Option<&WorkflowAuditCtx>,
    kind: AuditKind,
    payload: serde_json::Value,
) {
    emit_audit(audit_ctx, kind, payload).await;
}

pub(super) struct StepOutcome {
    pub(super) status: String,
    pub(super) message: String,
}

// 9 args is intentional here — workflow steps need access to
// the registry, engine, principal, cwd, both ACL sets, the audit
// ctx, and (since mission-pm-vertical-workflow-skill-invoke) the
// workflow name for the `WorkflowStepStarted`/`Completed` audit
// payloads. Bundling them into a struct would just push the same
// arity into a struct definition without removing the coupling,
// so we accept the lint locally.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_one_step(
    step: &WorkflowStep,
    workflow_name: &str,
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
        "skill_invoke" => {
            super::skill_step::run_skill_invoke_step(step, workflow_name, cwd, audit_ctx).await
        }
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
            message: format!("tool '{tool_name}' is in the bound vertical's [tools].denied list"),
        };
    }
    // DEC-025 #2 Genesis P3 — enforced probation on the workflow-driven
    // path. A freshly-hired Genesis teammate must NOT execute actions
    // unsupervised through a workflow either: while its window holds,
    // EVERY tool_call step is held pending (emits `ProbationHold`,
    // mirroring the agent runner's gate). This closes the
    // "workflow-driven teammate takes a different path" bypass the design
    // doc calls out. There is no per-session grant on this batch path, so
    // the hold stands until the window clears.
    if audit_ctx.is_some_and(|c| c.probation_active) {
        emit_audit(
            audit_ctx,
            AuditKind::ProbationHold,
            serde_json::json!({
                "tool": tool_name,
                "call_id": call_id,
                "reason": "probation_window_active",
                "source": "workflow_runner",
            }),
        )
        .await;
        return StepOutcome {
            status: "probation_hold".into(),
            message: format!(
                "tool '{tool_name}' held: this teammate is in its Genesis probation window — every action is approval-gated until probation ends"
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
    let tenant_id = match workflow_tenant(cwd, audit_ctx).await {
        Ok(t) => t,
        Err(e) => {
            return StepOutcome {
                status: "failed".into(),
                message: format!("resolve workflow tenant: {e}"),
            };
        }
    };
    let mut ctx = ToolContext::new_with_tenant(principal.clone(), cwd.to_path_buf(), tenant_id);
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
    let _ = ctx
        .audit
        .write(Some(ctx.session_id), kind, payload, None)
        .await;
}

async fn workflow_tenant(cwd: &Path, audit_ctx: Option<&WorkflowAuditCtx>) -> ArgoResult<TenantId> {
    if let Some(ctx) = audit_ctx {
        return Ok(ctx.tenant_id);
    }
    use crate::home::{db_path, ensure_dir, resolve_home};
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home).to_string_lossy().as_ref()).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _workspace) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

pub(super) async fn open_audit_ctx(session_id_str: &str) -> ArgoResult<WorkflowAuditCtx> {
    use crate::home::{db_path, ensure_dir, resolve_home};
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home).to_string_lossy().as_ref()).await?;
    store.migrate().await?;
    let store = Arc::new(store);
    let uuid = Uuid::parse_str(session_id_str)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{session_id_str}': {e}")))?;
    let sid = SessionId(uuid);
    let Some(session) = store.get_session(sid).await? else {
        return Err(ArgoError::Provider(format!(
            "no session with id '{session_id_str}'"
        )));
    };
    // DEC-025 #2 Genesis P3 — resolve probation off the session's bound
    // worker using the SAME store we just opened (no extra DB connection).
    // A read failure / missing worker degrades to `false` (no probation)
    // — same fail-open-on-unreadable-row posture as the agent runner's
    // fence, flagged for the adversarial pass.
    let probation_active = probation_active_for_session(&store, &session).await;
    let audit = AuditSink::new_with_tenant(store, session.tenant_id.to_string());
    Ok(WorkflowAuditCtx {
        audit,
        session_id: sid,
        tenant_id: session.tenant_id,
        probation_active,
    })
}

/// Resolve whether the session's bound worker is inside its Genesis P3
/// probation window, reading the worker row off the supplied store. Any
/// no-worker / missing-row / store-error path yields `false` (no
/// probation), matching the agent runner's
/// `genesis_signals_for_worker` degradation.
async fn probation_active_for_session(
    store: &Arc<SessionStore>,
    session: &argo_store::SessionHeader,
) -> bool {
    let Some(worker_id) = session.worker_id else {
        return false;
    };
    let identities = argo_identity::IdentityStore::new(store);
    match identities.get_worker(session.tenant_id, worker_id).await {
        Ok(Some(worker)) => {
            argo_identity::is_in_probation(&worker.persona.provenance_chain, chrono::Utc::now())
        }
        _ => false,
    }
}

impl WorkflowAuditCtx {
    /// Test-only constructor: build a session-backed audit ctx with the
    /// probation flag forced, so the workflow-step probation gate can be
    /// exercised without driving a full Genesis hire. Compiled out of
    /// release builds.
    #[cfg(test)]
    pub(super) async fn for_test(probation_active: bool) -> Self {
        let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
        store.migrate().await.expect("migrate");
        let (tenant, workspace) = store
            .ensure_workspace("wf-probation", "/tmp/wf-probation")
            .await
            .expect("ensure_workspace");
        let session_id = store
            .create_session(tenant, workspace, "m", "p", None, None)
            .await
            .expect("create session");
        let audit = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
        Self {
            audit,
            session_id,
            tenant_id: tenant,
            probation_active,
        }
    }
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
