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
use argo_provider::{ProviderManifest, Transport};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, MessageRole, SessionId, TenantId};
use std::path::Path;
use std::sync::Arc;
use tracing::Instrument;
use uuid::Uuid;

/// Default cap on tool-dispatch rounds within a single `run_turn`
/// when callers don't pass an explicit `max_rounds`. Generous
/// enough for legitimate planning chains; small enough that an
/// accidental loop fails fast.
///
/// ```
/// use argo_agent_runner::MAX_TOOL_ROUNDS;
/// assert_eq!(MAX_TOOL_ROUNDS, 8);
/// ```
pub const MAX_TOOL_ROUNDS: usize = 8;

/// Outcome of one agent-loop turn. Consumed by both the CLI
/// handler (which prints) and the gateway's
/// `POST /sessions/<id>/runs` endpoint (which serializes to JSON).
///
/// ```
/// use argo_agent_runner::TurnResult;
/// use uuid::Uuid;
/// let r = TurnResult {
///     assistant_id: Uuid::nil(),
///     text: "hello".into(),
///     tokens_in: Some(10),
///     tokens_out: Some(5),
///     stopped_on_tool_budget: false,
/// };
/// assert_eq!(r.text, "hello");
/// assert!(!r.stopped_on_tool_budget);
/// ```
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

/// Like [`run_turn_with_events`], but consults a [`PlannerHook`]
/// before the LLM round loop so coarse goals get a planner pass
/// and the resulting subtasks land in the [`SubtaskSink`] for the
/// existing handoff walker to pick up
/// (mission-planner-dispatch-loop-wire). When `hook` is `None` or
/// `user` is `None` (i.e. a "continue" turn carries no fresh
/// goal) the call degrades to the regular [`run_turn_with_events`]
/// path. Decomposition errors never bubble — see
/// [`crate::turn_planner::maybe_decompose`].
#[allow(clippy::too_many_arguments)]
pub async fn run_turn_with_planner_hook(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    user: Option<&str>,
    stream: bool,
    max_rounds: usize,
    events: Option<&RunEventSink>,
    hook: Option<&turn_planner::PlannerHook<'_>>,
) -> ArgoResult<TurnResult> {
    if let (Some(text), Some(h)) = (user, hook) {
        let parent = session_id.0.to_string();
        let _ =
            turn_planner::maybe_decompose(h.planner, h.gate, h.sink, Some(&parent), text, &h.ctx)
                .await;
    }
    run_turn_with_events(store, cwd, session_id, user, stream, max_rounds, events).await
}

/// Like `run_turn`, but with an optional `events` sink. The gateway
/// streaming endpoint passes a real `UnboundedSender`; the CLI +
/// non-streaming gateway pass `None`. Each event mirrors the audit
/// emission that just landed.
///
/// Wrapped in `tracing::info_span!("turn", …)` so every emission
/// inside the agent loop inherits `session_id`, `tenant_id`, and
/// `worker_id` without per-callsite changes
/// (mvp-structured-logging). `tenant_id`/`worker_id` are recorded
/// after the session row loads.
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
    let span = tracing::info_span!(
        "turn",
        session_id = %session_id.0,
        tenant_id = tracing::field::Empty,
        worker_id = tracing::field::Empty,
    );
    run_turn_with_events_inner(store, cwd, session_id, user, stream, max_rounds, events)
        .instrument(span)
        .await
}

#[allow(clippy::too_many_arguments)]
async fn run_turn_with_events_inner(
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
    let span = tracing::Span::current();
    span.record("tenant_id", tracing::field::display(&session.tenant_id.0));
    if let Some(w) = session.worker_id {
        span.record("worker_id", tracing::field::display(&w.0));
        // mission-prompt-cache-affinity-wire-mints: keep the
        // affinity registry warm so the next turn under the same
        // `(tenant, worker)` pair reuses this session id inside the
        // prompt-cache TTL window. No-op when no registry is wired.
        if let Some(affinity) = store.prompt_cache_affinity() {
            affinity.record(session.tenant_id, &w.to_string(), session_id);
        }
    }

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
    // Pre-turn budget gate (mission-delegation-budget-tree): if a
    // running subtask is bound to this session and its declared
    // budget is exhausted, abort cleanly with `BudgetExhausted`
    // before any provider work fires. Unbounded sessions (no
    // running subtask, or NULL budget) fall through unchanged.
    crate::turn_budget::check_pre_turn_budget(store, session_id, &audit).await?;
    let turn_start = build_turn_start_payload(
        &session.provider,
        &session.model,
        session.vertical_id.as_deref(),
    );
    audit
        .write(
            Some(session_id),
            AuditKind::TurnStart,
            turn_start.clone(),
            None,
        )
        .await?;
    emit_event(events, "TurnStart", turn_start);
    let spend_ctx = SpendCtx {
        tenant_id: session.tenant_id,
        provider: provider_id.as_str(),
        model: session.model.as_str(),
        pricing,
    };
    // mission-skill-select-variant-runtime-wire: per-turn invocation
    // id seeds `argo_skill::select_variant` and dedups
    // `record_outcome` writes on `(run_id, invocation_id)`. Uuidv7
    // is unique per turn even within the same session.
    let invocation_id = Uuid::now_v7().to_string();
    let mut pending_ab_outcomes: Vec<turn_skills_runtime::PendingAbOutcome> = Vec::new();
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
        &invocation_id,
        &mut pending_ab_outcomes,
    )
    .await;
    let succeeded = result.is_ok();
    // Side-channel: never fail the turn for A/B outcome IO.
    turn_skills_runtime::record_pending_outcomes(store, &pending_ab_outcomes, succeeded, None)
        .await;
    let kind = if succeeded {
        AuditKind::TurnEnd
    } else {
        AuditKind::Halt
    };
    let kind_label = if succeeded { "TurnEnd" } else { "Halt" };
    let _ = audit
        .write(Some(session_id), kind, serde_json::json!({}), None)
        .await;
    emit_event(events, kind_label, serde_json::json!({}));
    result
}

fn model_context_window(manifest: &ProviderManifest, model: &str) -> usize {
    let resolved = manifest.resolve_model_id(model);
    manifest
        .models
        .iter()
        .find(|m| m.id == resolved)
        .and_then(|m| m.context_window)
        .map_or(0, |n| n as usize)
}

fn load_context_compress_config(cwd: &Path) -> ArgoResult<argo_config::ContextCompressConfig> {
    let mut config = argo_config::load_from(cwd)?.runtime.context_compress;
    let local = [cwd.join(".skycot/argo.toml"), cwd.join("argo.toml")]
        .into_iter()
        .find(|path| path.exists());
    if let Some(path) = local {
        config = argo_config::ArgoConfig::load(&path)?
            .runtime
            .context_compress;
    }
    Ok(config)
}

struct ContextWindowPolicy {
    window_tokens: usize,
    compression: argo_runtime::context_compress::CompressionConfig,
}

impl ContextWindowPolicy {
    fn disabled() -> Self {
        Self::from_config(argo_config::ContextCompressConfig::default(), 0)
    }

    fn from_config(config: argo_config::ContextCompressConfig, window_tokens: usize) -> Self {
        Self {
            window_tokens,
            compression: argo_runtime::context_compress::CompressionConfig {
                enabled: config.enabled,
                trigger_pct: config.trigger_pct,
                keep_last_turns: config.keep_last_turns as usize,
                summary_max_tokens: config.summary_max_tokens as usize,
            },
        }
    }
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
#[cfg(test)]
async fn build_system_prompt(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    vertical_id: Option<&str>,
    tenant_id: TenantId,
    worker_id: Option<argo_store::WorkerId>,
) -> ArgoResult<Option<String>> {
    let (prompt, _pending) =
        build_system_prompt_with_ab(store, session_id, vertical_id, tenant_id, worker_id, "")
            .await?;
    Ok(prompt)
}

/// Compose the system prompt AND resolve A/B variants in one pass.
///
/// Behaves like [`build_system_prompt`] but additionally returns the
/// pending A/B outcomes that the caller must record after the turn
/// settles. `invocation_id` is the SHA-256 seed for
/// `argo_skill::select_variant`; pass an empty string when the
/// caller does NOT care about A/B selection (the variant choice
/// falls back to "keep all rows" because no run can match).
async fn build_system_prompt_with_ab(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    vertical_id: Option<&str>,
    tenant_id: TenantId,
    worker_id: Option<argo_store::WorkerId>,
    invocation_id: &str,
) -> ArgoResult<(Option<String>, Vec<turn_skills_runtime::PendingAbOutcome>)> {
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    let attached = skills.skills_for_session(session_id).await?;
    let mut active = if !attached.is_empty() {
        attached
    } else if let Some(vid) = vertical_id {
        skills.list_skills_for_vertical(vid).await?
    } else {
        skills.list_active().await?
    };
    turn_canary::apply_canary_routing(store, &skills, session_id, &mut active).await?;
    let (active, pending) =
        turn_skills_runtime::resolve_variants_for_turn(store, invocation_id, active).await;
    turn_skills_migration::emit_pending_migration_hints(store, &active).await;
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
    // mission-skill-session-boot-wire: render the trust-gated
    // "Available skills" block (name + version + description only,
    // no body) once per session boot, between the active workspace
    // skills and the memory composition. Empty list / lookup error
    // → omit; session boot never fails.
    if let Some(block) =
        turn_skills::compose_available_skills_block(store, tenant_id, worker_id).await
    {
        turn_skills::inject_skills_block(&mut out, &block);
    }
    let memory = turn_memory::compose_prompt_memory(store, tenant_id, worker_id).await;
    if !memory.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&memory);
    }
    let prompt = if out.is_empty() { None } else { Some(out) };
    Ok((prompt, pending))
}

#[path = "turn_helpers.rs"]
mod turn_helpers;

#[path = "turn_canary.rs"]
mod turn_canary;

#[path = "turn_memory.rs"]
mod turn_memory;

#[path = "turn_skills_runtime.rs"]
mod turn_skills_runtime;

#[path = "turn_skills.rs"]
mod turn_skills;

#[cfg(test)]
#[path = "turn_skills_tests.rs"]
mod turn_skills_tests;

#[path = "turn_planner.rs"]
pub mod turn_planner;

#[cfg(test)]
#[path = "turn_planner_tests.rs"]
mod turn_planner_tests;

#[path = "turn_skills_migration.rs"]
mod turn_skills_migration;

#[cfg(test)]
#[path = "turn_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "turn_tests_max_rounds.rs"]
mod tests_max_rounds;
