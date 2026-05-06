//! Stream H-2 — auto-approval gate hookup.
//!
//! Wraps the pure evaluator from
//! `argo_identity::auto_approval::evaluate` with the cli-side
//! glue:
//!
//! - Loads the workspace's [`AutoApprovalPolicy`] from
//!   `.skycot/auto_approval.toml` (defaults to disabled when the
//!   file is absent — preserves the v0 "approval-required" UX).
//! - Builds a [`PendingAction`] from an [`ActionRecord`] + the
//!   manifest's [`ActionDeclaration`] so the evaluator sees the
//!   action's risk class + estimated spend.
//! - Returns the [`Decision`] verbatim — the dispatch shim
//!   reads it and either auto-approves, blocks, or falls
//!   through to the existing approval-required path.
//!
//! H-3 (orchestrator) will replace the workspace-level policy
//! file with a per-worker policy threaded from the
//! `argo-identity` Worker. v1 keeps the surface
//! workspace-scoped so the gate lands without pulling in
//! identity threading first.

use super::worker_actions::ActionRecord;
use super::worker_actions_transitions;
use argo_identity::auto_approval::{
    dollars_to_micros, evaluate, Decision, PendingAction, SpendSnapshot,
};
use argo_identity::AutoApprovalPolicy;
use argo_memory::MemoryStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use argo_vertical::ActionDeclaration;
use chrono::Utc;
use std::path::Path;

/// Default location of the workspace-level policy file. Resolved
/// relative to the current working directory at dispatch time.
pub(crate) const POLICY_RELPATH: &str = ".skycot/auto_approval.toml";

/// Load the workspace policy from `.skycot/auto_approval.toml`.
/// Returns the default (disabled) policy when the file is
/// absent — every approval-gated action goes to a human, which
/// is exactly the v0 behaviour the cli has shipped to date.
pub(crate) fn load_workspace_policy(cwd: &Path) -> ArgoResult<AutoApprovalPolicy> {
    let path = cwd.join(POLICY_RELPATH);
    if !path.exists() {
        return Ok(AutoApprovalPolicy::default());
    }
    let body = std::fs::read_to_string(&path).map_err(|e| {
        ArgoError::Provider(format!(
            "auto-approval policy at {} could not be read: {e}",
            path.display()
        ))
    })?;
    toml::from_str(&body).map_err(|e| {
        ArgoError::Provider(format!(
            "auto-approval policy at {} is not valid TOML: {e}",
            path.display()
        ))
    })
}

/// Run the policy on a pending action. Pure wrapper around
/// `argo_identity::auto_approval::evaluate` + the cli-side
/// `PendingAction` constructor.
pub(crate) fn decide(
    policy: &AutoApprovalPolicy,
    action: &ActionRecord,
    decl: Option<&ActionDeclaration>,
    spend: SpendSnapshot,
) -> Decision {
    let estimated_usd_micros = decl
        .and_then(|d| d.executor.budget_usd())
        .map(dollars_to_micros);
    let pending = PendingAction {
        risk_level: action.risk_level.as_str(),
        requires_approval: action.requires_approval,
        estimated_usd_micros,
        action_type: action.action_type.as_str(),
    };
    evaluate(policy, &pending, spend, Utc::now())
}

/// Render a stable `approved_by` token for the audit row when
/// the gate auto-approves. Mirrors the brief's
/// `approved_by = "auto:<policy>"` requirement.
#[must_use]
pub(crate) fn auto_approver_token(reason: &str) -> String {
    format!("auto:{reason}")
}

/// Convert a [`Decision::Block`] verdict into an `ArgoError` the
/// dispatch shim can return. Centralised so the message format
/// stays consistent across call sites.
pub(crate) fn block_to_error(action_id: &str, reason: &str) -> ArgoError {
    ArgoError::Provider(format!(
        "action {action_id} blocked by auto-approval policy: {reason}"
    ))
}

/// Run the auto-approval gate. Called by
/// `worker_actions_dispatch::execute` BEFORE the
/// `pending → executing` transition. Auto-approves an
/// approval-gated action when the policy clears it (writing a
/// real `approved_by = "auto:<reason>"` audit row); falls
/// through on Ask; errors on Block.
pub(crate) async fn maybe_auto_approve(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
) -> ArgoResult<()> {
    let located = worker_actions_transitions::find_action(memory, workspace, action_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no action with action_id '{action_id}'")))?;
    if located.record.state != "pending" {
        return Ok(());
    }
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let policy = load_workspace_policy(&cwd)?;
    let decl = manifest_decl_for(&located.record.action_type);
    // v1 SpendSnapshot is zero — H-3 wires the budget ledger
    // reads in once the SqlitePool plumbing is threaded
    // through dispatch.
    let decision = decide(&policy, &located.record, decl.as_ref(), SpendSnapshot::zero());
    match decision {
        Decision::AutoApprove { reason } => {
            let approver = auto_approver_token(&reason);
            let note = format!("auto-approved: {reason}");
            worker_actions_transitions::approve(memory, workspace, action_id, &approver, Some(&note))
                .await?;
            Ok(())
        }
        Decision::Ask { .. } => Ok(()),
        Decision::Block { reason } => Err(block_to_error(action_id, &reason)),
    }
}

/// Look up the action's manifest declaration. Used to read the
/// executor's `budget_usd` when scoring the per-action cap.
/// Returns `None` when the action_type isn't declared in any
/// loaded vertical (the rest of the dispatch path will surface
/// the same error).
fn manifest_decl_for(action_type: &str) -> Option<ActionDeclaration> {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir).ok()?;
    let found = registry.list().find_map(|(_, loaded)| {
        loaded
            .manifest
            .actions
            .as_ref()
            .and_then(|a| a.actions.get(action_type))
            .cloned()
    });
    found
}

#[cfg(test)]
#[path = "worker_actions_auto_approval_tests.rs"]
mod tests;
