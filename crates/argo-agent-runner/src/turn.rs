//! Multi-turn agent loop driver.
//!
//! [`run_turn`] / [`run_turn_with_events`] orchestrate one
//! conversation turn against an open `SessionStore`: optionally
//! append a user message, call the provider, dispatch any tool
//! calls (capability-checked), feed the tool results back, repeat
//! up to `max_rounds`. Each step is hash-chained into the audit
//! log and (when pricing is known) recorded into `spend_ledger`.
//!
//! Streaming (`stream = true`) prints assistant text to stdout as
//! it arrives but otherwise runs the same multi-round loop.
//!
//! The inner tool-dispatch loop (`run_turn_with_tools`) lives in
//! the sibling `turn_helpers.rs`. The orchestration body for
//! `run_turn_with_events_inner` lives in `turn_run_loop.rs`, the
//! context-window sizing policy in `turn_context_policy.rs`, and
//! system-prompt composition in `turn_system_prompt.rs`, all so
//! this file stays under the 500-LOC ceiling (HARD RULE 6); see
//! also the `claude-agent-runner-turn-split` and
//! `codex-loc-budget-followup-wave3` briefs.
use crate::helpers::RunEventSink;
use argo_store::SessionStore;
// `TenantId` is unused in this parent file post-split, but the
// `turn_tests` sibling does `use super::*` and needs it in scope.
#[cfg(test)]
#[allow(unused_imports)]
use argo_types::TenantId;
use argo_types::{ArgoResult, SessionId};
use std::path::Path;
use std::sync::Arc;
use tracing::Instrument;
use uuid::Uuid;

/// Default cap on tool-dispatch rounds within a single `run_turn`
/// when callers don't pass an explicit `max_rounds`. Generous
/// enough for legitimate planning chains; small enough that an
/// accidental loop fails fast.
///
/// ```
/// use argo_agent_runner::MAX_TOOL_ROUNDS;
/// assert_eq!(MAX_TOOL_ROUNDS, 8);
/// ```
pub const MAX_TOOL_ROUNDS: usize = 8;

/// Outcome of one agent-loop turn. Consumed by both the CLI
/// handler (which prints) and the gateway's
/// `POST /sessions/<id>/runs` endpoint (which serializes to JSON).
///
/// ```
/// use argo_agent_runner::TurnResult;
/// use uuid::Uuid;
/// let r = TurnResult {
///     assistant_id: Uuid::nil(),
///     text: "hello".into(),
///     tokens_in: Some(10),
///     tokens_out: Some(5),
///     stopped_on_tool_budget: false,
/// };
/// assert_eq!(r.text, "hello");
/// assert!(!r.stopped_on_tool_budget);
/// ```
pub struct TurnResult {
    /// Persisted assistant message id.
    pub assistant_id: Uuid,
    /// Full assistant text.
    pub text: String,
    /// Provider-reported input tokens, if any.
    pub tokens_in: Option<u32>,
    /// Provider-reported output tokens, if any.
    pub tokens_out: Option<u32>,
    /// `true` when the loop exited because `max_rounds` was hit
    /// (the model still wanted more tool dispatch). The session
    /// state is consistent — every dispatched tool has its
    /// matching `tool_result` persisted — so the caller can
    /// safely issue another `run_turn` to continue. Worker v2's
    /// outer step loop uses this signal to convert a research-
    /// heavy turn into a clean step boundary instead of a
    /// transport error (#724).
    pub stopped_on_tool_budget: bool,
}

/// One agent-loop turn against an open SessionStore.
///
/// Reusable from both the CLI handler and the gateway's
/// `POST /sessions/<id>/runs` endpoint. When `stream = true` the
/// assistant text is flushed to stdout as it arrives — streaming
/// stays single-turn for now (no tool dispatch). Non-streaming
/// runs the full tool-dispatch loop, capped at
/// [`MAX_TOOL_ROUNDS`] rounds.
pub async fn run_turn(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    user: Option<&str>,
    stream: bool,
    max_rounds: usize,
) -> ArgoResult<TurnResult> {
    run_turn_with_events(store, cwd, session_id, user, stream, max_rounds, None).await
}

/// Like [`run_turn_with_events`], but consults a [`PlannerHook`]
/// before the LLM round loop so coarse goals get a planner pass
/// and the resulting subtasks land in the [`SubtaskSink`] for the
/// existing handoff walker to pick up
/// (mission-planner-dispatch-loop-wire). When `hook` is `None` or
/// `user` is `None` (i.e. a "continue" turn carries no fresh
/// goal) the call degrades to the regular [`run_turn_with_events`]
/// path. Decomposition errors never bubble — see
/// [`crate::turn_planner::maybe_decompose`].
#[allow(clippy::too_many_arguments)]
pub async fn run_turn_with_planner_hook(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    user: Option<&str>,
    stream: bool,
    max_rounds: usize,
    events: Option<&RunEventSink>,
    hook: Option<&turn_planner::PlannerHook<'_>>,
) -> ArgoResult<TurnResult> {
    if let (Some(text), Some(h)) = (user, hook) {
        let parent = session_id.0.to_string();
        let _ =
            turn_planner::maybe_decompose(h.planner, h.gate, h.sink, Some(&parent), text, &h.ctx)
                .await;
    }
    run_turn_with_events(store, cwd, session_id, user, stream, max_rounds, events).await
}

/// Like `run_turn`, but with an optional `events` sink. The gateway
/// streaming endpoint passes a real `UnboundedSender`; the CLI +
/// non-streaming gateway pass `None`. Each event mirrors the audit
/// emission that just landed.
///
/// Wrapped in the canonical agent turn span so every emission
/// inside the agent loop inherits `session_id`, `tenant_id`, and
/// `worker_id` without per-callsite changes
/// (mvp-structured-logging). `tenant_id`/`worker_id` are recorded
/// after the session row loads.
#[allow(clippy::too_many_arguments)]
pub async fn run_turn_with_events(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    user: Option<&str>,
    stream: bool,
    max_rounds: usize,
    events: Option<&RunEventSink>,
) -> ArgoResult<TurnResult> {
    let span = tracing::info_span!(
        argo_telemetry::spans::AGENT_TURN,
        session_id = %session_id.0,
        tenant_id = tracing::field::Empty,
        worker_id = tracing::field::Empty,
    );
    turn_run_loop::run_turn_with_events_inner(
        store, cwd, session_id, user, stream, max_rounds, events,
    )
    .instrument(span)
    .await
}

#[path = "turn_helpers.rs"]
mod turn_helpers;

#[path = "turn_helpers_context.rs"]
mod turn_helpers_context;

#[path = "turn_helpers_freeze.rs"]
mod turn_helpers_freeze;

#[path = "turn_helpers_overlay.rs"]
mod turn_helpers_overlay;

#[path = "turn_helpers_run.rs"]
mod turn_helpers_run;

#[path = "turn_helpers_truth_boundary.rs"]
mod turn_helpers_truth_boundary;

#[cfg(test)]
#[path = "turn_helpers_truth_boundary_tests.rs"]
mod turn_helpers_truth_boundary_tests;

#[path = "turn_canary.rs"]
mod turn_canary;

#[path = "turn_memory.rs"]
mod turn_memory;

#[path = "turn_helpers_workplace.rs"]
mod turn_helpers_workplace;

#[path = "turn_skills_runtime.rs"]
mod turn_skills_runtime;

#[path = "turn_skills.rs"]
mod turn_skills;

#[cfg(test)]
#[path = "turn_skills_tests.rs"]
mod turn_skills_tests;

#[cfg(test)]
#[path = "turn_library_integration_tests.rs"]
mod turn_library_integration_tests;

#[path = "turn_planner.rs"]
pub mod turn_planner;

#[cfg(test)]
#[path = "turn_planner_tests.rs"]
mod turn_planner_tests;

#[path = "turn_skills_migration.rs"]
mod turn_skills_migration;

#[path = "turn_context_policy.rs"]
mod turn_context_policy;

#[path = "turn_system_prompt.rs"]
mod turn_system_prompt;

#[path = "turn_run_loop.rs"]
mod turn_run_loop;

// Re-exports so the post-split sibling modules (turn_helpers_run,
// turn_helpers_context, turn_tests) continue to access these
// through `super::*` without code changes.
pub(crate) use turn_context_policy::{
    load_context_compress_config, model_context_window, ContextWindowPolicy,
};
pub(crate) use turn_system_prompt::build_system_prompt_with_ab;
#[cfg(test)]
pub(crate) use turn_system_prompt::build_system_prompt;
pub use turn_system_prompt::compose_system_prompt_for_diagnostics;

#[cfg(test)]
#[path = "turn_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "turn_tests_max_rounds.rs"]
mod tests_max_rounds;

#[cfg(test)]
#[path = "turn_tests_freeze.rs"]
mod tests_freeze;
