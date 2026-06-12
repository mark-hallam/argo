//! tg1 — emit a capability request when a worker's fence denies a tool.
//!
//! Extracted from `turn_helpers_run.rs` to keep that file under the
//! 500-LOC ceiling. One concern: at the per-call dispatch seam, detect
//! when the worker's derived capability profile would refuse a tool and
//! make that (otherwise silent) deny LEGIBLE — persist a pending
//! `capability_request` row + emit a `CapabilityGap` audit row so the
//! connect-on-demand grant card can surface "this teammate needed
//! <connector>". Observe-only: nothing here changes dispatch behavior.

use super::turn_helpers_run::TurnDispatchCtx;
use crate::helpers::{emit_event, RunEventSink};
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{SessionId, ToolCallRequest};
use std::sync::Arc;

/// Persist a pending `capability_request` + emit an audit row when the
/// worker's capability fence would refuse `call.name`. Runs the SAME
/// `engine.check` the dispatch seam runs, so it predicts the deny without
/// changing dispatch. No-op for owner/human sessions (the engine permits
/// everything → cheap early return) and for sessions with no `worker_id`
/// (the request is per-worker). The DB read only fires on the rare deny
/// path. Persist/audit failures are logged, never propagated — emission
/// is best-effort observability, not part of the dispatch contract.
pub(super) async fn maybe_record_capability_request(
    ctx: &TurnDispatchCtx,
    store: &Arc<SessionStore>,
    session_id: SessionId,
    audit: &AuditSink,
    events: Option<&RunEventSink>,
    call: &ToolCallRequest,
) {
    if ctx
        .engine
        .check(
            &ctx.principal,
            &argo_types::Capability::ToolDispatch {
                tool_name_glob: call.name.clone(),
            },
        )
        .is_ok()
    {
        return;
    }
    let Ok(Some(session)) = store.get_session(session_id).await else {
        return;
    };
    let Some(worker_id) = session.worker_id else {
        return;
    };
    let access = classify_access(&call.name);
    let run_id = session_id.to_string();
    if let Err(e) = argo_store::capability_request::record_pending(
        store.pool(),
        &session.tenant_id.to_string(),
        &worker_id.to_string(),
        &call.name,
        access,
        Some(&run_id),
    )
    .await
    {
        tracing::warn!(
            target: "agent_runner::capability_request",
            error = %e,
            tool = %call.name,
            "failed to persist capability_request on fence deny"
        );
        return;
    }
    let payload = serde_json::json!({
        "signal": "capability_request",
        "worker_id": worker_id.to_string(),
        "tool_or_connector": call.name,
        "requested_access": access,
        "run_id": run_id,
        "status": "pending",
    });
    if let Err(e) = audit
        .write(Some(session_id), AuditKind::CapabilityGap, payload.clone(), None)
        .await
    {
        tracing::warn!(
            target: "agent_runner::capability_request",
            error = %e,
            "capability_request audit write failed"
        );
    }
    emit_event(events, "CapabilityGap", payload);
}

/// Heuristic read/write classification for a denied tool. The fence deny
/// only tells us a tool was refused, not the access shape; the grant card
/// lets the user adjust, so this is a best-effort default — read-leaning
/// verbs map to `read`, everything else to `write` (the conservative
/// assumption for a connector that may mutate).
fn classify_access(tool: &str) -> &'static str {
    const READ_HINTS: &[&str] = &[
        "read", "get", "list", "search", "fetch", "query", "show", "view", "describe", "lookup",
    ];
    let lower = tool.to_ascii_lowercase();
    if READ_HINTS.iter().any(|h| lower.contains(h)) {
        "read"
    } else {
        "write"
    }
}

#[cfg(test)]
#[path = "turn_helpers_capability_request_tests.rs"]
mod tests;
