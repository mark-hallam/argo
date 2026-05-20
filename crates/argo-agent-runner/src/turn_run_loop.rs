//! Inner orchestration for a single agent turn extracted from
//! [`crate::turn`] so the parent stays under the 500-LOC ceiling
//! (HARD RULE 6). Loads the session row, runs the freeze + budget
//! gates, resolves the routing/transport, then hands off to
//! [`super::turn_helpers::run_turn_with_tools`] for the actual
//! tool-dispatch loop.

use crate::helpers::{emit_event, price_for, RunEventSink, SpendCtx};
use crate::transport::{build_transport, load_manifest};
use crate::turn_vertical::{
    build_session_grant_state, build_turn_start_payload, build_vertical_approval_set,
    build_vertical_deny_set, resolve_vertical_for_session,
};
use argo_audit::{AuditKind, AuditSink};
use argo_provider::Transport;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, MessageRole, SessionId};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

use super::turn_helpers;
use super::turn_helpers_freeze;
use super::turn_skills_runtime;
use super::TurnResult;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_turn_with_events_inner(
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

    // mission-pm-kill-switch-2-runner: pre-turn freeze gate. Refuses
    // to start a turn for a frozen worker before any provider work or
    // audit emission. No-op when the session has no `worker_id`.
    if let Some(wid) = session.worker_id {
        if let Some(rec) = argo_store::worker_freeze::is_frozen(
            store.pool(),
            &session.tenant_id.to_string(),
            &wid.to_string(),
        )
        .await?
        {
            return Err(ArgoError::Invariant(turn_helpers_freeze::freeze_message(
                &rec,
            )));
        }
    }

    if let Some(text) = user {
        let content = serde_json::json!([{"type": "text", "text": text}]);
        store
            .append_message(session_id, MessageRole::User, &content.to_string())
            .await?;
    }

    let manifest = load_manifest(cwd, &session.provider)?;
    let provider_id = manifest.provider.id.clone();
    let audit = AuditSink::new_with_tenant(store.clone(), session.tenant_id.to_string());
    // mission-claim-3-runtime-wiring-v1x: consult the tier router
    // *before* picking the model for this turn. Sessions with no
    // routing_policy or a non-tier policy (the v1.0 default) skip
    // this entirely; sessions opting into `RoutingPolicy::Tiered`
    // get their model overridden to the tier's `chosen_provider_model`.
    let routing_decision =
        crate::turn_routing::maybe_route_turn(store, &session, manifest.clone(), user, &audit)
            .await?;
    let effective_model: String = routing_decision
        .as_ref()
        .map(|d| d.chosen_provider_model.model.clone())
        .unwrap_or_else(|| session.model.clone());
    let pricing = price_for(&manifest, &effective_model);
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

    // Pre-turn budget gate (mission-delegation-budget-tree): if a
    // running subtask is bound to this session and its declared
    // budget is exhausted, abort cleanly with `BudgetExhausted`
    // before any provider work fires. Unbounded sessions (no
    // running subtask, or NULL budget) fall through unchanged.
    crate::turn_budget::check_pre_turn_budget(store, session_id, &audit).await?;
    let turn_start = build_turn_start_payload(
        &session.provider,
        &effective_model,
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
        model: effective_model.as_str(),
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
        &effective_model,
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
