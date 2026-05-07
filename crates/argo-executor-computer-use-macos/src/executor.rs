//! Public `MacosCuExecutor` orchestrator. Wires preflight + app
//! activation + screenshot capture + Anthropic CU loop +
//! event injection into a single `run()` entry point.
//!
//! The executor is structurally a sibling of
//! [`argo_executor_computer_use::ComputerUseExecutor`] — same
//! `RunRequest → Outcome` shape, same Anthropic-loop step
//! semantics. The only difference is the sandbox driver.
//!
//! For the V1.5 PoC the executor is buildable + testable but
//! NOT yet wired into `argo-cli`'s
//! `worker_actions_executor::resolve_executor` arm — that
//! ExecutorKind variant + dispatch arm ships as the follow-up
//! brief. The crate stands alone here as a library.

use crate::a11y::{activate_app, assert_front_app_is};
use crate::config::MacosCuConfig;
use crate::error::MacosCuError;
use crate::events::{action_kind, post, MacosActionResult};
use crate::preflight::{probe, PermissionReport};
use crate::screenshot::{capture_front_window, read_png, screen_size};
use crate::MacosCuResult;
use argo_executor_computer_use::e2b::CuAction;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

/// Top-level executor. Cloneable (config is `Clone`), so the same
/// instance can drive concurrent dispatches.
#[derive(Debug, Clone)]
pub struct MacosCuExecutor {
    config: MacosCuConfig,
}

/// One dispatch input. Mirrors
/// [`argo_executor_computer_use::RunRequest`] in shape so the
/// dispatcher can hand off between executors with minimal glue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacosCuRunRequest {
    /// Action id from the worker_action ledger row.
    pub action_id: String,
    /// Full system prompt — task description, target app, any
    /// app-specific context.
    pub system_prompt: String,
    /// Where to write the per-step screenshots + audit jsonl.
    pub audit_dir: PathBuf,
    /// Anthropic API key. The executor stamps a fresh client per
    /// run so concurrent dispatches don't share state.
    pub anthropic_api_key: String,
}

/// Outcome polarity matches the E2B executor:
/// [`argo_executor_computer_use::Outcome`]. Both feed the same
/// dispatcher arm; the dispatcher auto-completes on `Completed`
/// and routes to human handoff on `HumanHandoff`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MacosCuOutcome {
    /// Model emitted `stop_success`; the loop captured a result.
    Completed {
        /// Plain-English summary of what the loop accomplished.
        summary: String,
        /// Per-step audit-friendly trace.
        steps: Vec<StepRecord>,
        /// Wallclock seconds the loop took.
        wallclock_seconds: u32,
    },
    /// Budget exhausted, model emitted `stop_failure`, or a
    /// recoverable error surfaced.
    HumanHandoff {
        /// One-line operator-facing reason.
        reason: String,
        /// Steps taken so far.
        steps: Vec<StepRecord>,
    },
}

/// One CU loop step's audit row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRecord {
    /// 0-indexed step number in this dispatch.
    pub step: u32,
    /// Action kind label (`click` / `type` / `key` / ...).
    /// Owned `String` (not `&'static str`) so the row round-trips
    /// through the audit jsonl.
    pub action_kind: String,
    /// Operator-facing summary of the action.
    pub action_summary: String,
    /// Result returned by the OS.
    pub result: MacosActionResult,
    /// Whether this step ended the loop (stop_success or
    /// stop_failure). `None` if the model emitted a tool_use.
    pub stop_signal: Option<String>,
}

impl MacosCuExecutor {
    /// Build the executor. Validates the config; returns an error
    /// if `target_app` isn't in `allowed_apps` or
    /// `allowed_actions` is empty.
    pub fn new(config: MacosCuConfig) -> MacosCuResult<Self> {
        config
            .validate()
            .map_err(MacosCuError::Protocol)?;
        Ok(Self { config })
    }

    /// Probe the four required macOS permissions. Returns a
    /// report — call `report.all_granted()` and bail with
    /// `report.actionable_summary()` if any are missing.
    pub async fn preflight(&self) -> PermissionReport {
        probe()
    }

    /// Borrow the active config for the dispatcher.
    #[must_use]
    pub fn config(&self) -> &MacosCuConfig {
        &self.config
    }

    /// One CU loop tick — execute the action, capture follow-up
    /// state. Pure dispatch step; the full `run()` loop builds
    /// on top of this.
    pub async fn execute_step(
        &self,
        action: &CuAction,
    ) -> MacosCuResult<MacosActionResult> {
        // Re-assert front-app focus before each step so a model
        // can't drift into an off-target app via stale state.
        assert_front_app_is(&self.config.target_app).await?;
        post(action, &self.config).await
    }

    /// Bring the target app to front + return its post-activate
    /// screenshot bytes. Used by the loop to seed the first
    /// Anthropic CU request.
    pub async fn warm_up(
        &self,
        screenshot_dest: &std::path::Path,
    ) -> MacosCuResult<Vec<u8>> {
        activate_app(&self.config.target_app, &self.config).await?;
        capture_front_window(screenshot_dest)?;
        read_png(screenshot_dest)
    }

    /// Active screen geometry. Used by the loop to size the
    /// Anthropic tool spec.
    pub fn screen_size(&self) -> MacosCuResult<(u32, u32)> {
        screen_size()
    }

    /// One-tick wrapper that records the step into a `StepRecord`
    /// for audit.
    pub async fn execute_step_recorded(
        &self,
        step_idx: u32,
        action: &CuAction,
    ) -> MacosCuResult<StepRecord> {
        let action_kind = action_kind(action).to_string();
        let action_summary = action.summary();
        let result = self.execute_step(action).await?;
        Ok(StepRecord {
            step: step_idx,
            action_kind,
            action_summary,
            result,
            stop_signal: None,
        })
    }

    /// Force-emit a `HumanHandoff` outcome for a budget breach.
    /// Helper used by the loop to keep the budget-exhausted path
    /// uniform with other handoff causes.
    #[must_use]
    pub fn handoff(reason: impl Into<String>, steps: Vec<StepRecord>) -> MacosCuOutcome {
        MacosCuOutcome::HumanHandoff {
            reason: reason.into(),
            steps,
        }
    }

    /// Build a `Completed` outcome from a final summary + step
    /// trace.
    #[must_use]
    pub fn completed(
        summary: impl Into<String>,
        steps: Vec<StepRecord>,
        wallclock: std::time::Duration,
    ) -> MacosCuOutcome {
        MacosCuOutcome::Completed {
            summary: summary.into(),
            steps,
            wallclock_seconds: wallclock.as_secs() as u32,
        }
    }

    /// True if the loop has run past the wallclock or step
    /// ceilings configured.
    #[must_use]
    pub fn budget_exhausted(
        &self,
        steps_so_far: u32,
        started_at: Instant,
    ) -> bool {
        if steps_so_far >= self.config.max_steps {
            return true;
        }
        let elapsed = started_at.elapsed();
        elapsed.as_secs() >= u64::from(self.config.max_wallclock_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_validates_config() {
        let mut cfg = MacosCuConfig::for_notes_app();
        cfg.target_app = "com.apple.Terminal".to_string();
        let err = MacosCuExecutor::new(cfg).unwrap_err();
        assert!(matches!(err, MacosCuError::Protocol(_)));
    }

    #[test]
    fn new_accepts_default_config() {
        let cfg = MacosCuConfig::for_notes_app();
        let _ = MacosCuExecutor::new(cfg).unwrap();
    }

    #[test]
    fn handoff_helper_packages_reason() {
        let out = MacosCuExecutor::handoff("budget", vec![]);
        match out {
            MacosCuOutcome::HumanHandoff { reason, steps } => {
                assert_eq!(reason, "budget");
                assert!(steps.is_empty());
            }
            _ => panic!("expected HumanHandoff"),
        }
    }

    #[test]
    fn completed_helper_records_wallclock_in_seconds() {
        let out = MacosCuExecutor::completed(
            "ok",
            vec![],
            std::time::Duration::from_secs(42),
        );
        match out {
            MacosCuOutcome::Completed {
                wallclock_seconds, ..
            } => {
                assert_eq!(wallclock_seconds, 42);
            }
            _ => panic!("expected Completed"),
        }
    }

    #[test]
    fn budget_exhausted_when_steps_exceed_max() {
        let cfg = MacosCuConfig::for_notes_app();
        let exec = MacosCuExecutor::new(cfg).unwrap();
        let started = Instant::now();
        assert!(exec.budget_exhausted(exec.config.max_steps, started));
        assert!(exec.budget_exhausted(exec.config.max_steps + 1, started));
        assert!(!exec.budget_exhausted(exec.config.max_steps - 1, started));
    }

    #[test]
    fn budget_not_exhausted_below_ceiling() {
        let cfg = MacosCuConfig::for_notes_app();
        let exec = MacosCuExecutor::new(cfg).unwrap();
        assert!(!exec.budget_exhausted(0, Instant::now()));
    }
}
