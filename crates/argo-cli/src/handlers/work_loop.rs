//! Multi-step worker orchestration (#722).
//!
//! `argo work serve` runs each job through a small loop here:
//! one "step" is one full agent turn (which itself may dispatch
//! tools internally up to `MAX_TOOL_ROUNDS`), and we keep
//! issuing steps until the model emits a stop marker or we hit
//! `--max-steps`. The outer loop is what makes Worker v2 feel
//! "autonomous" — research → verify → draft → checklist all
//! happen inside one queued job rather than requiring the
//! human to re-prompt between phases.
//!
//! The orchestrator is parametric over the actual step driver
//! so unit tests can exercise it with mock closures (no LLM,
//! no SessionStore) — the integration with the real
//! `chat::run_turn` lives in `work.rs::process_job`.
//!
//! ## Stop markers
//!
//! The bound vertical's system prompt instructs the model to
//! end every turn with one of:
//!
//!   `[STOP: TASK_COMPLETE]`        — the task is fully done
//!   `[STOP: APPROVAL_REQUIRED]`    — drafts ready; needs human review
//!   `[STOP: NEED_INFO <one-line>]` — missing critical fact only the human supplies
//!   `[CONTINUE]`                   — more work to do; another step follows
//!
//! Anything else is treated as `[CONTINUE]` (lenient — newer
//! models drift on terse format demands and an unmarked
//! response shouldn't cause the worker to abort early). The
//! parser itself lives in `work_loop_stop_marker.rs`; we
//! re-export it as `pub(crate) use` so callers still see
//! `work_loop::parse_stop_marker`.

use std::future::Future;

pub(crate) use super::work_loop_stop_marker::parse_stop_marker;

/// Why the multi-step loop ended. Carried back to the caller
/// (process_job) so it can be persisted into job.json and the
/// completed_jobs memory body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StopReason {
    TaskComplete,
    ApprovalRequired,
    NeedInfo(String),
    MaxStepsReached,
    /// Pre-flight: neither web_search nor web_extract is available
    /// to the bound vertical, but multi-step mode (>1 step) was
    /// requested. We refuse to start rather than silently produce
    /// an unverified output.
    ResearchToolsUnavailable,
}

impl StopReason {
    /// Lower-snake-case wire form for JSON / memory rows.
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            StopReason::TaskComplete => "task_complete",
            StopReason::ApprovalRequired => "approval_required",
            StopReason::NeedInfo(_) => "need_info",
            StopReason::MaxStepsReached => "max_steps_reached",
            StopReason::ResearchToolsUnavailable => "research_tools_unavailable",
        }
    }

    /// Human-readable detail string for `NeedInfo`, empty for the
    /// other variants. Used in the artifact + memory payload.
    pub(crate) fn detail(&self) -> &str {
        match self {
            StopReason::NeedInfo(s) => s.as_str(),
            _ => "",
        }
    }
}

/// Build the synthetic per-step user message that nudges the
/// agent forward in step 2..N. Step 1's user message is the
/// real task prompt; this is for "keep going" rounds.
pub(crate) fn continue_nudge(step: u32, max_steps: u32) -> String {
    format!(
        "[Multi-step worker mode | step {step}/{max_steps}] Continue. \
         End with [STOP: TASK_COMPLETE], [STOP: APPROVAL_REQUIRED], \
         [STOP: NEED_INFO <reason>], or [CONTINUE]."
    )
}

/// Outcome of the multi-step loop, returned to the caller.
#[derive(Debug, Clone)]
pub(crate) struct LoopOutcome {
    pub steps_completed: u32,
    pub stop_reason: StopReason,
    pub final_text: String,
}

/// One step's result, supplied by the closure the caller passes.
#[derive(Debug, Clone)]
pub(crate) struct StepResult {
    pub text: String,
}

/// Drive the multi-step loop. Each iteration calls `step_fn`
/// with the step number (1-based) and the user message to
/// append; step 1 gets the real task prompt, step N>1 gets the
/// continue-nudge string.
///
/// Returns when the model emits a stop marker or we run out
/// of steps. The caller is responsible for persisting per-step
/// data — the loop only cares about driving the conversation
/// forward and detecting termination.
pub(crate) async fn run_loop<F, Fut>(
    initial_prompt: String,
    max_steps: u32,
    mut step_fn: F,
) -> LoopOutcome
where
    F: FnMut(u32, String) -> Fut,
    Fut: Future<Output = StepResult>,
{
    if max_steps == 0 {
        return LoopOutcome {
            steps_completed: 0,
            stop_reason: StopReason::MaxStepsReached,
            final_text: String::new(),
        };
    }
    let mut prompt = initial_prompt;
    let mut last_text = String::new();
    let mut steps_completed = 0u32;
    for step in 1..=max_steps {
        let result = step_fn(step, prompt).await;
        steps_completed = step;
        last_text = result.text;
        if let Some(reason) = parse_stop_marker(&last_text) {
            return LoopOutcome {
                steps_completed,
                stop_reason: reason,
                final_text: last_text,
            };
        }
        prompt = continue_nudge(step + 1, max_steps);
    }
    LoopOutcome {
        steps_completed,
        stop_reason: StopReason::MaxStepsReached,
        final_text: last_text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Loop with closures that always emit `[CONTINUE]` (i.e. no
    /// stop marker) — the loop must cap at `max_steps`. Locks the
    /// "max steps prevents infinite loops" guarantee.
    #[tokio::test]
    async fn loop_caps_at_max_steps_when_model_never_stops() {
        let calls = RefCell::new(0u32);
        let outcome = run_loop("seed prompt".into(), 3, |step, _prompt| {
            *calls.borrow_mut() += 1;
            async move {
                StepResult {
                    text: format!("step {step} continues [CONTINUE]"),
                }
            }
        })
        .await;
        assert_eq!(*calls.borrow(), 3);
        assert_eq!(outcome.steps_completed, 3);
        assert_eq!(outcome.stop_reason, StopReason::MaxStepsReached);
    }

    /// Closure emits TASK_COMPLETE on step 2 → loop must stop early.
    #[tokio::test]
    async fn loop_stops_early_on_task_complete() {
        let outcome = run_loop("seed".into(), 5, |step, _prompt| async move {
            if step == 2 {
                StepResult {
                    text: "wrapping up [STOP: TASK_COMPLETE]".into(),
                }
            } else {
                StepResult {
                    text: "[CONTINUE]".into(),
                }
            }
        })
        .await;
        assert_eq!(outcome.steps_completed, 2);
        assert_eq!(outcome.stop_reason, StopReason::TaskComplete);
    }

    /// Step 1 returns APPROVAL_REQUIRED — loop returns immediately
    /// with steps_completed = 1, exercising the no-iterations-after
    /// path.
    #[tokio::test]
    async fn loop_stops_on_first_step_when_approval_required() {
        let outcome = run_loop("seed".into(), 5, |_, _| async move {
            StepResult {
                text: "drafts ready\n[STOP: APPROVAL_REQUIRED]".into(),
            }
        })
        .await;
        assert_eq!(outcome.steps_completed, 1);
        assert_eq!(outcome.stop_reason, StopReason::ApprovalRequired);
    }

    #[tokio::test]
    async fn zero_max_steps_returns_immediately() {
        let outcome = run_loop("seed".into(), 0, |_, _| async move {
            StepResult {
                text: "should not run".into(),
            }
        })
        .await;
        assert_eq!(outcome.steps_completed, 0);
        assert_eq!(outcome.stop_reason, StopReason::MaxStepsReached);
    }

    #[test]
    fn continue_nudge_includes_step_numbering() {
        let out = continue_nudge(3, 5);
        assert!(out.contains("step 3/5"), "got: {out}");
    }

    /// #724 — when chat::run_turn returns Ok with empty text
    /// (e.g. the model only emitted tool_use blocks before
    /// max_rounds capped the inner loop), parse_stop_marker
    /// returns None on empty text, so the outer loop must
    /// continue to the next step instead of treating empty as
    /// a stop condition.
    #[tokio::test]
    async fn loop_continues_when_step_returns_empty_text() {
        let calls = RefCell::new(0u32);
        let outcome = run_loop("seed".into(), 4, |_step, _prompt| {
            *calls.borrow_mut() += 1;
            async move {
                StepResult {
                    text: String::new(),
                }
            }
        })
        .await;
        assert_eq!(*calls.borrow(), 4, "outer loop must run all 4 steps");
        assert_eq!(outcome.steps_completed, 4);
        assert_eq!(
            outcome.stop_reason,
            StopReason::MaxStepsReached,
            "empty text has no stop marker so loop caps at max_steps"
        );
    }

    /// Same shape as the live integration: each step returns
    /// the breadcrumb run_live_loop would append on a
    /// tool-budget-exhausted Ok TurnResult — text without any
    /// stop marker. The outer loop must run all max_steps and
    /// surface MaxStepsReached (NOT NeedInfo).
    #[tokio::test]
    async fn loop_treats_tool_budget_breadcrumb_as_continuation() {
        let outcome = run_loop("seed".into(), 3, |step, _prompt| async move {
            StepResult {
                text: format!(
                    "step {step} did some work\n\n_[step boundary: \
                     tool-round budget exhausted; continuing in next step]_\n"
                ),
            }
        })
        .await;
        assert_eq!(outcome.steps_completed, 3);
        assert_eq!(outcome.stop_reason, StopReason::MaxStepsReached);
        assert!(
            outcome.final_text.contains("step boundary"),
            "final_text must preserve the breadcrumb so the artifact splitter surfaces it"
        );
    }
}
