//! Public `MacosCuExecutor` orchestrator: preflight, app
//! activation, screenshot capture, Anthropic CU loop, and event
//! injection through native macOS primitives.

use crate::a11y::{activate_app, assert_front_app_is};
use crate::config::MacosCuConfig;
use crate::error::MacosCuError;
use crate::events::{action_kind, post, MacosActionResult};
use crate::executor_loop::run_loop_with_client;
use crate::preflight::{probe, PermissionReport};
use crate::screenshot::{capture_front_window, read_png, screen_size};
use crate::MacosCuResult;
use argo_executor_computer_use::anthropic::AnthropicClient;
use argo_executor_computer_use::audit::AuditLog;
use argo_executor_computer_use::budget::BudgetTracker;
use argo_executor_computer_use::config::DEFAULT_TOOL_VARIANT;
use argo_executor_computer_use::e2b::{ActionResult, CuAction};
use argo_executor_computer_use::profile::Profile;
use argo_executor_computer_use::restart::RestartCoordinator;
use argo_executor_computer_use::{CuError, CuStepRunner};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

/// Top-level executor. Cloneable (config is `Clone`), so the same
/// instance can drive concurrent dispatches.
#[derive(Debug, Clone)]
pub struct MacosCuExecutor {
    config: MacosCuConfig,
}

/// One native-macOS CU dispatch input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacosCuRunRequest {
    pub action_id: String,
    pub system_prompt: String,
    pub audit_dir: PathBuf,
    pub anthropic_api_key: String,
}

/// Outcome polarity matches the E2B executor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MacosCuOutcome {
    Completed {
        summary: String,
        steps: Vec<MacosStepRecord>,
        wallclock_seconds: u32,
    },
    HumanHandoff {
        reason: String,
        steps: Vec<MacosStepRecord>,
    },
}

/// One CU loop step's audit row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacosStepRecord {
    pub step: u32,
    pub action_kind: String,
    pub action_summary: String,
    pub result: MacosActionResult,
    pub stop_signal: Option<String>,
}

/// Per-dispatch native macOS session used by `CuStepRunner`.
#[derive(Debug, Clone)]
pub struct MacosCuSession {
    pub(crate) screenshot_path: PathBuf,
}

impl MacosCuExecutor {
    /// Build the executor. Validates the config; returns an error
    /// if `target_app` isn't in `allowed_apps` or
    /// `allowed_actions` is empty.
    pub fn new(config: MacosCuConfig) -> MacosCuResult<Self> {
        config.validate().map_err(MacosCuError::Protocol)?;
        Ok(Self { config })
    }

    /// Probe the four required macOS permissions. Returns a
    /// report — call `report.all_granted()` and bail with
    /// `report.actionable_summary()` if any are missing.
    pub async fn preflight(&self) -> PermissionReport {
        probe()
    }

    /// Run the autonomous Anthropic computer-use loop against
    /// native macOS step primitives.
    pub async fn run(&self, req: MacosCuRunRequest) -> MacosCuResult<MacosCuOutcome> {
        let report = self.preflight().await;
        if !report.all_granted() {
            return Ok(Self::handoff(report.actionable_summary(), Vec::new()));
        }
        let audit = AuditLog::create(req.audit_dir.clone()).map_err(macos_from_cu)?;
        let (width, height) = self.screen_size()?;
        let profile = profile_from_config(&self.config, width, height);
        let anthropic =
            anthropic_client(&self.config, &req.anthropic_api_key).map_err(macos_from_cu)?;
        let session = MacosCuSession {
            screenshot_path: req.audit_dir.join("front-window.png"),
        };
        let mut tracker = BudgetTracker::new(&profile);
        run_loop_with_client(
            self,
            &anthropic,
            &req,
            &profile,
            &session,
            &audit,
            &mut tracker,
            Instant::now(),
        )
        .await
        .map_err(macos_from_cu)
    }

    /// Borrow the active config for the dispatcher.
    #[must_use]
    pub fn config(&self) -> &MacosCuConfig {
        &self.config
    }

    /// One CU loop tick — execute the action, capture follow-up
    /// state. Pure dispatch step; the full `run()` loop builds
    /// on top of this.
    pub async fn execute_step(&self, action: &CuAction) -> MacosCuResult<MacosActionResult> {
        // Re-assert front-app focus before each step so a model
        // can't drift into an off-target app via stale state.
        assert_front_app_is(&self.config.target_app).await?;
        post(action, &self.config).await
    }

    /// Bring the target app to front + return its post-activate
    /// screenshot bytes. Used by the loop to seed the first
    /// Anthropic CU request.
    pub async fn warm_up(&self, screenshot_dest: &std::path::Path) -> MacosCuResult<Vec<u8>> {
        activate_app(&self.config.target_app, &self.config).await?;
        capture_front_window(screenshot_dest)?;
        read_png(screenshot_dest)
    }

    /// Active screen geometry. Used by the loop to size the
    /// Anthropic tool spec.
    pub fn screen_size(&self) -> MacosCuResult<(u32, u32)> {
        screen_size()
    }

    /// One-tick wrapper that records the step into a `MacosStepRecord`
    /// for audit.
    pub async fn execute_step_recorded(
        &self,
        step_idx: u32,
        action: &CuAction,
    ) -> MacosCuResult<MacosStepRecord> {
        let action_kind = action_kind(action).to_string();
        let action_summary = action.summary();
        let result = self.execute_step(action).await?;
        Ok(MacosStepRecord {
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
    pub fn handoff(reason: impl Into<String>, steps: Vec<MacosStepRecord>) -> MacosCuOutcome {
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
        steps: Vec<MacosStepRecord>,
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
    pub fn budget_exhausted(&self, steps_so_far: u32, started_at: Instant) -> bool {
        if steps_so_far >= self.config.max_steps {
            return true;
        }
        let elapsed = started_at.elapsed();
        elapsed.as_secs() >= u64::from(self.config.max_wallclock_seconds)
    }
}

#[async_trait]
impl CuStepRunner for MacosCuExecutor {
    type Session = MacosCuSession;

    async fn preflight(&self) -> Result<(), CuError> {
        let report = MacosCuExecutor::preflight(self).await;
        if report.all_granted() {
            Ok(())
        } else {
            Err(CuError::Protocol(report.actionable_summary()))
        }
    }

    async fn warm_up(&self, session: &Self::Session) -> Result<Vec<u8>, CuError> {
        MacosCuExecutor::warm_up(self, &session.screenshot_path)
            .await
            .map_err(|e| macos_to_cu(e, "macos.warm_up"))
    }

    async fn capture_screenshot(&self, session: &Self::Session) -> Result<Vec<u8>, CuError> {
        capture_front_window(&session.screenshot_path)
            .map_err(|e| macos_to_cu(e, "macos.screenshot"))?;
        read_png(&session.screenshot_path).map_err(|e| macos_to_cu(e, "macos.screenshot.read"))
    }

    async fn execute_step_recorded(
        &self,
        _session: &Self::Session,
        action: &CuAction,
    ) -> Result<ActionResult, CuError> {
        let result = MacosCuExecutor::execute_step(self, action)
            .await
            .map_err(|e| macos_to_cu(e, "macos.action"))?;
        Ok(ActionResult {
            ok: result.ok,
            text: result.text,
            screenshot_b64: None,
        })
    }

    async fn recover_after_error(
        &self,
        err: &CuError,
        stage: &'static str,
        _profile: &Profile,
        _coordinator: &mut RestartCoordinator,
        _session: &mut Self::Session,
        _screenshot: &mut Vec<u8>,
        _system_prompt: &mut String,
        _base_system_prompt: &str,
    ) -> Result<Option<String>, CuError> {
        Ok(Some(format!("{stage} failed: {err}")))
    }
}

fn profile_from_config(config: &MacosCuConfig, width: u32, height: u32) -> Profile {
    Profile {
        sandbox_template: "native-macos".into(),
        description: format!("Native macOS target app {}", config.target_app),
        url_whitelist: Vec::new(),
        max_tokens_per_action: 150_000,
        max_wallclock_seconds: config.max_wallclock_seconds,
        max_steps: config.max_steps,
        screen_width: width,
        screen_height: height,
        model: config.anthropic_model.clone(),
        restart: Default::default(),
    }
}

fn anthropic_client(config: &MacosCuConfig, api_key: &str) -> Result<AnthropicClient, CuError> {
    AnthropicClient::new(
        config
            .anthropic_base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com".into()),
        api_key.to_string(),
        config.anthropic_model.clone(),
        DEFAULT_TOOL_VARIANT.to_string(),
        config.anthropic_timeout_seconds,
    )
}

fn macos_from_cu(err: CuError) -> MacosCuError {
    let msg = err.to_string();
    match err {
        CuError::Io { source, .. } => MacosCuError::Io(source),
        CuError::Http { .. } | CuError::HttpStatus { .. } => MacosCuError::Anthropic(msg),
        _ => MacosCuError::Protocol(msg),
    }
}

fn macos_to_cu(err: MacosCuError, stage: &'static str) -> CuError {
    let message = err.to_string();
    if err.is_recoverable_to_handoff() {
        CuError::BudgetExhausted {
            step: 0,
            reason: format!("{stage}: {message}"),
        }
    } else {
        CuError::Protocol(format!("{stage}: {message}"))
    }
}
