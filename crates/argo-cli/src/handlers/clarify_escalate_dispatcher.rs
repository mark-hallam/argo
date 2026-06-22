//! The `argo-cli` [`ClarifyDispatcher`] backing the in-turn `clarify` tool on
//! the headless `work serve` run path (`work-serve-wire-clarify-dispatcher`).
//!
//! # Why a headless dispatcher
//!
//! The `clarify` tool lets an agent PAUSE to ask the operator a question
//! instead of guessing. Its default backend errors with "clarify dispatcher
//! not wired", so on the headless worker-run path every `clarify` call FAILED —
//! the agent's correct instinct to ask had nowhere to go, wasting a step and
//! muddying the transcript.
//!
//! In a non-interactive run there is no live human to answer mid-turn, so this
//! dispatcher does NOT block and does NOT fabricate an answer. It RECORDS the
//! question into a shared one-slot [`ClarifyCell`] and returns a SUCCESS result
//! telling the agent the question was escalated and to stop. After the turn,
//! `work_steps::run_live_loop` reads the cell and, when set, ends the step with
//! a `[STOP: NEED_INFO: <question>]` marker — so the run terminates cleanly at
//! `stop_reason: need_info` with the question captured on the job's stop
//! surface, exactly like the runner's own transport-error NEED_INFO path.
//!
//! # Safety
//!
//! This adds NO execution and NO autonomous answer. It only records text and
//! returns a stop instruction; the deterministic NEED_INFO stop is applied by
//! the loop, not the model.

use argo_tool::{ClarifyAnswer, ClarifyDispatcher};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

/// Shared one-slot cell holding the FIRST clarify question raised this turn.
/// The dispatcher writes it; `run_live_loop` reads + clears it after the turn
/// to emit the deterministic `[STOP: NEED_INFO]` marker.
pub(crate) type ClarifyCell = Arc<Mutex<Option<String>>>;

/// Headless `clarify` backend: records the question, escalates as NEED_INFO.
#[derive(Debug)]
pub(crate) struct ClarifyEscalateDispatcher {
    cell: ClarifyCell,
}

impl ClarifyEscalateDispatcher {
    /// Wire the dispatcher to the run's shared question cell.
    pub(crate) fn new(cell: ClarifyCell) -> Self {
        Self { cell }
    }
}

#[async_trait]
impl ClarifyDispatcher for ClarifyEscalateDispatcher {
    fn surface_id(&self) -> &str {
        "work-serve-escalate"
    }

    async fn ask(
        &self,
        question: &str,
        options: Option<&[String]>,
    ) -> Result<ClarifyAnswer, String> {
        // Record the FIRST question only. A NEED_INFO stop ends the run, so a
        // later ask in the same turn would never be reached — but guard anyway
        // so the captured detail is the question the agent actually paused on.
        if let Ok(mut slot) = self.cell.lock() {
            if slot.is_none() {
                let recorded = match options {
                    Some(opts) if !opts.is_empty() => {
                        format!("{} (options: {})", question.trim(), opts.join(" | "))
                    }
                    _ => question.trim().to_string(),
                };
                *slot = Some(recorded);
            }
        }
        Ok(ClarifyAnswer {
            answer: "Question escalated to the operator as NEED_INFO. Stop now — \
                     the run will pause for an answer; do not guess or proceed."
                .to_string(),
            option_index: None,
        })
    }
}

#[cfg(test)]
#[path = "clarify_escalate_dispatcher_tests.rs"]
mod tests;
