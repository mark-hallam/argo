//! Multi-turn agent loop driver.
//!
//! [`run_turn`] / [`run_turn_with_events`] orchestrate one
//! conversation turn against an open `SessionStore`: optionally
//! append a user message, call the provider, dispatch any tool
//! calls (capability-checked), feed the tool results back, repeat
//! up to `max_rounds`. Each step is hash-chained into the audit
//! log and (when pricing is known) recorded into `spend_ledger`.
//!
//! Streaming (`stream = true`) prints assistant text to stdout as
//! it arrives but otherwise runs the same multi-round loop.
use crate::dispatch::{build_dispatch_ctx, dispatch_one, provider_schemas};
use crate::helpers::{
    build_assistant_content, emit_event, message_from_row, price_for, record_turn_spend,
    stream_one_round, RoundResp, RunEventSink, SpendCtx,
};
use crate::transport::{build_transport, load_manifest};
use crate::turn_vertical::{
    build_session_grant_state, build_turn_start_payload, build_vertical_approval_set,
    build_vertical_deny_set, consume_one_shot_if_present, resolve_vertical_for_session,
    SessionGrantState,
};
use argo_audit::{AuditKind, AuditSink};
use argo_provider::{ProviderRequest, Sampling, Transport};
use argo_store::SessionStore;
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, MessageRole, SessionId, ToolCallResult,
};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Default cap on tool-dispatch rounds within a single `run_turn`
/// when callers don't pass an explicit `max_rounds`. Generous
/// enough for legitimate planning chains; small enough that an
/// accidental loop fails fast.
pub const MAX_TOOL_ROUNDS: usize = 8;

/// Outcome of one agent-loop turn. Consumed by both the CLI
/// handler (which prints) and the gateway's
/// `POST /sessions/<id>/runs` endpoint (which serializes to JSON).
pub struct TurnResult {
    /// Persisted assistant message id.
    pub assistant_id: Uuid,
    /// Full assistant text.
    pub text: String,
    /// Provider-reported input tokens, if any.
    pub tokens_in: Option<u32>,
    /// Provider-reported output tokens, if any.
    pub tokens_out: Option<u32>,
}

/// One agent-loop turn against an open SessionStore.
///
/// Reusable from both the CLI handler and the gateway's
/// `POST /sessions/<id>/runs` endpoint. When `stream = true` the
/// assistant text is flushed to stdout as it arrives — streaming
/// stays single-turn for now (no tool dispatch). Non-streaming
/// runs the full tool-dispatch loop, capped at
/// [`MAX_TOOL_ROUNDS`] rounds.
pub async fn run_turn(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    user: Option<&str>,
    stream: bool,
    max_rounds: usize,
) -> ArgoResult<TurnResult> {
    run_turn_with_events(store, cwd, session_id, user, stream, max_rounds, None).await
}

/// Like `run_turn`, but with an optional `events` sink. The gateway
/// streaming endpoint passes a real `UnboundedSender`; the CLI +
/// non-streaming gateway pass `None`. Each event mirrors the audit
/// emission that just landed.
#[allow(clippy::too_many_arguments)]
pub async fn run_turn_with_events(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    user: Option<&str>,
    stream: bool,
    max_rounds: usize,
    events: Option<&RunEventSink>,
) -> ArgoResult<TurnResult> {
    let session = store
        .get_session(session_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {}", session_id.0)))?;

    if let Some(text) = user {
        let content = serde_json::json!([{"type": "text", "text": text}]);
        store
            .append_message(session_id, MessageRole::User, &content.to_string())
            .await?;
    }

    let manifest = load_manifest(cwd, &session.provider)?;
    let pricing = price_for(&manifest, &session.model);
    let provider_id = manifest.provider.id.clone();
    let transport: Box<dyn Transport> = build_transport(manifest).await?;

    // Vertical lookup: when the session is bound, load the
    // declared role/system_prompt + carry the slug into the audit
    // payload so /audit/by-vertical rollups become possible (#654).
    // Also build the deny-set the dispatcher will consult per-call
    // (#655). Per-vertical load errors degrade gracefully — the
    // turn still runs; the prompt just falls back to skill-derived.
    let vertical_ctx = resolve_vertical_for_session(session.vertical_id.as_deref());
    let vertical_deny = build_vertical_deny_set(session.vertical_id.as_deref());
    let vertical_approval = build_vertical_approval_set(session.vertical_id.as_deref());
    // Operator-issued approvals from `argo session approve`
    // (#664) + one-shot grant tracking (#667). The approval gate
    // lets these through even when the vertical's manifest would
    // have blocked them; one-shot grants are consumed on first
    // matching dispatch so subsequent calls re-block.
    let session_grants = build_session_grant_state(store, session_id).await;

    let audit = AuditSink::new(store.clone());
    let turn_start = build_turn_start_payload(
        &session.provider,
        &session.model,
        session.vertical_id.as_deref(),
    );
    audit
        .write(Some(session_id), AuditKind::TurnStart, turn_start.clone())
        .await?;
    emit_event(events, "TurnStart", turn_start);
    let spend_ctx = SpendCtx {
        tenant_id: session.tenant_id,
        provider: provider_id.as_str(),
        model: session.model.as_str(),
        pricing,
    };
    let result = run_turn_with_tools(
        store,
        transport.as_ref(),
        cwd,
        session_id,
        &session.model,
        &audit,
        &spend_ctx,
        max_rounds,
        stream,
        events,
        vertical_ctx.as_deref(),
        &vertical_deny,
        &vertical_approval,
        session_grants,
    )
    .await;
    let kind = if result.is_ok() {
        AuditKind::TurnEnd
    } else {
        AuditKind::Halt
    };
    let kind_label = if result.is_ok() { "TurnEnd" } else { "Halt" };
    let _ = audit
        .write(Some(session_id), kind, serde_json::json!({}))
        .await;
    emit_event(events, kind_label, serde_json::json!({}));
    result
}

#[allow(clippy::too_many_arguments)]
async fn run_turn_with_tools(
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
    vertical_deny: &std::collections::HashSet<String>,
    vertical_approval: &std::collections::HashSet<String>,
    session_grants: SessionGrantState,
) -> ArgoResult<TurnResult> {
    let mut session_grants = session_grants;
    let (registry, engine, principal) = build_dispatch_ctx(cwd)?;
    let tools = provider_schemas(&registry);
    let mut system_prompt = build_system_prompt(store, session_id).await?;
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
            });
        }

        for call in &resp.tool_calls {
            let call_payload = serde_json::json!({"tool": call.name, "call_id": call.id});
            audit
                .write(Some(session_id), AuditKind::ToolCall, call_payload.clone())
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
                    .write(Some(session_id), AuditKind::RbacDeny, deny_payload.clone())
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
            } else if vertical_approval.contains(&call.name)
                && !session_grants.allows(&call.name)
            {
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
                let approved_via_grant = vertical_approval.contains(&call.name)
                    && session_grants.allows(&call.name);
                let res = dispatch_one(&registry, &engine, &principal, cwd, call).await;
                if approved_via_grant {
                    consume_one_shot_if_present(
                        store, session_id, &call.name, &mut session_grants, audit, events,
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
                )
                .await?;
            emit_event(events, "ToolResult", result_payload);
            let tool_msg = serde_json::json!([ContentBlock::ToolResult(result)]);
            store
                .append_message(session_id, MessageRole::Tool, &tool_msg.to_string())
                .await?;
        }

        if round + 1 == max_rounds {
            return Err(ArgoError::Provider(format!(
                "tool-dispatch loop hit max_rounds={max_rounds}; aborting"
            )));
        }
    }
    Ok(TurnResult {
        assistant_id: last_assistant_id,
        text: last_text,
        tokens_in: last_tokens_in,
        tokens_out: last_tokens_out,
    })
}

/// Concatenate skill bodies into a single system-prompt string
/// that's passed on every round of the turn. Returns `None` when
/// nothing is in scope (the provider transport then sends no
/// `system` block at all).
///
/// Resolution order:
/// 1. If `session_skill` rows exist for `session_id`, use *only*
///    those skills (per-session curation, #279).
/// 2. Otherwise fall back to every active row in the workspace.
async fn build_system_prompt(
    store: &Arc<SessionStore>,
    session_id: SessionId,
) -> ArgoResult<Option<String>> {
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    let attached = skills.skills_for_session(session_id).await?;
    let active = if attached.is_empty() {
        skills.list_active().await?
    } else {
        attached
    };
    if active.is_empty() {
        return Ok(None);
    }
    let mut out = String::from(
        "You have access to the following workspace skills. \
         Apply them as guidance for tool selection, response style, \
         and domain knowledge.\n\n",
    );
    for s in active {
        out.push_str(&format!(
            "## {} (v{})\n",
            s.frontmatter.name, s.frontmatter.version
        ));
        if !s.frontmatter.description.is_empty() {
            out.push_str(&s.frontmatter.description);
            out.push('\n');
        }
        out.push('\n');
        out.push_str(s.body.trim_end());
        out.push_str("\n\n---\n\n");
    }
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_store::SessionStore;

    #[tokio::test]
    async fn build_system_prompt_returns_none_for_empty_workspace() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let prompt = build_system_prompt(&store, SessionId(uuid::Uuid::nil()))
            .await
            .unwrap();
        assert!(prompt.is_none());
    }

    #[tokio::test]
    async fn build_system_prompt_concatenates_active_skills() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let skills = argo_skill::SkillStore::new(Arc::clone(&store));
        let raw = "---\nname: alpha\nversion: \"0.1\"\ndescription: D\n\
                   lock: none\nprovenance:\n  source: builtin\n  hash: sha256-x\n---\nbody-of-alpha";
        skills.install(raw, None).await.unwrap();
        let prompt = build_system_prompt(&store, SessionId(uuid::Uuid::nil()))
            .await
            .unwrap()
            .unwrap();
        assert!(prompt.contains("## alpha (v0.1)"));
        assert!(prompt.contains("body-of-alpha"));
        assert!(prompt.starts_with("You have access to"));
    }

    #[tokio::test]
    async fn build_system_prompt_honours_per_session_attachments() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let (tenant, ws) = store.ensure_workspace("ws", "/tmp/sa").await.unwrap();
        let sid = store
            .create_session(tenant, ws, "m", "p", Some("t"), None)
            .await
            .unwrap();

        let skills = argo_skill::SkillStore::new(Arc::clone(&store));
        let raw_a = "---\nname: alpha\nversion: \"0.1\"\ndescription: D\n\
                     lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nbody-A";
        let raw_b = "---\nname: beta\nversion: \"0.1\"\ndescription: D\n\
                     lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nbody-B";
        let _alpha = skills.install(raw_a, None).await.unwrap();
        let beta = skills.install(raw_b, None).await.unwrap();

        // Default behaviour: both active skills are in scope.
        let prompt_default = build_system_prompt(&store, sid).await.unwrap().unwrap();
        assert!(prompt_default.contains("body-A"));
        assert!(prompt_default.contains("body-B"));

        // Attach only beta — alpha should now drop out.
        skills.attach_to_session(sid, beta.id).await.unwrap();
        let prompt_curated = build_system_prompt(&store, sid).await.unwrap().unwrap();
        assert!(prompt_curated.contains("body-B"));
        assert!(!prompt_curated.contains("body-A"));

        // Detach: falls back to all-active.
        let removed = skills.detach_from_session(sid, beta.id).await.unwrap();
        assert_eq!(removed, 1);
        let prompt_after = build_system_prompt(&store, sid).await.unwrap().unwrap();
        assert!(prompt_after.contains("body-A"));
        assert!(prompt_after.contains("body-B"));
    }

    // Vertical-related unit tests live in `turn_vertical.rs`
    // since the helpers were extracted there to keep this file
    // under the 500-LOC budget (#654).
}
