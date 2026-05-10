//! `LoopExit` control-flow enum + the restart-coordinator handler
//! that drives recovery from transient sandbox / Anthropic failures.
//! Extracted from `executor.rs` (step 2 of `mvp-split-cu-executor`)
//! to keep both `executor.rs` and `executor_step_helpers.rs` under
//! their respective LOC ceilings.

use std::time::Duration;

use crate::budget::BudgetVerdict;
use crate::e2b::{E2bClient, Sandbox};
use crate::executor_types::RunResult;
use crate::profile::Profile;
use crate::restart::{self, emit_restart_event, RestartCoordinator, RestartDecision};
use crate::CuError;

#[derive(Debug)]
pub(crate) enum LoopExit {
    Success(RunResult),
    Handoff(String),
}

pub(crate) fn stringify_verdict(v: BudgetVerdict) -> String {
    match v {
        BudgetVerdict::Exhausted { reason } => reason,
        BudgetVerdict::Ok => "(unexpected ok verdict)".into(),
    }
}

/// Consult the restart coordinator on a transient failure. Returns
/// `Ok(None)` when the loop should retry (state has already been
/// reset / context restored), `Ok(Some(reason))` when the caller
/// should fall through to handoff with the given reason. Errors
/// only on a fresh sandbox open / restore failure.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_restart(
    e2b: &E2bClient,
    err: &CuError,
    stage: &'static str,
    profile: &Profile,
    coordinator: &mut RestartCoordinator,
    sandbox: &mut Sandbox,
    screenshot: &mut Vec<u8>,
    system_prompt: &mut String,
    base_system_prompt: &str,
) -> Result<Option<String>, CuError> {
    match coordinator.should_restart(err) {
        RestartDecision::Continue => Ok(Some(format!("{stage} failed: {err}"))),
        RestartDecision::Bail => {
            emit_restart_event(stage, coordinator.attempts(), Some(false));
            Ok(Some(format!(
                "{stage} failed (retries exhausted): {err}"
            )))
        }
        RestartDecision::Retry { with_context } => {
            emit_restart_event(stage, coordinator.attempts(), None);
            if !with_context {
                if let Some(snap) = coordinator.last_snapshot().cloned() {
                    // Re-seed prompt + screenshot from snapshot.
                    // Map the static-string Invariant into a
                    // CuError::Protocol so the loop can surface it
                    // through the existing taxonomy.
                    restart::restore_from(&snap, system_prompt, screenshot)
                        .map_err(|e| CuError::Protocol(e.to_string()))?;
                } else {
                    // No snapshot yet — reset the prompt to the
                    // base; the first fresh step starts clean.
                    *system_prompt = base_system_prompt.to_string();
                }
                // Best-effort close of the wedged sandbox; open a
                // fresh one. The pool slot keeps the original
                // entry — its TTL reaper will evict it. The fresh
                // sandbox lives only for this dispatch.
                let _ = e2b.close_sandbox(sandbox).await;
                *sandbox = e2b.open_sandbox(&profile.sandbox_template).await?;
                *screenshot = e2b.screenshot(sandbox).await?;
            }
            let backoff = u64::from(coordinator.backoff_ms());
            if backoff > 0 {
                tokio::time::sleep(Duration::from_millis(backoff)).await;
            }
            Ok(None)
        }
    }
}
