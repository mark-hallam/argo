//! Audit emission for the durable capability grant/revoke route (tg3 / tg3b).
//!
//! Split out of `routes_worker_capability_grant.rs` to keep both files under
//! the 500-LOC ceiling (HARD RULE 6). The load-bearing invariant lives in the
//! parent header: on this fence-LOOSENING surface the audit row is written
//! BEFORE the manifest mutates and a sink error fails the request, so a grant
//! can never persist without its audit row.

use super::super::routes_auth_principal::session_principal_id_for_audit;
use super::Access;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::Worker;
use argo_store::SessionStore;
use argo_types::ArgoResult;
use serde_json::{json, Value};
use std::sync::Arc;

/// Emit the grant audit row. Returns the sink result so the SUCCESS path can
/// treat audit as a hard prerequisite (tg3b); deny callers ignore it.
#[allow(clippy::too_many_arguments)]
pub(super) async fn emit_grant_audit(
    store: &Arc<SessionStore>,
    req: &str,
    worker: &Worker,
    kind: AuditKind,
    outcome: &str,
    tool: &str,
    access: Access,
    request_id: Option<&str>,
) -> ArgoResult<()> {
    let payload = json!({
        "endpoint": "worker.capability_granted",
        "worker_id": worker.id.to_string(),
        "tool": tool,
        "access": access.as_str(),
        "outcome": outcome,
        "request_id": request_id,
        "granted_by": session_principal_id_for_audit(req),
    });
    write_audit(store, req, worker, kind, payload).await
}

pub(super) async fn emit_revoke_audit(
    store: &Arc<SessionStore>,
    req: &str,
    worker: &Worker,
    tool: &str,
    outcome: &str,
) -> ArgoResult<()> {
    let payload = json!({
        "endpoint": "worker.capability_revoked",
        "worker_id": worker.id.to_string(),
        "tool": tool,
        "outcome": outcome,
        "revoked_by": session_principal_id_for_audit(req),
    });
    write_audit(store, req, worker, AuditKind::WorkerCapabilityGrantChanged, payload).await
}

/// Write one audit row, logging a warn AND returning the sink error so callers
/// can fail the request on it (the loosening paths do; the deny paths don't).
/// The `#[cfg(test)]` seam forces a sink failure to pin the prerequisite.
pub(super) async fn write_audit(
    store: &Arc<SessionStore>,
    req: &str,
    worker: &Worker,
    kind: AuditKind,
    payload: Value,
) -> ArgoResult<()> {
    #[cfg(test)]
    if std::env::var("SKYCOT_TEST_FORCE_AUDIT_SINK_ERROR").is_ok() {
        return Err(argo_types::ArgoError::Io(
            "forced audit sink error (tg3b test seam)".to_string(),
        ));
    }
    let sink = AuditSink::new_with_tenant(Arc::clone(store), worker.tenant_id.to_string());
    let result = sink
        .write(None, kind, payload, session_principal_id_for_audit(req))
        .await;
    if let Err(e) = &result {
        tracing::warn!(
            target: "gateway::worker_capability_grant",
            error = %e,
            "worker.capability_grant audit write failed"
        );
    }
    result
}
