//! Anthropic CU loop driver for `MacosCuExecutor`.
//!
//! Extracted from `executor.rs` under the
//! `codex-split-files-450-500-wave` brief to keep that file below
//! the 420-LOC headroom target. Public surface unchanged — the
//! orchestrator entry point [`MacosCuExecutor::run`] is still the
//! callable API; this module owns the per-step driver only.
use crate::events::{action_kind, MacosActionResult};
use crate::executor::{MacosCuExecutor, MacosCuOutcome, MacosCuRunRequest, MacosStepRecord};
use argo_executor_computer_use::anthropic::{
    AnthropicCall, CuStepRequest, CuStepResponse, StopReason,
};
use argo_executor_computer_use::audit::{AuditLog, CuAuditEvent};
use argo_executor_computer_use::budget::{BudgetTracker, BudgetVerdict};
use argo_executor_computer_use::e2b::{ActionResult, CuAction};
use argo_executor_computer_use::profile::Profile;
use argo_executor_computer_use::{CuError, CuStepRunner};
use std::time::Instant;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_loop_with_client<R, A>(
    runner: &R,
    anthropic: &A,
    req: &MacosCuRunRequest,
    profile: &Profile,
    session: &R::Session,
    audit: &AuditLog,
    tracker: &mut BudgetTracker,
    started_at: Instant,
) -> Result<MacosCuOutcome, CuError>
where
    R: CuStepRunner + ?Sized,
    A: AnthropicCall + ?Sized,
{
    runner.preflight().await?;
    let mut state = LoopState::new(req.system_prompt.clone(), runner.warm_up(session).await?);
    audit.save_screenshot(0, &state.screenshot)?;
    loop {
        let responses = anthropic.step(step_request(profile, &state)).await?;
        for response in responses {
            if let Some(outcome) = handle_response(
                runner, session, audit, tracker, &mut state, response, started_at,
            )
            .await?
            {
                return Ok(outcome);
            }
        }
    }
}

pub(crate) struct LoopState {
    prior_actions: Vec<String>,
    steps: Vec<MacosStepRecord>,
    screenshot: Vec<u8>,
    system_prompt: String,
}

impl LoopState {
    fn new(system_prompt: String, screenshot: Vec<u8>) -> Self {
        Self {
            prior_actions: Vec::new(),
            steps: Vec::new(),
            screenshot,
            system_prompt,
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
) -> Result<Option<MacosCuOutcome>, CuError>
where
    R: CuStepRunner + ?Sized,
{
    match response {
        CuStepResponse::Stop {
            reason,
            text,
            tokens_used,
        } => Ok(Some(stop_outcome(
            reason,
            text,
            tokens_used,
            tracker,
            state,
            started_at,
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
) -> MacosCuOutcome {
    if let BudgetVerdict::Exhausted { reason } = tracker.charge_step(tokens_used) {
        return MacosCuExecutor::handoff(reason, state.steps.clone());
    }
    match reason {
        StopReason::Success => MacosCuExecutor::completed(
            text.unwrap_or_else(|| "macOS computer-use action completed".into()),
            state.steps.clone(),
            started_at.elapsed(),
        ),
        StopReason::Failure => MacosCuExecutor::handoff(
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
) -> Result<Option<MacosCuOutcome>, CuError>
where
    R: CuStepRunner + ?Sized,
{
    if let BudgetVerdict::Exhausted { reason } = tracker.charge_step(tokens_used) {
        return Ok(Some(MacosCuExecutor::handoff(reason, state.steps.clone())));
    }
    let step = tracker.step();
    let result = match runner.execute_step_recorded(session, &action).await {
        Ok(result) => result,
        Err(err) => {
            return Ok(Some(MacosCuExecutor::handoff(
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

fn step_record(step: u32, action: CuAction, result: ActionResult) -> MacosStepRecord {
    MacosStepRecord {
        step,
        action_kind: action_kind(&action).to_string(),
        action_summary: action.summary(),
        result: MacosActionResult {
            ok: result.ok,
            text: result.text,
            front_app_after: None,
        },
        stop_signal: None,
    }
}
