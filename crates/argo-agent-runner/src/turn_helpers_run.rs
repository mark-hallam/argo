//! Per-round helpers extracted from
//! [`super::turn_helpers::run_turn_with_tools`] to keep that
//! orchestrator under the 80-LOC budget. Each helper owns one
//! phase of the tool-dispatch loop.
//!
//! No public API changes — every symbol here is `pub(super)` and
//! only consumed by `turn_helpers::run_turn_with_tools`.

use super::turn_skills_runtime::PendingAbOutcome;
use super::{build_system_prompt_with_ab, ContextWindowPolicy, TurnResult};
use crate::dispatch::{build_dispatch_ctx, dispatch_one_with_tenant, provider_schemas_filtered};
use crate::helpers::{
    build_assistant_content, emit_event, record_turn_spend, stream_one_round, RoundResp,
    RunEventSink, SpendCtx,
};
use crate::transport::load_manifest;
use crate::turn_vertical::{consume_one_shot_if_present, SessionGrantState};
use argo_audit::{AuditKind, AuditSink};
use argo_capability::CapabilityEngine;
use argo_identity::{MemoryNs, MemoryScope};
use argo_provider::{ProviderRequest, ToolSchema, Transport};
use argo_store::SessionStore;
use argo_tool::ToolRegistry;
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, MessageRole, Principal, SessionId, ToolCallRequest,
    ToolCallResult,
};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Per-turn dispatch state — built once per call to
/// `run_turn_with_tools` and then reused across rounds.
pub(super) struct TurnDispatchCtx {
    pub(super) registry: ToolRegistry,
    pub(super) engine: CapabilityEngine,
    pub(super) principal: Principal,
    pub(super) tools: Vec<ToolSchema>,
    pub(super) system_prompt: Option<String>,
    pub(super) context_policy: ContextWindowPolicy,
    pub(super) orch_session_id: argo_orchestrator::SessionId,
}

/// Assemble the dispatch registry, system prompt, tools, and
/// context-window policy for `run_turn_with_tools`. Pulls in the
/// `MemoryStore`, optional per-worker memory namespace, the
/// orchestrator store, and any vertical prompt prefix.
#[allow(clippy::too_many_arguments)]
pub(super) async fn build_turn_dispatch_ctx(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    model: &str,
    spend: &SpendCtx<'_>,
    vertical_prompt: Option<&str>,
    vertical_scope: Option<&str>,
    vertical_deny: &std::collections::HashSet<String>,
    worker_id: Option<argo_store::WorkerId>,
    invocation_id: &str,
    pending_ab_outcomes: &mut Vec<PendingAbOutcome>,
) -> ArgoResult<TurnDispatchCtx> {
    let session = store
        .get_session(session_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {}", session_id.0)))?;
    let context_policy = match load_manifest(cwd, &session.provider) {
        Ok(manifest) => ContextWindowPolicy::from_config(
            super::load_context_compress_config(cwd)?,
            super::model_context_window(&manifest, model),
        ),
        Err(_) => ContextWindowPolicy::disabled(),
    };
    // claude-memory-tool-wire: thread the live MemoryStore +
    // workspace through dispatch so the in-core `memory` tool can
    // actually persist. Vertical-bound writes go through
    // `append_for_vertical` when `vertical_scope` is set; otherwise
    // writes stay workspace-global.
    let memory_ctx = store.first_workspace().await?.map(|ws| {
        (
            Arc::new(argo_memory::MemoryStore::new(store.clone())),
            ws,
            vertical_scope.map(str::to_string),
        )
    });
    // codex-memory-tool-namespace-wiring (F-03 P1): when the
    // session carries a `worker_id`, bind the in-core `memory` tool
    // to a per-worker `MemoryNs` so the `query_scored` / `*_team`
    // actions address the tenant-scoped namespace store.
    let memory_namespace =
        worker_id.map(|wid| MemoryNs::new(session.tenant_id, wid, MemoryScope::WorkerSelf));
    let orchestrator_ctx = Arc::new(argo_orchestrator::OrchestratorStore::new(store.clone()));
    let orch_session_id = argo_orchestrator::SessionId::from_string(session_id.to_string());
    let (registry, engine, principal) = build_dispatch_ctx(
        cwd,
        memory_ctx,
        memory_namespace,
        orchestrator_ctx,
        Arc::clone(store),
    )?;
    // #721: hide manifest-denied tools from the model's tool list.
    let tools = provider_schemas_filtered(&registry, vertical_deny);
    let (mut system_prompt, ab_pending) = build_system_prompt_with_ab(
        store,
        session_id,
        vertical_scope,
        spend.tenant_id,
        worker_id,
        invocation_id,
    )
    .await?;
    pending_ab_outcomes.extend(ab_pending);
    if let Some(vp) = vertical_prompt {
        let merged = match system_prompt.take() {
            Some(skill_prompt) => format!("{vp}\n\n---\n\n{skill_prompt}"),
            None => vp.to_string(),
        };
        system_prompt = Some(merged);
    }
    Ok(TurnDispatchCtx {
        registry,
        engine,
        principal,
        tools,
        system_prompt,
        context_policy,
        orch_session_id,
    })
}

/// Outcome of a single provider call — either the assistant gave a
/// final text response (no tool calls) or it issued one-or-more
/// tool calls that need dispatching.
pub(super) struct RoundOutcome {
    pub(super) assistant_id: Uuid,
    pub(super) text: String,
    pub(super) tokens_in: Option<u32>,
    pub(super) tokens_out: Option<u32>,
    pub(super) tool_calls: Vec<ToolCallRequest>,
}

/// Issue one provider round: call the transport (streamed or
/// blocking), persist the assistant message, write the audit/event
/// pair, and record spend. Returns the parsed
/// [`RoundOutcome`] for the caller to inspect.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_one_round(
    store: &Arc<SessionStore>,
    transport: &dyn Transport,
    session_id: SessionId,
    audit: &AuditSink,
    spend: &SpendCtx<'_>,
    events: Option<&RunEventSink>,
    req: ProviderRequest,
    stream: bool,
    round: usize,
) -> ArgoResult<RoundOutcome> {
    let resp = if stream {
        stream_one_round(transport, &req).await?
    } else {
        let r = transport.complete(&req).await?;
        RoundResp {
            text: r.text.unwrap_or_default(),
            tool_calls: r.tool_calls,
            tokens_in: r.tokens_in,
            tokens_out: r.tokens_out,
        }
    };
    let assistant_content = build_assistant_content(&resp.text, &resp.tool_calls);
    let assistant_id = store
        .append_message(
            session_id,
            MessageRole::Assistant,
            &assistant_content.to_string(),
        )
        .await?;
    let assistant_payload = serde_json::json!({
        "message_id": assistant_id,
        "round": round,
        "tool_calls": resp.tool_calls.len(),
        "tokens_in": resp.tokens_in,
        "tokens_out": resp.tokens_out,
    });
    audit
        .write(
            Some(session_id),
            AuditKind::AssistantMessage,
            assistant_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "AssistantMessage", assistant_payload);
    record_turn_spend(store, session_id, spend, resp.tokens_in, resp.tokens_out).await?;
    crate::turn_budget::record_subtask_turn_spend(
        store,
        session_id,
        spend,
        resp.tokens_in,
        resp.tokens_out,
    )
    .await?;
    Ok(RoundOutcome {
        assistant_id,
        text: resp.text,
        tokens_in: resp.tokens_in,
        tokens_out: resp.tokens_out,
        tool_calls: resp.tool_calls,
    })
}

/// Dispatch a single tool call through the deny/approval/grant
/// gates and persist the audit + ToolResult message. Mutates
/// `session_grants` when a one-shot grant is consumed.
#[allow(clippy::too_many_arguments)]
pub(super) async fn dispatch_one_tool_call(
    ctx: &TurnDispatchCtx,
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    audit: &AuditSink,
    spend: &SpendCtx<'_>,
    events: Option<&RunEventSink>,
    vertical_deny: &std::collections::HashSet<String>,
    vertical_approval: &std::collections::HashSet<String>,
    session_grants: &mut SessionGrantState,
    call: &ToolCallRequest,
) -> ArgoResult<()> {
    let call_payload = serde_json::json!({"tool": call.name, "call_id": call.id});
    audit
        .write(
            Some(session_id),
            AuditKind::ToolCall,
            call_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "ToolCall", call_payload);
    let result = resolve_tool_call(
        ctx,
        store,
        cwd,
        session_id,
        audit,
        spend,
        events,
        vertical_deny,
        vertical_approval,
        session_grants,
        call,
    )
    .await?;
    let result_payload = serde_json::json!({
        "tool": call.name,
        "call_id": call.id,
        "is_error": result.is_error,
        "duration_ms": result.duration_ms,
    });
    audit
        .write(
            Some(session_id),
            AuditKind::ToolResult,
            result_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "ToolResult", result_payload);
    let tool_msg = serde_json::json!([ContentBlock::ToolResult(result)]);
    store
        .append_message(session_id, MessageRole::Tool, &tool_msg.to_string())
        .await?;
    Ok(())
}

/// Apply the deny-wins → escalation-gate → grant-override → dispatch
/// flow. Deny is the platform's hard "never"; the escalation gate
/// pauses for human approval unless the session already has a
/// matching grant (one-shot grants consume after dispatch).
#[allow(clippy::too_many_arguments)]
async fn resolve_tool_call(
    ctx: &TurnDispatchCtx,
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    audit: &AuditSink,
    spend: &SpendCtx<'_>,
    events: Option<&RunEventSink>,
    vertical_deny: &std::collections::HashSet<String>,
    vertical_approval: &std::collections::HashSet<String>,
    session_grants: &mut SessionGrantState,
    call: &ToolCallRequest,
) -> ArgoResult<ToolCallResult> {
    if vertical_deny.contains(&call.name) {
        return emit_deny(session_id, audit, events, call).await;
    }
    if vertical_approval.contains(&call.name) && !session_grants.allows(&call.name) {
        return emit_escalation_required(session_id, audit, events, call).await;
    }
    let approved_via_grant =
        vertical_approval.contains(&call.name) && session_grants.allows(&call.name);
    let res = dispatch_one_with_tenant(
        &ctx.registry,
        &ctx.engine,
        &ctx.principal,
        spend.tenant_id,
        cwd,
        &ctx.orch_session_id,
        call,
    )
    .await;
    if approved_via_grant {
        consume_one_shot_if_present(
            store,
            session_id,
            &call.name,
            session_grants,
            audit,
            events,
        )
        .await?;
    }
    Ok(res)
}

async fn emit_deny(
    session_id: SessionId,
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    call: &ToolCallRequest,
) -> ArgoResult<ToolCallResult> {
    let deny_payload = serde_json::json!({
        "tool": call.name,
        "call_id": call.id,
        "denied_by": "vertical_manifest",
    });
    audit
        .write(
            Some(session_id),
            AuditKind::RbacDeny,
            deny_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "RbacDeny", deny_payload);
    Ok(argo_tool::error_text(
        call.id,
        format!(
            "tool '{}' denied by the bound vertical's manifest [tools].denied list",
            call.name
        ),
        0,
    ))
}

async fn emit_escalation_required(
    session_id: SessionId,
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    call: &ToolCallRequest,
) -> ArgoResult<ToolCallResult> {
    let escalation_payload = serde_json::json!({
        "tool": call.name,
        "call_id": call.id,
        "reason": "human_approval_required",
        "source": "vertical_manifest",
    });
    audit
        .write(
            Some(session_id),
            AuditKind::EscalationRequired,
            escalation_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "EscalationRequired", escalation_payload);
    Ok(argo_tool::error_text(
        call.id,
        format!(
            "tool '{}' requires human approval (declared in the bound vertical's manifest [escalation].human_approval_required); run `argo session approve <session-id> --tool {}` (add --once for a single-dispatch grant) and re-issue the turn",
            call.name, call.name
        ),
        0,
    ))
}

/// Emit the `Halt` audit + event when the round-budget is reached.
/// Caller still owns building the final [`TurnResult`].
pub(super) async fn emit_halt(
    session_id: SessionId,
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    max_rounds: usize,
) -> ArgoResult<()> {
    let halt_payload = serde_json::json!({
        "reason": "tool_budget_exhausted",
        "max_rounds": max_rounds,
    });
    audit
        .write(
            Some(session_id),
            AuditKind::Halt,
            halt_payload.clone(),
            None,
        )
        .await?;
    emit_event(events, "Halt", halt_payload);
    Ok(())
}

/// Wrap the round outcome's text/tokens into the no-tool-call exit
/// [`TurnResult`] so the parent doesn't have to repeat the struct
/// literal in two places.
pub(super) fn finalize_turn(outcome: RoundOutcome) -> TurnResult {
    TurnResult {
        assistant_id: outcome.assistant_id,
        text: outcome.text,
        tokens_in: outcome.tokens_in,
        tokens_out: outcome.tokens_out,
        stopped_on_tool_budget: false,
    }
}
