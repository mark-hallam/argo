//! Anthropic CU loop driver + `CuStepRunner` impl for
//! `WindowsCuExecutor` (win-cu-executor-windows-run-loop, DEC-054).
//!
//! ## Why this mirrors macOS instead of calling the shared `run_loop`
//!
//! The brief asked to "reuse the shared `run_loop`", but that is not
//! reachable here: `argo_executor_computer_use::executor_step_helpers::run_loop`
//! is `pub(crate)` (not exported) AND builds a *concrete* `AnthropicClient`
//! internally (`anthropic_for(...)`), so it cannot be (a) called from this
//! crate or (b) driven by a scripted client for a HERMETIC test. That is the
//! exact reason macOS ships its own `run_loop_with_client` generic over
//! `A: AnthropicCall` (`argo-executor-computer-use-macos/src/executor_loop.rs`).
//! This module mirrors that proven macOS pattern so the loop is testable with
//! NO API key and NO live desktop. True de-duplication (making the shared loop
//! generic + `pub`) is a canonical-crate refactor left to a follow-up.
//!
//! The loop itself is NOT re-invented: turn-taking, the risk-tiered ACT floor
//! (DEC-051), budgeting, and audit rows match the macOS/shared semantics.

use crate::error::WindowsCuError;
use crate::events::{action_kind, post, WindowsActionResult};
use crate::executor::{
    WindowsCuExecutor, WindowsCuOutcome, WindowsCuRunRequest, WindowsCuSession, WindowsStepRecord,
};
use argo_executor_computer_use::anthropic::{AnthropicCall, CuStepResponse, StopReason};
use argo_executor_computer_use::audit::{AuditLog, CuAuditEvent};
use crate::cu_risk;
use argo_executor_computer_use::budget::{BudgetTracker, BudgetVerdict};
use argo_executor_computer_use::e2b::{ActionResult, CuAction};
use argo_executor_computer_use::profile::Profile;
use argo_executor_computer_use::restart::RestartCoordinator;
use argo_executor_computer_use::{anthropic::CuStepRequest, CuError, CuStepRunner};
use async_trait::async_trait;
use std::time::Instant;

/// Map a `WindowsCuError` into the platform-neutral `CuError` the shared
/// loop speaks, tagging the failing stage (mirrors macOS `macos_to_cu`).
pub(crate) fn windows_to_cu(err: WindowsCuError, stage: &'static str) -> CuError {
    match err {
        WindowsCuError::Io(source) => CuError::Io {
            what: stage,
            source,
        },
        WindowsCuError::Anthropic(msg) => CuError::Protocol(format!("{stage}: {msg}")),
        other => CuError::Protocol(format!("{stage}: {other}")),
    }
}

/// Per-step CU loop driver, generic over the Anthropic client so a scripted
/// `AnthropicCall` can drive it hermetically in tests. Mirrors macOS
/// `run_loop_with_client`.
#[allow(clippy::too_many_arguments)]
pub async fn run_loop_with_client<R, A>(
    runner: &R,
    anthropic: &A,
    req: &WindowsCuRunRequest,
    profile: &Profile,
    session: &R::Session,
    audit: &AuditLog,
    tracker: &mut BudgetTracker,
    started_at: Instant,
) -> Result<WindowsCuOutcome, CuError>
where
    R: CuStepRunner + ?Sized,
    A: AnthropicCall + ?Sized,
{
    runner.preflight().await?;
    let mut state = LoopState::new(
        req.system_prompt.clone(),
        runner.warm_up(session).await?,
        req.act_approved,
    );
    audit.save_screenshot(0, &state.screenshot)?;
    loop {
        let responses = anthropic.step(step_request(profile, &state)).await?;
        for response in responses {
            if let Some(outcome) =
                handle_response(runner, session, audit, tracker, &mut state, response, started_at)
                    .await?
            {
                return Ok(outcome);
            }
        }
    }
}

struct LoopState {
    prior_actions: Vec<String>,
    steps: Vec<WindowsStepRecord>,
    screenshot: Vec<u8>,
    system_prompt: String,
    /// Risk-tiered floor (DEC-051): while `false`, the loop holds at the first
    /// ACT op and returns `HumanHandoff`.
    act_approved: bool,
}

impl LoopState {
    fn new(system_prompt: String, screenshot: Vec<u8>, act_approved: bool) -> Self {
        Self {
            prior_actions: Vec::new(),
            steps: Vec::new(),
            screenshot,
            system_prompt,
            act_approved,
        }
    }
}

fn step_request<'a>(profile: &'a Profile, state: &'a LoopState) -> CuStepRequest<'a> {
    CuStepRequest {
        system_prompt: &state.system_prompt,
        screenshot_png: &state.screenshot,
        prior_actions: &state.prior_actions,
        screen_width: profile.screen_width,
        screen_height: profile.screen_height,
    }
}

async fn handle_response<R>(
    runner: &R,
    session: &R::Session,
    audit: &AuditLog,
    tracker: &mut BudgetTracker,
    state: &mut LoopState,
    response: CuStepResponse,
    started_at: Instant,
) -> Result<Option<WindowsCuOutcome>, CuError>
where
    R: CuStepRunner + ?Sized,
{
    match response {
        CuStepResponse::Stop {
            reason,
            text,
            tokens_used,
        } => Ok(Some(stop_outcome(
            reason, text, tokens_used, tracker, state, started_at,
        ))),
        CuStepResponse::ToolCall {
            action,
            tokens_used,
        } => handle_tool_call(runner, session, audit, tracker, state, action, tokens_used).await,
    }
}

fn stop_outcome(
    reason: StopReason,
    text: Option<String>,
    tokens_used: u32,
    tracker: &mut BudgetTracker,
    state: &LoopState,
    started_at: Instant,
) -> WindowsCuOutcome {
    if let BudgetVerdict::Exhausted { reason } = tracker.charge_step(tokens_used) {
        return WindowsCuExecutor::handoff(reason, state.steps.clone());
    }
    match reason {
        StopReason::Success => WindowsCuExecutor::completed(
            text.unwrap_or_else(|| "Windows computer-use action completed".into()),
            state.steps.clone(),
            started_at.elapsed(),
        ),
        StopReason::Failure => WindowsCuExecutor::handoff(
            text.unwrap_or_else(|| "model emitted stop_failure".into()),
            state.steps.clone(),
        ),
    }
}

async fn handle_tool_call<R>(
    runner: &R,
    session: &R::Session,
    audit: &AuditLog,
    tracker: &mut BudgetTracker,
    state: &mut LoopState,
    action: CuAction,
    tokens_used: u32,
) -> Result<Option<WindowsCuOutcome>, CuError>
where
    R: CuStepRunner + ?Sized,
{
    // Risk-tiered computer-use floor (DEC-051): ACT ops (click, key, drag,
    // navigate, …) hold for operator approval; AUTONOMOUS ops (observe + STAGE
    // input like `type`/scroll/cursor-move) execute. Unknown → Act (fail
    // closed). Identical policy to the macOS loop.
    if cu_risk::classify(&action).is_act() && !state.act_approved {
        return Ok(Some(WindowsCuExecutor::handoff(
            format!(
                "computer-use floor: ACT operation `{}` held for operator approval \
                 (risk-tiered floor, DEC-051) — Approve & run to resume",
                action_kind(&action)
            ),
            state.steps.clone(),
        )));
    }
    if let BudgetVerdict::Exhausted { reason } = tracker.charge_step(tokens_used) {
        return Ok(Some(WindowsCuExecutor::handoff(reason, state.steps.clone())));
    }
    let step = tracker.step();
    let result = match runner.execute_step_recorded(session, &action).await {
        Ok(result) => result,
        Err(err) => {
            return Ok(Some(WindowsCuExecutor::handoff(
                err.to_string(),
                state.steps.clone(),
            )))
        }
    };
    state
        .prior_actions
        .push(format!("step {step}: {}", action.summary()));
    state.screenshot = runner.capture_screenshot(session).await?;
    append_step(audit, tracker, state, step, action, result, tokens_used)?;
    Ok(None)
}

fn append_step(
    audit: &AuditLog,
    tracker: &BudgetTracker,
    state: &mut LoopState,
    step: u32,
    action: CuAction,
    result: ActionResult,
    tokens_used: u32,
) -> Result<(), CuError> {
    let screenshot_relpath = audit.save_screenshot(step, &state.screenshot)?;
    audit.append(&CuAuditEvent {
        step,
        at: chrono::Utc::now().to_rfc3339(),
        action: action.clone(),
        result: result.clone(),
        screenshot_relpath,
        tokens_used,
        budget: tracker.snapshot(),
    })?;
    state.steps.push(step_record(step, action, result));
    Ok(())
}

fn step_record(step: u32, action: CuAction, result: ActionResult) -> WindowsStepRecord {
    WindowsStepRecord {
        step,
        action_kind: action_kind(&action).to_string(),
        action_summary: action.summary(),
        result: WindowsActionResult {
            ok: result.ok,
            text: result.text,
            front_app_after: None,
        },
        stop_signal: None,
    }
}

#[async_trait]
impl CuStepRunner for WindowsCuExecutor {
    type Session = WindowsCuSession;

    async fn preflight(&self) -> Result<(), CuError> {
        let report = WindowsCuExecutor::preflight(self).await;
        if report.all_passed() {
            Ok(())
        } else {
            Err(CuError::Protocol(report.actionable_summary()))
        }
    }

    async fn warm_up(&self, session: &Self::Session) -> Result<Vec<u8>, CuError> {
        WindowsCuExecutor::warm_up(self).map_err(|e| windows_to_cu(e, "windows.warm_up"))?;
        primary_screenshot(self, session)
    }

    async fn capture_screenshot(&self, session: &Self::Session) -> Result<Vec<u8>, CuError> {
        primary_screenshot(self, session)
    }

    async fn execute_step_recorded(
        &self,
        _session: &Self::Session,
        action: &CuAction,
    ) -> Result<ActionResult, CuError> {
        let result = post(action, self.config())
            .await
            .map_err(|e| windows_to_cu(e, "windows.action"))?;
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

/// Capture the primary monitor as a single PNG for the loop's
/// screenshot→model step. Reuses the real per-monitor PNG path (#2919).
fn primary_screenshot(exec: &WindowsCuExecutor, session: &WindowsCuSession) -> Result<Vec<u8>, CuError> {
    let shots = exec
        .warm_up_screenshots(&session.screenshot_dir)
        .map_err(|e| windows_to_cu(e, "windows.screenshot"))?;
    let primary = shots
        .iter()
        .find(|s| s.info.is_primary)
        .or_else(|| shots.first())
        .ok_or_else(|| CuError::Protocol("no monitor captured".into()))?;
    Ok(primary.image_bytes.clone())
}
