//! Worker v3 action ledger CLI dispatch.
//!
//! The wire schema (`ActionState`, `ActionRecord`) lives in
//! `worker_actions_schema.rs`; the marketplace `ActionType` enum
//! lives in `worker_actions_type.rs`. Both are re-exported below so
//! callers keep using `super::worker_actions::ActionState` etc.

use crate::cli::cli_worker_cmds::WorkerCmd;
use argo_telemetry::span_context::{enter_context, LogContext};
use argo_types::ArgoResult;

// Re-export short() so the existing tests in `worker_actions_tests.rs`
// can keep calling it via `super::short` after the
// `claude-worker-actions-split` refactor.
#[cfg(test)]
pub(crate) use super::worker_actions_query::short;

// `ActionState` + `ActionRecord` live in `worker_actions_schema.rs`;
// `ActionType` lives in `worker_actions_type.rs`. They are re-exported
// here so callers (and the unit tests below via `use super::*`) keep
// using `super::worker_actions::ActionState` / `ActionRecord` /
// `ActionType` unchanged.
#[allow(unused_imports)]
pub(crate) use super::worker_actions_schema::{ActionRecord, ActionState};
#[allow(unused_imports)]
pub(crate) use super::worker_actions_type::ActionType;

// ───────────────────────── CLI ───────────────────────────

pub async fn run(cmd: WorkerCmd) -> ArgoResult<()> {
    // Worker-loop iteration boundary: every CLI invocation of a
    // `worker <subcommand>` is one logical iteration. Extract the
    // worker_id from the subcommand args (when present) so log lines
    // emitted under the handler carry it via the
    // ContextInjectorLayer.
    let _worker_ctx = enter_context(LogContext {
        worker_id: worker_id_from_cmd(&cmd),
        ..LogContext::default()
    });
    super::worker_actions_run_dispatch::dispatch(cmd).await
}

/// Pull the `worker_id` (if any) off the dispatched subcommand so the
/// [`enter_context`] guard at the top of [`run`] carries it. Returns
/// `None` for subcommands that don't address a specific worker (fleet
/// listing, elasticity status, etc.) — the
/// [`ContextInjectorLayer`](argo_telemetry::span_context) then emits
/// the configured `worker_id=-` marker.
fn worker_id_from_cmd(cmd: &WorkerCmd) -> Option<String> {
    match cmd {
        WorkerCmd::Show { worker_id, .. }
        | WorkerCmd::Portrait { worker_id, .. }
        | WorkerCmd::Update { worker_id, .. }
        | WorkerCmd::Resume { worker_id, .. }
        | WorkerCmd::Freeze { worker_id, .. }
        | WorkerCmd::Unfreeze { worker_id, .. }
        | WorkerCmd::Anomalies { worker_id, .. }
        | WorkerCmd::Rehydrate { worker_id, .. }
        | WorkerCmd::Delete { worker_id, .. }
        | WorkerCmd::Promote { worker_id, .. }
        | WorkerCmd::Level { worker_id, .. }
        | WorkerCmd::Skills { worker_id, .. } => Some(worker_id.clone()),
        WorkerCmd::Onboard { id, .. } => id.clone(),
        _ => None,
    }
}

#[cfg(test)]
#[path = "worker_actions_tests.rs"]
mod tests;
