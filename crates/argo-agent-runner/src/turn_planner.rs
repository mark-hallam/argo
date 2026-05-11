//! Optional planner hook for the agent dispatch loop. When the
//! agent receives a coarse goal (the worker's task description /
//! mission statement), [`maybe_decompose`] runs the gate →
//! planner → [`SubtaskSink`] chain so the existing handoff walker
//! can pick up the decomposed children via the persisted subtask
//! ledger rows (mission-planner-dispatch-loop-wire).
//!
//! Loaded by `turn.rs` via the same `#[path = "..."]` sibling
//! pattern used for `turn_helpers.rs`. Kept narrow so `turn.rs`
//! stays under the 500-LOC ceiling (HARD RULE 6).
//!
//! Failure semantics: every failure mode degrades to a logged
//! [`DecomposeOutcome::*Error`] and the dispatch loop falls
//! through to its existing single-task path. The hook NEVER
//! returns `Err`; a flaky planner / sink / gate must not crash a
//! turn.

use argo_runtime::{
    record_decomposition, DecomposeGate, Planner, PlannerContext, SubtaskSink,
};

/// Bundle of dependencies the planner hook needs. Held by the
/// new [`crate::run_turn_with_planner_hook`] entry point and
/// passed to [`maybe_decompose`].
///
/// Lifetime-borrowed so callers can keep the planner / gate /
/// sink in their own `Arc`s without forcing the agent loop to
/// own them. The [`PlannerContext`] is owned because it's
/// per-turn (carries the worker id) and cheap to clone.
pub struct PlannerHook<'a> {
    /// Decompose the goal when the gate says so.
    pub planner: &'a dyn Planner,
    /// Decide whether to call the planner at all.
    pub gate: &'a dyn DecomposeGate,
    /// Persist the resulting subtask rows so the walker picks
    /// them up.
    pub sink: &'a dyn SubtaskSink,
    /// Per-turn context (worker id, vertical, hints).
    pub ctx: PlannerContext,
}

impl std::fmt::Debug for PlannerHook<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlannerHook")
            .field("worker_id", &self.ctx.worker_id)
            .field("vertical", &self.ctx.vertical)
            .finish_non_exhaustive()
    }
}

/// Outcome of one [`maybe_decompose`] call. Returned for tests
/// and to let callers emit follow-up tracing events. The agent
/// dispatch loop discards it — the side-effect (subtask rows
/// written via the sink) is what matters at runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecomposeOutcome {
    /// Gate said "no" — single-task dispatch should proceed.
    SkippedByGate,
    /// Gate errored — treated as `false` and logged at warn.
    GateError,
    /// Planner produced `children` subtasks AND the sink
    /// accepted every row. The walker picks them up.
    Decomposed {
        /// Number of subtasks the planner emitted.
        children: usize,
    },
    /// Gate said "yes" but the planner errored. Logged at warn;
    /// single-task dispatch proceeds.
    PlannerError,
    /// Planner produced subtasks but the sink rejected one.
    /// Logged at warn; single-task dispatch proceeds.
    SinkError {
        /// Number of subtasks the planner emitted before the
        /// sink failed (one or more rows may have been
        /// persisted depending on the sink impl).
        children: usize,
    },
}

/// Run the gate → planner → sink chain for `goal`. Never returns
/// `Err`: every failure mode degrades to a logged
/// [`DecomposeOutcome`] variant so the dispatch loop can fall
/// through to its existing single-task behavior.
///
/// The gate is consulted exactly once per call. When it returns
/// `Ok(true)` the planner runs and every emitted subtask is
/// passed through [`record_decomposition`]; on success a
/// `tracing::info!` event with the child count is emitted so
/// `/audit/by-decomposition` rollups become possible (#50).
pub async fn maybe_decompose(
    planner: &dyn Planner,
    gate: &dyn DecomposeGate,
    sink: &dyn SubtaskSink,
    parent_session_id: Option<&str>,
    goal: &str,
    ctx: &PlannerContext,
) -> DecomposeOutcome {
    match gate.should_decompose(goal, ctx).await {
        Ok(false) => return DecomposeOutcome::SkippedByGate,
        Err(e) => {
            tracing::warn!(error = %e, goal_len = goal.len(), "decompose_gate_error");
            return DecomposeOutcome::GateError;
        }
        Ok(true) => {}
    }
    let result = match planner.decompose(goal, ctx).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, goal_len = goal.len(), "planner_decompose_error");
            return DecomposeOutcome::PlannerError;
        }
    };
    let children = result.workflow.tasks.len();
    if let Err(e) = record_decomposition(sink, goal, parent_session_id, ctx, &result).await {
        tracing::warn!(error = %e, children, "subtask_sink_error");
        return DecomposeOutcome::SinkError { children };
    }
    tracing::info!(
        parent = %goal_preview(goal),
        children,
        planner_kind = %result.planner_kind,
        "goal_decomposed"
    );
    DecomposeOutcome::Decomposed { children }
}

/// Trim a goal to a 60-char preview for the structured tracing
/// event. Long goals would otherwise dominate the log line and
/// blow out audit rollups.
fn goal_preview(goal: &str) -> String {
    const PREVIEW_MAX: usize = 60;
    let trimmed = goal.trim();
    if trimmed.chars().count() <= PREVIEW_MAX {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(PREVIEW_MAX).collect();
    out.push('…');
    out
}
