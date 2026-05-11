//! Inner agent-loop helpers extracted from `turn.rs` to keep that
//! file under the 500-LOC ceiling (HARD RULE 6).
//!
//! Loaded via `#[path = "turn_helpers.rs"] mod turn_helpers;` from
//! `turn.rs`, mirroring the `turn_memory.rs` sibling pattern. The
//! tool-dispatch loop driven by [`run_turn_with_tools`] lives here;
//! the public entry points (`run_turn` / `run_turn_with_events`)
//! and the system-prompt composer remain in `turn.rs`.
//!
//! No public API changes — `run_turn_with_tools` is `pub(super)`
//! and only `turn::run_turn_with_events` calls it.
use super::turn_skills_runtime::PendingAbOutcome;
use super::{build_system_prompt_with_ab, TurnResult};
use crate::dispatch::{build_dispatch_ctx, dispatch_one, provider_schemas_filtered};
use crate::helpers::{
    build_assistant_content, emit_event, message_from_row, record_turn_spend, stream_one_round,
    RoundResp, RunEventSink, SpendCtx,
};
use crate::turn_vertical::{consume_one_shot_if_present, SessionGrantState};
use argo_audit::{AuditKind, AuditSink};
use argo_provider::{ProviderRequest, Sampling, Transport};
use argo_store::SessionStore;
use argo_types::{ArgoResult, ContentBlock, MessageRole, SessionId, ToolCallResult};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_turn_with_tools(
    store: &Arc<SessionStore>,
    transport: &dyn Transport,
    cwd: &Path,
    session_id: SessionId,
    model: &str,
    audit: &AuditSink,
    spend: &SpendCtx<'_>,
    max_rounds: usize,
    stream: bool,
    events: Option<&RunEventSink>,
    vertical_prompt: Option<&str>,
    vertical_scope: Option<&str>,
    vertical_deny: &std::collections::HashSet<String>,
    vertical_approval: &std::collections::HashSet<String>,
    session_grants: SessionGrantState,
    worker_id: Option<argo_store::WorkerId>,
    invocation_id: &str,
    pending_ab_outcomes: &mut Vec<PendingAbOutcome>,
) -> ArgoResult<TurnResult> {
    let mut session_grants = session_grants;
    // claude-memory-tool-wire: thread the live MemoryStore + workspace
    // through dispatch so the in-core `memory` tool can actually
    // persist. Vertical-bound writes go through `append_for_vertical`
    // when `vertical_scope` is set; otherwise writes stay
    // workspace-global.
    let memory_ctx = store.first_workspace().await?.map(|ws| {
        (
            Arc::new(argo_memory::MemoryStore::new(store.clone())),
            ws,
            vertical_scope.map(str::to_string),
        )
    });
    let (registry, engine, principal) = build_dispatch_ctx(cwd, memory_ctx)?;
    // #721: hide manifest-denied tools from the model's tool list
    // entirely — the per-call deny check below remains the hard
    // authoritative guard; this is the discoverability layer that
    // stops the agent wasting turns calling things that will get
    // rejected anyway (observed in the first live worker run).
    let tools = provider_schemas_filtered(&registry, vertical_deny);
    // The vertical_id flows in via the parent `run_turn` call
    // through `vertical_prompt` (when set) — but build_system_prompt
    // needs the raw id, not the prompt body. The agent-runner's
    // ergonomic seam: when we have a `vertical_prompt`, use the
    // explicitly threaded `vertical_scope` value (id-only, no
    // file content). Both come from the same session.vertical_id.
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
    // Vertical-bound session: prepend the vertical's
    // [role].system_prompt content so it lands ahead of skill-
    // derived guidance (#654). Falls through cleanly when the
    // session isn't bound or the file couldn't be read.
    if let Some(vp) = vertical_prompt {
        let merged = match system_prompt.take() {
            Some(skill_prompt) => format!("{vp}\n\n---\n\n{skill_prompt}"),
            None => vp.to_string(),
        };
        system_prompt = Some(merged);
    }

    let mut last_text = String::new();
    let mut last_tokens_in: Option<u32> = None;
    let mut last_tokens_out: Option<u32> = None;
    let mut last_assistant_id = Uuid::nil();

    for round in 0..max_rounds {
        let rows = store.list_messages(session_id).await?;
        let messages = rows
            .iter()
            .map(|r| message_from_row(r, session_id))
            .collect::<ArgoResult<Vec<_>>>()?;

        let req = ProviderRequest {
            model: model.to_string(),
            messages,
            tools: tools.clone(),
            system: system_prompt.clone(),
            sampling: Sampling::default(),
        };
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
        last_text = resp.text.clone();
        last_tokens_in = resp.tokens_in;
        last_tokens_out = resp.tokens_out;

        let assistant_content = build_assistant_content(&resp.text, &resp.tool_calls);
        last_assistant_id = store
            .append_message(
                session_id,
                MessageRole::Assistant,
                &assistant_content.to_string(),
            )
            .await?;
        let assistant_payload = serde_json::json!({
            "message_id": last_assistant_id,
            "round": round,
            "tool_calls": resp.tool_calls.len(),
            "tokens_in": last_tokens_in,
            "tokens_out": last_tokens_out,
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
        record_turn_spend(store, session_id, spend, last_tokens_in, last_tokens_out).await?;

        if resp.tool_calls.is_empty() {
            return Ok(TurnResult {
                assistant_id: last_assistant_id,
                text: last_text,
                tokens_in: last_tokens_in,
                tokens_out: last_tokens_out,
                stopped_on_tool_budget: false,
            });
        }

        for call in &resp.tool_calls {
            let call_payload = serde_json::json!({"tool": call.name, "call_id": call.id});
            audit
                .write(Some(session_id), AuditKind::ToolCall, call_payload.clone(), None)
                .await?;
            emit_event(events, "ToolCall", call_payload);

            // Deny-wins (#655) → escalation gate (#660) →
            // operator grants (#664/#667) → dispatch. Deny is
            // the platform's hard "never"; the gate pauses for
            // human approval; grants override the gate (one-shot
            // grants consume after dispatch in the else branch).
            let result: ToolCallResult = if vertical_deny.contains(&call.name) {
                let deny_payload = serde_json::json!({
                    "tool": call.name,
                    "call_id": call.id,
                    "denied_by": "vertical_manifest",
                });
                audit
                    .write(Some(session_id), AuditKind::RbacDeny, deny_payload.clone(), None)
                    .await?;
                emit_event(events, "RbacDeny", deny_payload);
                argo_tool::error_text(
                    call.id,
                    format!(
                        "tool '{}' denied by the bound vertical's manifest [tools].denied list",
                        call.name
                    ),
                    0,
                )
            } else if vertical_approval.contains(&call.name) && !session_grants.allows(&call.name) {
                // Approval gate (#660) — but if the operator has
                // explicitly approved this tool for this session
                // via `argo session approve` (#664/#667), the
                // grant overrides the gate and we fall through
                // to dispatch.
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
                argo_tool::error_text(
                    call.id,
                    format!(
                        "tool '{}' requires human approval (declared in the bound vertical's manifest [escalation].human_approval_required); run `argo session approve <session-id> --tool {}` (add --once for a single-dispatch grant) and re-issue the turn",
                        call.name, call.name
                    ),
                    0,
                )
            } else {
                // #667: track grant-gate permission so we can
                // consume the one-shot after the dispatch.
                let approved_via_grant =
                    vertical_approval.contains(&call.name) && session_grants.allows(&call.name);
                let res = dispatch_one(&registry, &engine, &principal, cwd, call).await;
                if approved_via_grant {
                    consume_one_shot_if_present(
                        store,
                        session_id,
                        &call.name,
                        &mut session_grants,
                        audit,
                        events,
                    )
                    .await?;
                }
                res
            };
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
        }

        // #724 — when we hit `max_rounds` the for-loop ends
        // naturally; we DON'T return Err. The session state is
        // consistent (every dispatched tool just got its
        // `tool_result` appended above) so the caller can issue
        // another `run_turn` to continue. We surface the cap via
        // the `stopped_on_tool_budget` flag and an audit hint so
        // the outer worker step loop (in argo-cli) can convert
        // this into a clean step boundary rather than treating
        // it as a transport error and stopping the job.
    }
    let halt_payload = serde_json::json!({
        "reason": "tool_budget_exhausted",
        "max_rounds": max_rounds,
    });
    audit
        .write(Some(session_id), AuditKind::Halt, halt_payload.clone(), None)
        .await?;
    emit_event(events, "Halt", halt_payload);
    Ok(TurnResult {
        assistant_id: last_assistant_id,
        text: last_text,
        tokens_in: last_tokens_in,
        tokens_out: last_tokens_out,
        stopped_on_tool_budget: true,
    })
}
