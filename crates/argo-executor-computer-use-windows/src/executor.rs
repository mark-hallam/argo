//! Public `WindowsCuExecutor` orchestrator. Wires preflight +
//! window activation + multi-monitor screenshot capture + the
//! Anthropic CU loop + SendInput-based event injection into one
//! `run()` entry point.
//!
//! Structurally a sibling of
//! [`argo_executor_computer_use_macos::MacosCuExecutor`] — same
//! `RunRequest → Outcome` shape, same Anthropic-loop step
//! semantics. The only difference is the input / capture driver.
//!
//! For V1.5 the executor is buildable + testable but not yet
//! wired into `argo-cli`'s `worker_actions_executor::resolve_executor`
//! dispatch arm — that lands as a follow-up brief.

use crate::config::WindowsCuConfig;
use crate::error::WindowsCuError;
use crate::events::{action_kind, post, WindowsActionResult};
use crate::preflight::{probe, WindowsPreflightReport};
use crate::screenshot::{capture_all_monitors, MonitorScreenshot};
use crate::WindowsCuResult;
use argo_executor_computer_use::e2b::CuAction;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

/// Top-level executor. Cloneable (config is `Clone`), so the same
/// instance can drive concurrent dispatches.
#[derive(Debug, Clone)]
pub struct WindowsCuExecutor {
    config: WindowsCuConfig,
}

/// One dispatch input. Mirrors
/// [`argo_executor_computer_use::RunRequest`] in shape so the
/// dispatcher can hand off between executors with minimal glue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsCuRunRequest {
    /// Action id from the worker_action ledger row.
    pub action_id: String,
    /// Full system prompt — task description, target app, any
    /// app-specific context.
    pub system_prompt: String,
    /// Where to write per-step screenshots + the audit jsonl.
    pub audit_dir: PathBuf,
    /// Anthropic API key. The executor stamps a fresh client per
    /// run so concurrent dispatches don't share state.
    pub anthropic_api_key: String,
}

/// Outcome polarity matches the cross-platform executor:
/// [`argo_executor_computer_use::Outcome`]. Both feed the same
/// dispatcher arm; the dispatcher auto-completes on `Completed`
/// and routes to human handoff on `HumanHandoff`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowsCuOutcome {
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
    pub action_kind: String,
    /// Operator-facing summary of the action.
    pub action_summary: String,
    /// Result returned by the OS.
    pub result: WindowsActionResult,
    /// Whether this step ended the loop (`stop_success` /
    /// `stop_failure`). `None` if the model emitted a `tool_use`.
    pub stop_signal: Option<String>,
}

impl WindowsCuExecutor {
    /// Build the executor. Validates the config; returns an error
    /// if `target_app` isn't in `allowed_apps` or
    /// `allowed_actions` is empty.
    pub fn new(config: WindowsCuConfig) -> WindowsCuResult<Self> {
        config.validate().map_err(WindowsCuError::Protocol)?;
        Ok(Self { config })
    }

    /// Probe the six Windows-specific preflight axes. Returns a
    /// report — call `report.all_passed()` and bail with
    /// `report.actionable_summary()` if any are failing.
    pub async fn preflight(&self) -> WindowsPreflightReport {
        probe()
    }

    /// Borrow the active config for the dispatcher.
    #[must_use]
    pub fn config(&self) -> &WindowsCuConfig {
        &self.config
    }

    /// Declare per-monitor v2 DPI awareness for the current
    /// process and initialise COM for UIAutomation use on this
    /// thread. Idempotent — safe to call once per executor
    /// instance.
    pub fn warm_up(&self) -> WindowsCuResult<()> {
        if !cfg!(target_os = "windows") {
            return Err(WindowsCuError::NotWindows);
        }
        warm_up_impl(self.config.dpi_aware)
    }

    /// One CU loop tick — execute the action, capture the
    /// follow-up result. The full `run()` loop builds on top of
    /// this.
    pub async fn execute_step(
        &self,
        action: &CuAction,
    ) -> WindowsCuResult<WindowsActionResult> {
        post(action, &self.config).await
    }

    /// Capture every attached monitor and write per-monitor PNGs
    /// into `audit_dir`. Used by the loop to seed the first
    /// Anthropic CU request.
    pub fn warm_up_screenshots(
        &self,
        audit_dir: &std::path::Path,
    ) -> WindowsCuResult<Vec<MonitorScreenshot>> {
        capture_all_monitors(audit_dir)
    }

    /// Record one step into a `StepRecord` for audit.
    pub async fn execute_step_recorded(
        &self,
        step_idx: u32,
        action: &CuAction,
    ) -> WindowsCuResult<StepRecord> {
        let kind = action_kind(action).to_string();
        let summary = action.summary();
        let result = self.execute_step(action).await?;
        Ok(StepRecord {
            step: step_idx,
            action_kind: kind,
            action_summary: summary,
            result,
            stop_signal: None,
        })
    }

    /// Force-emit a `HumanHandoff` outcome for a budget breach.
    #[must_use]
    pub fn handoff(reason: impl Into<String>, steps: Vec<StepRecord>) -> WindowsCuOutcome {
        WindowsCuOutcome::HumanHandoff {
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
    ) -> WindowsCuOutcome {
        WindowsCuOutcome::Completed {
            summary: summary.into(),
            steps,
            wallclock_seconds: wallclock.as_secs() as u32,
        }
    }

    /// True if the loop has run past the wallclock or step
    /// ceilings configured.
    #[must_use]
    pub fn budget_exhausted(&self, steps_so_far: u32, started_at: Instant) -> bool {
        if steps_so_far >= self.config.max_steps {
            return true;
        }
        let elapsed = started_at.elapsed();
        elapsed.as_secs() >= u64::from(self.config.max_wallclock_seconds)
    }
}

#[cfg(target_os = "windows")]
fn warm_up_impl(dpi_aware: bool) -> WindowsCuResult<()> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    unsafe {
        // CoInitializeEx is idempotent per-thread; subsequent
        // calls return S_FALSE which `windows::core::Result`
        // treats as `Ok`.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if dpi_aware {
            // SetProcessDpiAwarenessContext fails if the
            // process already declared awareness — that's
            // benign for us; treat any error as "already set".
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn warm_up_impl(_dpi_aware: bool) -> WindowsCuResult<()> {
    Err(WindowsCuError::NotWindows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_validates_config() {
        let mut cfg = WindowsCuConfig::for_notepad_app();
        cfg.target_app = "cmd.exe".to_string();
        let err = WindowsCuExecutor::new(cfg).unwrap_err();
        assert!(matches!(err, WindowsCuError::Protocol(_)));
    }

    #[test]
    fn new_accepts_default_config() {
        let cfg = WindowsCuConfig::for_notepad_app();
        let _ = WindowsCuExecutor::new(cfg).unwrap();
    }

    #[test]
    fn handoff_helper_packages_reason() {
        let out = WindowsCuExecutor::handoff("budget", vec![]);
        match out {
            WindowsCuOutcome::HumanHandoff { reason, steps } => {
                assert_eq!(reason, "budget");
                assert!(steps.is_empty());
            }
            _ => panic!("expected HumanHandoff"),
        }
    }

    #[test]
    fn completed_helper_records_wallclock_in_seconds() {
        let out = WindowsCuExecutor::completed(
            "ok",
            vec![],
            std::time::Duration::from_secs(42),
        );
        match out {
            WindowsCuOutcome::Completed {
                wallclock_seconds, ..
            } => {
                assert_eq!(wallclock_seconds, 42);
            }
            _ => panic!("expected Completed"),
        }
    }

    #[test]
    fn budget_exhausted_when_steps_exceed_max() {
        let cfg = WindowsCuConfig::for_notepad_app();
        let exec = WindowsCuExecutor::new(cfg).unwrap();
        let started = Instant::now();
        assert!(exec.budget_exhausted(exec.config.max_steps, started));
        assert!(exec.budget_exhausted(exec.config.max_steps + 1, started));
        assert!(!exec.budget_exhausted(exec.config.max_steps - 1, started));
    }

    #[test]
    fn budget_not_exhausted_below_ceiling() {
        let cfg = WindowsCuConfig::for_notepad_app();
        let exec = WindowsCuExecutor::new(cfg).unwrap();
        assert!(!exec.budget_exhausted(0, Instant::now()));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn warm_up_returns_not_windows_off_platform() {
        let cfg = WindowsCuConfig::for_notepad_app();
        let exec = WindowsCuExecutor::new(cfg).unwrap();
        let err = exec.warm_up().unwrap_err();
        assert!(matches!(err, WindowsCuError::NotWindows));
    }
}
