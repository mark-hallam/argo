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
//!
//! The inner tool-dispatch loop (`run_turn_with_tools`) lives in
//! the sibling `turn_helpers.rs` so this file stays under the
//! 500-LOC ceiling (HARD RULE 6); see also the
//! `claude-agent-runner-turn-split` brief.
use crate::helpers::{emit_event, price_for, RunEventSink, SpendCtx};
use crate::transport::{build_transport, load_manifest};
use crate::turn_vertical::{
    build_session_grant_state, build_turn_start_payload, build_vertical_approval_set,
    build_vertical_deny_set, resolve_vertical_for_session,
};
use argo_audit::{AuditKind, AuditSink};
use argo_provider::Transport;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, MessageRole, SessionId, TenantId};
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
    /// `true` when the loop exited because `max_rounds` was hit
    /// (the model still wanted more tool dispatch). The session
    /// state is consistent — every dispatched tool has its
    /// matching `tool_result` persisted — so the caller can
    /// safely issue another `run_turn` to continue. Worker v2's
    /// outer step loop uses this signal to convert a research-
    /// heavy turn into a clean step boundary instead of a
    /// transport error (#724).
    pub stopped_on_tool_budget: bool,
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
        .write(Some(session_id), AuditKind::TurnStart, turn_start.clone(), None)
        .await?;
    emit_event(events, "TurnStart", turn_start);
    let spend_ctx = SpendCtx {
        tenant_id: session.tenant_id,
        provider: provider_id.as_str(),
        model: session.model.as_str(),
        pricing,
    };
    let result = turn_helpers::run_turn_with_tools(
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
        session.vertical_id.as_deref(),
        &vertical_deny,
        &vertical_approval,
        session_grants,
        session.worker_id,
    )
    .await;
    let kind = if result.is_ok() {
        AuditKind::TurnEnd
    } else {
        AuditKind::Halt
    };
    let kind_label = if result.is_ok() { "TurnEnd" } else { "Halt" };
    let _ = audit
        .write(Some(session_id), kind, serde_json::json!({}), None)
        .await;
    emit_event(events, kind_label, serde_json::json!({}));
    result
}

/// Concatenate skill bodies into a single system-prompt string
/// that's passed on every round of the turn. Returns `None` when
/// nothing is in scope (the provider transport then sends no
/// `system` block at all).
///
/// Resolution order:
/// 1. If `session_skill` rows exist for `session_id`, use *only*
///    those skills (per-session curation, #279).
/// 2. Otherwise, when `vertical_id` is bound (#679 — Rule 6),
///    fall back to skills scoped to that vertical
///    (`SkillStore::list_skills_for_vertical`). Cross-vertical
///    skill bleed-through is blocked at this layer.
/// 3. Otherwise (unbound legacy session), fall back to every
///    active row in the workspace.
async fn build_system_prompt(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    vertical_id: Option<&str>,
    tenant_id: TenantId,
    worker_id: Option<argo_store::WorkerId>,
) -> ArgoResult<Option<String>> {
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    let attached = skills.skills_for_session(session_id).await?;
    let active = if !attached.is_empty() {
        attached
    } else if let Some(vid) = vertical_id {
        skills.list_skills_for_vertical(vid).await?
    } else {
        skills.list_active().await?
    };
    let mut out = String::new();
    if !active.is_empty() {
        out.push_str(
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
    }
    let memory = turn_memory::compose_prompt_memory(store, tenant_id, worker_id).await;
    if !memory.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&memory);
    }
    if out.is_empty() {
        Ok(None)
    } else {
        Ok(Some(out))
    }
}

#[path = "turn_helpers.rs"]
mod turn_helpers;

#[path = "turn_memory.rs"]
mod turn_memory;

#[cfg(test)]
#[path = "turn_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "turn_tests_max_rounds.rs"]
mod tests_max_rounds;
