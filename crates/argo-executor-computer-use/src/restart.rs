//! Restart strategy for the CU executor: snapshot / reset /
//! retry-with-context on transient failures, capped at a per-action
//! attempt budget.
//!
//! Today, when a CU executor hits an unrecoverable error mid-trajectory
//! (model emits a malformed action, sandbox session times out, screenshot
//! capture fails) the existing path returns `HumanHandoff` and the run
//! dies. The most common failure modes are transient — a stale tab, a
//! renderer hiccup, a model response that referenced coordinates from an
//! earlier screenshot — all of which a fresh sandbox or a re-prompt
//! recovers cleanly.
//!
//! This module codifies that recovery as three discrete strategies the
//! operator opts in to via the executor manifest:
//!
//! - [`RestartStrategy::None`] — pre-existing behaviour: one-shot
//!   dispatch, fall through to [`crate::Outcome::HumanHandoff`] on
//!   transient failure.
//! - [`RestartStrategy::RetryWithContext`] — re-prompt the *same*
//!   sandbox session up to `max_attempts` times. Trajectory log is
//!   preserved across attempts, an optional `backoff_ms` sleeps between
//!   them. Suits transient model parse failures.
//! - [`RestartStrategy::SnapshotResetRetry`] — periodically snapshots
//!   loop state (every `snapshot_every_n_steps` steps); on transient
//!   failure tears down the sandbox, opens a fresh one, and re-seeds it
//!   from the most recent snapshot. Suits a stale or wedged sandbox.
//!
//! The flow inside `executor::run_loop` is:
//!
//! 1. After every successful CU step, call
//!    [`RestartCoordinator::should_snapshot`] / [`record_snapshot`] to
//!    keep the snapshot buffer current.
//! 2. On a per-step error (Anthropic call, sandbox action), call
//!    [`RestartCoordinator::should_restart`].
//!    - [`RestartDecision::Continue`] → existing handoff path.
//!    - [`RestartDecision::Retry`] → restart per `with_context`.
//!    - [`RestartDecision::Bail`] → retry budget exhausted, handoff.
//!
//! Retry budget is *cumulative per dispatch*: a successful retry does
//! not reset the counter, so worst-case runtime is bounded by the
//! configured `max_attempts`.

use crate::CuError;
use argo_types::ArgoResult;
use serde::{Deserialize, Serialize};

/// Per-profile restart strategy. Lives in
/// `manifests/executors/computer_use.toml` under
/// `[profiles.<name>.restart]`. Default [`RestartStrategy::None`]
/// preserves pre-existing behaviour.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, tag = "kind", rename_all = "snake_case")]
pub enum RestartStrategy {
    /// No restart. The first transient failure falls through to the
    /// existing `HumanHandoff` path.
    #[default]
    None,
    /// Re-prompt the same sandbox session up to `max_attempts` times.
    /// Trajectory log is preserved across attempts; `backoff_ms` sleeps
    /// between them so a flaky upstream has time to recover.
    RetryWithContext {
        /// Cap on retries per dispatch. Hit → fall through to handoff.
        max_attempts: u32,
        /// Delay before each retry, milliseconds.
        backoff_ms: u32,
    },
    /// Snapshot + reset. Tears down the current sandbox, opens a fresh
    /// one, and re-seeds it from the most recent snapshot. Snapshots
    /// happen every `snapshot_every_n_steps` successful steps; up to
    /// `max_attempts` resets per dispatch.
    SnapshotResetRetry {
        /// Cap on resets per dispatch.
        max_attempts: u32,
        /// Snapshot cadence. `0` disables snapshots (the loop still
        /// honours `max_attempts` but has no fallback context to seed).
        snapshot_every_n_steps: u32,
    },
}

/// Coordinator decision returned by
/// [`RestartCoordinator::should_restart`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartDecision {
    /// No restart applies. Either the strategy is
    /// [`RestartStrategy::None`] or the error is fatal (not in the
    /// retryable set). Caller falls through to the existing handoff
    /// path unchanged.
    Continue,
    /// Restart now. `with_context = true` keeps the current sandbox
    /// session and re-prompts (RetryWithContext mode). `false`
    /// requires a fresh sandbox seeded from the most recent snapshot
    /// (SnapshotResetRetry mode).
    Retry {
        /// Whether to keep the existing sandbox session.
        with_context: bool,
    },
    /// Retry budget exhausted. Caller falls through to handoff.
    Bail,
}

/// Frozen snapshot of CU loop state. Used by
/// [`RestartStrategy::SnapshotResetRetry`] to seed a fresh sandbox so
/// the model has continuity.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// Latest screenshot bytes captured before the failure point.
    pub last_screenshot: Vec<u8>,
    /// Step count at snapshot time. The restart resumes from this
    /// index so the trajectory log + audit row numbering is consistent.
    pub last_action_idx: u32,
    /// Original goal description plus the last 200 chars of the
    /// trajectory log. Re-prompted into the fresh sandbox's system
    /// prompt; no model call required to build it.
    pub goal_summary: String,
}

/// Mid-dispatch coordinator. Holds the strategy + per-action attempt
/// counter + the most recent snapshot. Built once per dispatch by the
/// executor; mutated by the loop on each step / failure.
#[derive(Debug)]
pub struct RestartCoordinator {
    strategy: RestartStrategy,
    attempts: u32,
    last_snapshot: Option<Snapshot>,
}

impl RestartCoordinator {
    /// Build for the given strategy with zero attempts logged.
    #[must_use]
    pub fn new(strategy: RestartStrategy) -> Self {
        Self {
            strategy,
            attempts: 0,
            last_snapshot: None,
        }
    }

    /// Read-only accessor for the configured strategy. Used by
    /// telemetry to label `cu.restart` events with the mode.
    #[must_use]
    pub fn strategy(&self) -> &RestartStrategy {
        &self.strategy
    }

    /// Decide whether to restart on the given failure. Charges one
    /// attempt against the budget when returning [`RestartDecision::Retry`].
    ///
    /// Decision tree:
    /// - [`RestartStrategy::None`] → always [`RestartDecision::Continue`].
    /// - Fatal error (see [`is_retryable_error`]) → [`RestartDecision::Continue`].
    /// - Attempts ≥ `max_attempts` → [`RestartDecision::Bail`].
    /// - Otherwise → [`RestartDecision::Retry { with_context }`].
    pub fn should_restart(&mut self, err: &CuError) -> RestartDecision {
        if matches!(self.strategy, RestartStrategy::None) {
            return RestartDecision::Continue;
        }
        if !is_retryable_error(err) {
            return RestartDecision::Continue;
        }
        let max = max_attempts(&self.strategy);
        if self.attempts >= max {
            return RestartDecision::Bail;
        }
        self.attempts = self.attempts.saturating_add(1);
        let with_context = matches!(self.strategy, RestartStrategy::RetryWithContext { .. });
        RestartDecision::Retry { with_context }
    }

    /// Attempts charged so far this dispatch.
    #[must_use]
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Record a snapshot. Caller honours the cadence via
    /// [`Self::should_snapshot`].
    pub fn record_snapshot(&mut self, snap: Snapshot) {
        self.last_snapshot = Some(snap);
    }

    /// True when the configured strategy wants a snapshot at the given
    /// step. Returns `false` for non-snapshot strategies and for step
    /// 0 (no useful state to capture yet).
    #[must_use]
    pub fn should_snapshot(&self, step: u32) -> bool {
        match self.strategy {
            RestartStrategy::SnapshotResetRetry {
                snapshot_every_n_steps,
                ..
            } => {
                snapshot_every_n_steps > 0 && step > 0 && step.is_multiple_of(snapshot_every_n_steps)
            }
            _ => false,
        }
    }

    /// Most recent snapshot. The executor reads this on
    /// [`RestartDecision::Retry { with_context: false }`] to seed the
    /// fresh sandbox.
    #[must_use]
    pub fn last_snapshot(&self) -> Option<&Snapshot> {
        self.last_snapshot.as_ref()
    }

    /// Backoff between attempts in milliseconds. Zero for strategies
    /// that don't define one.
    #[must_use]
    pub fn backoff_ms(&self) -> u32 {
        match self.strategy {
            RestartStrategy::RetryWithContext { backoff_ms, .. } => backoff_ms,
            _ => 0,
        }
    }
}

/// Build a snapshot from current loop state. Goal summary is the
/// original task description plus the last 200 chars of the trajectory
/// log (no model call required) so the fresh session has continuity.
#[must_use]
pub fn take_snapshot(
    last_screenshot: &[u8],
    last_action_idx: u32,
    description: &str,
    trajectory: &[String],
) -> Snapshot {
    let joined = trajectory.join(" / ");
    let tail = tail_chars(&joined, 200);
    let goal_summary = if tail.is_empty() {
        description.to_string()
    } else {
        format!("{description} -- recent: {tail}")
    };
    Snapshot {
        last_screenshot: last_screenshot.to_vec(),
        last_action_idx,
        goal_summary,
    }
}

/// Re-seed a fresh sandbox session with the snapshot's goal summary
/// and last screenshot. The executor passes the next step's
/// `system_prompt` and `screenshot` buffers by mutable reference; this
/// function rewrites them in place so the next
/// [`crate::anthropic::CuStepRequest`] carries the restart context.
/// No I/O, no model call.
///
/// Returns [`argo_types::ArgoError::Invariant`] only if the snapshot's
/// screenshot is empty — every legitimate snapshot carries the bytes
/// captured at snapshot time, so empty means a programming error.
pub fn restore_from(
    snap: &Snapshot,
    system_prompt: &mut String,
    screenshot: &mut Vec<u8>,
) -> ArgoResult<()> {
    if snap.last_screenshot.is_empty() {
        return Err(argo_types::ArgoError::Invariant(
            "restart::restore_from: snapshot.last_screenshot must not be empty".into(),
        ));
    }
    let base = system_prompt.clone();
    *system_prompt = format!(
        "{base}\n\nRESTART CONTEXT (resuming from step {}): {}",
        snap.last_action_idx, snap.goal_summary
    );
    *screenshot = snap.last_screenshot.clone();
    Ok(())
}

/// Emit the `cu.restart` telemetry event. The executor calls this
/// every time the coordinator returns [`RestartDecision::Retry`] (and
/// once more after the retry resolves with `success`). Uses the
/// existing `tracing` macros — no new exporter.
pub fn emit_restart_event(reason: &str, attempt: u32, success: Option<bool>) {
    tracing::info!(
        target: "argo_executor_computer_use::restart",
        event = "cu.restart",
        reason,
        attempt,
        success = success.map(|b| if b { "true" } else { "false" }).unwrap_or("pending"),
        "cu restart event",
    );
}

fn max_attempts(s: &RestartStrategy) -> u32 {
    match s {
        RestartStrategy::None => 0,
        RestartStrategy::RetryWithContext { max_attempts, .. }
        | RestartStrategy::SnapshotResetRetry { max_attempts, .. } => *max_attempts,
    }
}

/// Classify a `CuError` as retryable or fatal. Mirrors the brief's
/// taxonomy: transport / timeout / parse failures are retryable;
/// budget / config / permission errors are fatal.
fn is_retryable_error(err: &CuError) -> bool {
    match err {
        CuError::Http { .. } | CuError::HttpStatus { .. } => true,
        // Model emitted an unparseable response → re-prompt has a
        // strong chance of recovery.
        CuError::Protocol(_) => true,
        // Transient JSON parse failure on a network read.
        CuError::Json { .. } => true,
        CuError::BudgetExhausted { .. }
        | CuError::MissingEnv(_)
        | CuError::UnknownProfile { .. }
        | CuError::Toml { .. }
        | CuError::Io { .. } => false,
    }
}

/// Char-safe tail-trim. `s.split_at(byte_index)` panics on a non-char
/// boundary; we trim by counting from the end instead.
fn tail_chars(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        return s.to_string();
    }
    let skip = total - max_chars;
    s.chars().skip(skip).collect()
}

#[cfg(test)]
#[path = "restart_tests.rs"]
mod tests;
