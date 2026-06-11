//! Inline executor dispatch for the gateway "Run now" path
//! (DEC-050 G2 · `cu2-wire-executor-dispatch-into-gateway-run-path`).
//!
//! Before this module the gateway run path
//! ([`super::work_dispatch::process_job`]) emitted + classified a
//! teammate's actions and then STOPPED: `emit_and_persist_actions`
//! wrote each `ActionRecord` at `pending`/`blocked` and returned, so a
//! `computer_use` / `document_creation` / MCP action was inert — the
//! teammate "did 0 actions". Executor dispatch existed only in the CLI
//! (`argo worker actions execute` →
//! [`super::worker_actions_dispatch`]).
//!
//! This module closes that gap by REUSING — never forking — the exact
//! resolve + dispatch + transition helpers the CLI execute path uses:
//! [`super::worker_actions_executor_promotion::resolve_executor_with_promotion`]
//! (which is byte-for-byte [`super::worker_actions_executor::resolve_executor`]
//! when the `ARGO_EXECUTION_GRADUATION` flag is unset),
//! [`super::worker_actions_transitions::execute`] /
//! [`super::worker_actions_transitions::complete`] /
//! [`super::worker_actions_transitions::fail`], and the
//! [`super::worker_actions_executor::Executor`] trait.
//!
//! ## Gate (the load-bearing safety property)
//!
//! An emitted action auto-executes inline ONLY when EVERY one of these
//! holds (see [`is_auto_eligible_with_flag`]):
//!
//! 1. `ARGO_EXECUTION_GRADUATION` is enabled (DEC-041 master switch).
//!    **Unset — the shipped default — means zero inline dispatch**, so
//!    the gateway path behaves exactly as it did before this module:
//!    emit + classify, no execution. The flag flip is operator-gated.
//! 2. The row is still `pending` — a `blocked` row (DEC-028 red line /
//!    evidence-gate downgrade) is never auto-run.
//! 3. `requires_approval == false` — an approval-gated action stays
//!    held for the operator's approvals inbox; it is never auto-approved
//!    here.
//! 4. `risk_level == "low"` — only the safe tier auto-runs; medium /
//!    high stay held even when autonomous.
//!
//! Probation is honoured separately by an
//! [`super::worker_actions_auto_approval::ensure_circuit_allows_dispatch`]
//! check in [`dispatch_emitted_actions_inline`] before any transition.

use super::worker_actions::ActionRecord;
use super::worker_actions_auto_approval;
use super::worker_actions_executor::{ActionOutcome, Executor};
use super::worker_actions_executor_promotion::{
    execution_graduation_enabled, resolve_executor_for_approved, resolve_executor_with_promotion,
};
use super::worker_actions_transitions;
use argo_memory::MemoryStore;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use std::path::Path;
use std::sync::Arc;

/// Iterate the freshly-emitted action records and inline-dispatch every
/// auto-eligible one through its declared executor, persisting the
/// `executing → completed|failed` transition to the worker_action
/// ledger. Returns the count actually dispatched (0 when the
/// `ARGO_EXECUTION_GRADUATION` flag is unset — the shipped default).
///
/// Honours each gate before doing any work: the static eligibility gate
/// ([`is_auto_eligible`]) plus the per-worker circuit (probation) check.
/// A held action is left untouched at `pending`. A resolve or dispatch
/// error transitions the row to `failed` (never silently dropped) so the
/// operator sees the failure in the ledger.
pub(crate) async fn dispatch_emitted_actions_inline(
    store: &Arc<SessionStore>,
    memory: &MemoryStore,
    workspace: WorkspaceId,
    tenant: TenantId,
    vertical_id: &str,
    records: &[ActionRecord],
) -> usize {
    if !execution_graduation_enabled() {
        // Default-OFF fast path: don't even load the vertical registry.
        return 0;
    }
    let cwd = match std::env::current_dir() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("argo work: warning — inline dispatch skipped, cwd unavailable: {e}");
            return 0;
        }
    };
    let mut dispatched = 0usize;
    for rec in records {
        if !is_auto_eligible(rec) {
            continue;
        }
        // Probation gate: an open circuit pauses the worker. Best-effort
        // — a lookup failure leaves the action held rather than risking
        // a dispatch the circuit meant to stop.
        if let Err(e) =
            worker_actions_auto_approval::ensure_circuit_allows_dispatch(memory, workspace, &rec.action_id)
                .await
        {
            eprintln!(
                "argo work: action {} held — circuit gate refused dispatch: {e}",
                rec.action_id
            );
            continue;
        }
        // operator_approved=false: the resolver re-checks ARGO_EXECUTION_GRADUATION
        // itself, so the flag-off safety of this autonomous host-control path is
        // guarded BOTH here (the early-return above) AND inside the resolver —
        // two independent guards, not one (adversarial-review hardening 2026-06-11).
        let executor = match resolve_for(store, tenant, vertical_id, rec, false).await {
            Ok(ex) => ex,
            Err(e) => {
                eprintln!(
                    "argo work: action {} — could not resolve executor: {e}",
                    rec.action_id
                );
                continue;
            }
        };
        match run_one(memory, workspace, rec, executor.as_ref(), &cwd).await {
            Ok(()) => dispatched += 1,
            Err(e) => eprintln!(
                "argo work: action {} — inline dispatch failed: {e}",
                rec.action_id
            ),
        }
    }
    dispatched
}

/// Execute an action the operator has just EXPLICITLY APPROVED in the
/// `/approvals` inbox (cu5 · DEC-050 G5 — the approval→work loop).
///
/// Unlike the autonomous path [`dispatch_emitted_actions_inline`], this is
/// NOT gated by `ARGO_EXECUTION_GRADUATION` or the static
/// [`is_auto_eligible`] predicate: the operator's approval IS the
/// authorization, so an approval-gated / medium-or-high-risk action runs
/// here precisely because a human cleared it. The per-worker circuit
/// (probation) gate is still honoured — an open circuit holds the action
/// even after approval.
///
/// `rec` must already be in `approved` (or a `pending` autonomous-safe)
/// state; [`run_one`] drives `approved → executing → completed|failed`,
/// reusing the SAME resolve + dispatch + transition helpers the CLI
/// `argo worker actions execute` path uses. Returns `Ok(())` on a
/// completed / human-handoff dispatch and `Err` on a resolve / dispatch
/// failure (the row is transitioned to `failed` before the error returns).
pub(crate) async fn execute_approved_action_inline(
    store: &Arc<SessionStore>,
    memory: &MemoryStore,
    workspace: WorkspaceId,
    tenant: TenantId,
    vertical_id: &str,
    rec: &ActionRecord,
) -> ArgoResult<()> {
    worker_actions_auto_approval::ensure_circuit_allows_dispatch(memory, workspace, &rec.action_id)
        .await?;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    // operator_approved=true: the operator cleared this action in /approvals,
    // so promotions are honoured regardless of ARGO_EXECUTION_GRADUATION
    // (approval IS the authorization, #2818).
    let executor = resolve_for(store, tenant, vertical_id, rec, true).await?;
    run_one(memory, workspace, rec, executor.as_ref(), &cwd).await
}

/// Resolve the executor the bound vertical declares for `rec`'s
/// action_type, routing through the per-action execution-graduation
/// resolver (the flag is already known-on at the only call site). Mirrors
/// the CLI `invoke_executor_for` resolve, but keyed on the gateway run's
/// real `tenant` rather than the cwd-derived workspace.
async fn resolve_for(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    vertical_id: &str,
    rec: &ActionRecord,
    operator_approved: bool,
) -> ArgoResult<Box<dyn Executor>> {
    let dir = super::vertical::resolve_verticals_dir();
    // cu1g (DEC-050): genesis-aware resolve — flat catalog first, then the
    // tenant's own `.genesis/<tenant>/<vid>/` (fail-closed cross-tenant).
    // The flat-only registry lookup made every approved Genesis action
    // fail dispatch with "vertical not loaded".
    let loaded = argo_vertical::load_for_worker(&dir, &tenant.to_string(), vertical_id)
        .ok_or_else(|| {
            argo_types::ArgoError::Provider(format!(
                "inline dispatch: vertical '{vertical_id}' not loadable for tenant '{tenant}' — \
                 cannot resolve executor for action_type '{}'",
                rec.action_type
            ))
        })?;
    let worker_id = rec.worker_id.to_string();
    if operator_approved {
        // cu5 approve path: the operator cleared the action, so promotions
        // are honoured regardless of ARGO_EXECUTION_GRADUATION (#2818).
        resolve_executor_for_approved(store, tenant, &worker_id, &loaded.manifest, &rec.action_type)
            .await
    } else {
        // Autonomous path: defense-in-depth — the resolver re-checks the
        // graduation flag itself, so honouring a promotion flag-off would
        // require BOTH this guard AND the caller's early-return to fail
        // (adversarial-review hardening 2026-06-11).
        resolve_executor_with_promotion(store, tenant, &worker_id, &loaded.manifest, &rec.action_type)
            .await
    }
}

/// Drive one already-eligible record through the executor: transition
/// `pending → executing`, dispatch, then settle the row per the
/// [`ActionOutcome`]. Factored out so a test can inject a mock executor
/// without standing up the vertical registry or the env flag.
///
/// * [`ActionOutcome::Completed`] → `executing → completed` stamped with
///   the executor's `result_json`.
/// * [`ActionOutcome::HumanHandoff`] → left at `executing` (the operator
///   carries it out offline and runs `argo worker actions complete`).
/// * dispatch error → `executing → failed` with the error as the reason.
async fn run_one(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    rec: &ActionRecord,
    executor: &dyn Executor,
    cwd: &Path,
) -> ArgoResult<()> {
    // pending → executing. Autonomous-safe rows advance directly (the
    // eligibility gate already proved `requires_approval == false`).
    let executing = worker_actions_transitions::execute(memory, workspace, &rec.action_id).await?;
    match executor.dispatch(&executing, cwd) {
        Ok(ActionOutcome::Completed { result_json, .. }) => {
            let result_str =
                serde_json::to_string(&result_json).unwrap_or_else(|_| "{}".into());
            let note = format!("auto-completed by inline executor for `{}`", rec.action_type);
            worker_actions_transitions::complete(
                memory,
                workspace,
                &rec.action_id,
                Some(&note),
                Some(&result_str),
            )
            .await?;
            Ok(())
        }
        Ok(ActionOutcome::HumanHandoff { .. }) => Ok(()),
        Err(e) => {
            let reason = format!("inline executor dispatch error: {e}");
            worker_actions_transitions::fail(memory, workspace, &rec.action_id, &reason).await?;
            Err(e)
        }
    }
}

/// Static eligibility gate, reading the live `ARGO_EXECUTION_GRADUATION`
/// flag. See [`is_auto_eligible_with_flag`] for the rule.
fn is_auto_eligible(rec: &ActionRecord) -> bool {
    is_auto_eligible_with_flag(rec, execution_graduation_enabled())
}

/// Pure eligibility predicate with the graduation flag passed
/// explicitly, so the gate is unit-testable without mutating process
/// environment (which races across parallel tests — same reason
/// [`super::worker_actions_executor_promotion`] factors its flag parse
/// out). An action auto-executes inline ONLY when all hold:
/// graduation-enabled AND `pending` AND not approval-gated AND low-risk.
fn is_auto_eligible_with_flag(rec: &ActionRecord, graduation_enabled: bool) -> bool {
    graduation_enabled
        && rec.state == "pending"
        && !rec.requires_approval
        && rec.risk_level == "low"
}

#[cfg(test)]
#[path = "work_dispatch_executor_inline_tests.rs"]
mod tests;
