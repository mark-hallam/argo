//! Audit-write helpers for the workflow-definition gateway handlers.
//!
//! Extracted from `routes_workflows_handlers.rs` under
//! `mission-workflow-audit-write-transactional` so the parent stays
//! under HARD RULE 6 (500-LOC ceiling). Two transactional shapes
//! live here:
//!
//! * [`run_transactional_audit`] — open one sqlx tx, run the caller's
//!   `body` (which performs BOTH the workflow-row mutation and the
//!   `audit_record` insert via [`AuditSink::write_in_tx`]), commit,
//!   then fire the audit post-commit fan-out. If `body` returns
//!   `Err`, the tx is dropped before commit and both writes roll
//!   back together — closes the P1 atomicity gap targeted by
//!   `mission-workflow-audit-write-transactional`.
//! * [`emit_audit_row`] — non-transactional fallback retained for
//!   `create_version`, which the brief leaves out of scope.

use argo_audit::{AuditKind, AuditPostCommit, AuditSink};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use futures::future::BoxFuture;
use serde_json::{json, Value};
use sqlx::{Sqlite, Transaction};
use std::sync::Arc;

/// Open a single sqlx transaction, run the caller's `body`, commit,
/// then fire the audit post-commit fan-out. `body` is expected to
/// perform BOTH the workflow-row mutation and the `audit_record`
/// insert via [`AuditSink::write_in_tx`] — the returned guard is
/// what carries fan-out state into the post-commit step.
///
/// The `AuditSink` is constructed fresh per call to keep the seam
/// narrow; production wiring can share a sink later without changing
/// this helper's signature.
pub(super) async fn run_transactional_audit<F>(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    body: F,
) -> ArgoResult<()>
where
    F: for<'c> FnOnce(
        &'c mut Transaction<'_, Sqlite>,
        &'c AuditSink,
    ) -> BoxFuture<'c, ArgoResult<AuditPostCommit>>,
{
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    let mut tx = store
        .pool()
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow audit tx begin: {e}")))?;
    let post = body(&mut tx, &sink).await?;
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("workflow audit tx commit: {e}")))?;
    if let Err(e) = sink.after_commit_fanout(post).await {
        tracing::warn!(
            target: "gateway::routes_workflows",
            error = %e,
            "workflow audit post-commit fanout failed",
        );
    }
    Ok(())
}

/// Canonical workflow audit payload — used by every workflow audit
/// emit (in-tx and the non-transactional `create_version` fallback)
/// so subscribers see one wire shape regardless of handler path.
pub(super) fn workflow_audit_payload(
    tenant: TenantId,
    vertical_id: &str,
    name: &str,
    version_id: &str,
    actor: &str,
) -> Value {
    json!({
        "actor": actor,
        "tenant_id": tenant.to_string(),
        "vertical_id": vertical_id,
        "name": name,
        "version_id": version_id,
    })
}

/// Post-success audit emit for `create_version`. The
/// `mission-workflow-audit-write-transactional` brief only names the
/// 3 `set_head` / `archive` / `rollback` mutators, so `create_version`
/// keeps the pre-existing (non-transactional) audit-write shape
/// until a follow-up brief widens the scope.
pub(super) async fn emit_audit_row(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    kind: AuditKind,
    vertical_id: &str,
    name: &str,
    version_id: &str,
    actor: &str,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    let payload = workflow_audit_payload(tenant, vertical_id, name, version_id, actor);
    if let Err(e) = sink
        .write(None, kind, payload, Some(actor.to_string()))
        .await
    {
        tracing::warn!(
            target: "gateway::routes_workflows",
            error = %e,
            kind = %kind.label(),
            vertical_id = %vertical_id,
            name = %name,
            "workflow audit write failed",
        );
    }
}
