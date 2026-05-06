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
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::auto_approval::{
    dollars_to_micros, evaluate, Decision, PendingAction, SpendSnapshot,
};
use argo_identity::{circuit, worker_budget, AutoApprovalPolicy, CircuitState};
use argo_memory::MemoryStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use argo_vertical::ActionDeclaration;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite};
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
#[cfg(test)]
pub(crate) fn decide(
    policy: &AutoApprovalPolicy,
    action: &ActionRecord,
    decl: Option<&ActionDeclaration>,
    spend: SpendSnapshot,
) -> Decision {
    decide_at(policy, action, decl, spend, Utc::now())
}

fn decide_at(
    policy: &AutoApprovalPolicy,
    action: &ActionRecord,
    decl: Option<&ActionDeclaration>,
    spend: SpendSnapshot,
    now: DateTime<Utc>,
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
    evaluate(policy, &pending, spend, now)
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

/// Refuse dispatch when the worker's circuit is open. Half-open
/// is allowed through so the operator-resumed probe can run.
pub(crate) async fn ensure_circuit_allows_dispatch(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
) -> ArgoResult<()> {
    let located = worker_actions_transitions::find_action(memory, workspace, action_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no action with action_id '{action_id}'")))?;
    let worker_id = worker_id_for_action(&located.record);
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let store = open_budget_store(&cwd).await?;
    match circuit::state_for(store.pool(), &worker_id).await? {
        CircuitState::Open { reason, .. } => Err(ArgoError::Provider(format!(
            "circuit_open: worker {worker_id} paused: {reason}"
        ))),
        CircuitState::Closed | CircuitState::HalfOpen => Ok(()),
    }
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
    // Autonomous-safe actions (manifest's `requires_approval = false`)
    // bypass the gate entirely. The existing dispatch path advances
    // them pending → executing directly without an approve step;
    // calling `approve` here would error because `approve` rejects
    // autonomous-safe rows ("use `execute` to advance directly").
    // Regression caught by the chain_result_propagation integration
    // test on a vertical with autonomous-safe verify_price.
    if !located.record.requires_approval {
        return Ok(());
    }
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let workspace_policy = load_workspace_policy(&cwd)?;
    // Stream `claude-policy-as-code` — layer workspace policy
    // files over the explicit AutoApprovalPolicy. Soft-fails on
    // corrupt YAML so a single bad file never breaks dispatch
    // (the operator-facing CLI surface ships errors loudly via
    // `argo policy validate`). Role + tenant context aren't yet
    // threaded through the gate — wildcard selectors
    // (`Tenant: '*'`) catch all workers for v1; per-worker
    // selectors land in a v1.x follow-up that adds the worker
    // lookup before `decide_at`.
    let policy_files =
        argo_identity::policy::load_policy_dir(&cwd.join(crate::handlers::policy_handlers::POLICY_DIR))
            .unwrap_or_default();
    let effective = argo_identity::policy::resolve_effective_policy(
        &workspace_policy,
        &policy_files,
        "",
        "",
        &located.record.action_type,
    );
    let policy = effective.auto_approval;
    let decl = manifest_decl_for(&located.record.action_type);
    let store = open_budget_store(&cwd).await?;
    maybe_auto_approve_with_pool(
        memory,
        workspace,
        action_id,
        &located.record,
        decl.as_ref(),
        &policy,
        store.pool(),
        Utc::now(),
    )
    .await?;
    Ok(())
}

async fn open_budget_store(cwd: &Path) -> ArgoResult<SessionStore> {
    let home = resolve_home(cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

#[allow(clippy::too_many_arguments)]
async fn maybe_auto_approve_with_pool(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_id: &str,
    action: &ActionRecord,
    decl: Option<&ActionDeclaration>,
    policy: &AutoApprovalPolicy,
    pool: &Pool<Sqlite>,
    now: DateTime<Utc>,
) -> ArgoResult<Decision> {
    if !action.requires_approval {
        return Ok(decide_at(policy, action, decl, SpendSnapshot::zero(), now));
    }
    let worker_id = worker_id_for_action(action);
    let today_usd_micros = worker_budget::spend_today_for_worker(pool, &worker_id, now).await?;
    let spend = SpendSnapshot {
        today_usd_micros,
        this_month_usd_micros: 0,
    };
    let decision = decide_at(policy, action, decl, spend, now);
    match &decision {
        Decision::AutoApprove { reason } => {
            let approver = auto_approver_token(reason);
            let note = format!("auto-approved: {reason}");
            worker_actions_transitions::approve(
                memory,
                workspace,
                action_id,
                &approver,
                Some(&note),
            )
            .await?;
            worker_budget::record_charge(
                pool,
                action_id,
                &worker_id,
                estimated_cost_micros(decl),
                now,
            )
            .await?;
            Ok(decision)
        }
        Decision::Ask { .. } => Ok(decision),
        Decision::Block { reason } => Err(block_to_error(action_id, reason)),
    }
}

fn estimated_cost_micros(decl: Option<&ActionDeclaration>) -> u64 {
    decl.and_then(|d| d.executor.budget_usd())
        .map(dollars_to_micros)
        .unwrap_or(0)
}

pub(crate) fn worker_id_for_action(action: &ActionRecord) -> String {
    action
        .payload
        .get("worker_id")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            action
                .payload
                .get("worker")
                .and_then(|worker| worker.get("id"))
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or(&action.job_id)
        .to_string()
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
