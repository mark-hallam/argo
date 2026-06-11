//! cu5 (DEC-050 G5 · `cu5-spa-approve-and-execute-held-actions`): close
//! the human-in-the-loop approval→work loop IN the product.
//!
//! When the operator approves a held worker action in `/approvals`
//! ([`super::routes_approvals_json::decide_response`]), this module
//! (a) writes the hash-chained `ApprovalGranted` audit row, (b) dispatches
//! the now-approved action through its declared executor — REUSING the cu2
//! inline dispatcher
//! ([`crate::handlers::work_dispatch_executor_inline::execute_approved_action_inline`])
//! so an approval becomes REAL execution in one click — and (c) writes the
//! hash-chained `ApprovalConsumed` audit row recording the execution
//! outcome.
//!
//! Execution is best-effort relative to the approval: a resolve / dispatch
//! failure transitions the worker-action row to `failed` and is reported
//! in the response `execution` field, but the approval itself still stands
//! (HTTP 200). The operator's approval is the authorization, so this path
//! is NOT gated by `ARGO_EXECUTION_GRADUATION` (that flag gates the
//! AUTONOMOUS cu2 path where no human is in the loop); the per-worker
//! circuit (probation) gate inside the inline dispatcher still applies.

use super::routes_approvals_json::emit_approval_declined;
use super::routes_approvals_json_helpers::{ok_decide, server_error};
use crate::handlers::work_dispatch_executor_inline::execute_approved_action_inline;
use crate::handlers::worker_actions::ActionRecord;
use crate::handlers::worker_actions_transitions_lookup::LocatedAction;
use argo_audit::{AuditKind, AuditSink};
use argo_memory::MemoryStore;
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use serde_json::json;
use std::sync::Arc;

/// Finalise an approve / refuse decision after the worker-action
/// state-machine transition has succeeded: using the request tenant
/// resolved by the caller, either audit the decline (refuse) or audit
/// the grant + execute the held action (approve, cu5 · DEC-050 G5).
/// Factored out of
/// [`super::routes_approvals_json::decide_response`] to keep that handler
/// under the 80-LOC function budget.
#[allow(clippy::too_many_arguments)]
pub(super) async fn finalize_decision(
    tenant: TenantId,
    store: &Arc<SessionStore>,
    memory: &MemoryStore,
    workspace: WorkspaceId,
    decision: &str,
    located: &LocatedAction,
    decided: &ActionRecord,
    actor: &str,
    approval_id: &str,
) -> (&'static str, String) {
    // cu1h (DEC-050 #6): the request tenant is resolved once in
    // `decide_response` and threaded in here — this path previously
    // re-resolved it from `req`, a redundant (and fallible) second pass.
    if decision == "refuse" {
        if let Err(e) =
            emit_approval_declined(store, &tenant.to_string(), actor, &located.record, decided).await
        {
            return server_error(&format!("approval declined audit: {e}"));
        }
        return ok_decide(approval_id, decision);
    }
    approve_and_execute(
        store,
        memory,
        workspace,
        tenant,
        &located.vertical_id,
        decided,
        actor,
        approval_id,
    )
    .await
}

/// Audit the approval, dispatch the just-approved action through its
/// executor, audit the execution outcome, and return the decide envelope
/// (with an `execution` field — `"dispatched"` or `"failed"`).
///
/// `decided` is the `approved` record produced by the `approve()`
/// transition; the inline dispatcher re-reads the latest row by
/// `action_id`, so only its `action_id` / `action_type` / `worker_id`
/// are load-bearing here.
#[allow(clippy::too_many_arguments)]
pub(super) async fn approve_and_execute(
    store: &Arc<SessionStore>,
    memory: &MemoryStore,
    workspace: WorkspaceId,
    tenant: TenantId,
    vertical_id: &str,
    decided: &ActionRecord,
    actor: &str,
    approval_id: &str,
) -> (&'static str, String) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());

    // (a) hash-chained ApprovalGranted row — the approval half of the loop.
    let granted = json!({
        "approval_id": approval_id,
        "decided_by": actor,
        "decision": "approve",
        "decided_at": decided.created_at,
        "action_type": decided.action_type,
    });
    if let Err(e) = sink
        .write(None, AuditKind::ApprovalGranted, granted, Some(actor.to_string()))
        .await
    {
        return server_error(&format!("approval granted audit: {e}"));
    }

    // (b) dispatch the approved action through its declared executor.
    let outcome =
        execute_approved_action_inline(store, memory, workspace, tenant, vertical_id, decided)
            .await;
    let (execution, detail) = match &outcome {
        Ok(()) => ("dispatched", String::new()),
        Err(e) => ("failed", e.to_string()),
    };

    // (c) hash-chained ApprovalConsumed row — the execution half of the loop.
    let consumed = json!({
        "approval_id": approval_id,
        "action_type": decided.action_type,
        "execution": execution,
        "detail": detail,
        "executed_by": actor,
    });
    if let Err(e) = sink
        .write(None, AuditKind::ApprovalConsumed, consumed, Some(actor.to_string()))
        .await
    {
        return server_error(&format!("approval consumed audit: {e}"));
    }

    (
        "200 OK",
        json!({
            "ok": true,
            "approval_id": approval_id,
            "decision": "approve",
            "execution": execution,
        })
        .to_string(),
    )
}
