//! Pre-turn + pre-dispatch worker-freeze gate (cooperative).
//! Sibling of `turn_helpers.rs`; v1 caches `is_frozen` once per turn —
//! freezes applied mid-turn land at the next turn boundary.

use argo_audit::{AuditKind, AuditSink};
use argo_store::{worker_freeze, FreezeRecord, SessionStore, WorkerId};
use argo_types::{ArgoError, ArgoResult, SessionId, TenantId};

/// User-facing freeze message; matched verbatim by sibling CLI/UI slices.
pub(super) fn freeze_message(rec: &FreezeRecord) -> String {
    format!(
        "Worker {} is frozen by {}: {}. New turns and tool calls are blocked. In-flight tool calls continue to completion.",
        rec.worker_id, rec.frozen_by, rec.reason
    )
}

/// Per-turn cache wrapper around [`worker_freeze::is_frozen`].
pub(super) async fn check_or_cache(
    store: &SessionStore,
    tenant_id: TenantId,
    worker_id: WorkerId,
    cache: &mut Option<Option<FreezeRecord>>,
) -> ArgoResult<Option<FreezeRecord>> {
    if let Some(slot) = cache.as_ref() {
        return Ok(slot.clone());
    }
    let row = worker_freeze::is_frozen(
        store.pool(),
        &tenant_id.to_string(),
        &worker_id.to_string(),
    )
    .await?;
    *cache = Some(row.clone());
    Ok(row)
}

/// Emit a `WorkerFreezeBlocked` audit row for a refused tool call.
pub(super) async fn emit_blocked_audit(
    audit: &AuditSink,
    session_id: SessionId,
    worker_id: WorkerId,
    tool_name: &str,
    rec: &FreezeRecord,
) -> ArgoResult<()> {
    audit
        .write(
            Some(session_id),
            AuditKind::WorkerFreezeBlocked,
            serde_json::json!({
                "worker_id": worker_id.to_string(),
                "tool": tool_name,
                "frozen_by": rec.frozen_by,
                "reason": rec.reason,
            }),
            None,
        )
        .await
}

/// Pre-dispatch gate. Combines [`check_or_cache`] + [`emit_blocked_audit`]
/// and errors with [`freeze_message`] when the cached lookup says frozen.
pub(super) async fn pre_dispatch_or_block(
    store: &SessionStore,
    audit: &AuditSink,
    session_id: SessionId,
    tenant_id: TenantId,
    worker_id: Option<WorkerId>,
    tool_name: &str,
    cache: &mut Option<Option<FreezeRecord>>,
) -> ArgoResult<()> {
    let Some(wid) = worker_id else {
        return Ok(());
    };
    let Some(rec) = check_or_cache(store, tenant_id, wid, cache).await? else {
        return Ok(());
    };
    emit_blocked_audit(audit, session_id, wid, tool_name, &rec).await?;
    Err(ArgoError::Invariant(freeze_message(&rec)))
}
