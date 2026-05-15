//! CU per-step loop ([`run_loop`]) + per-profile Anthropic-client
//! builder ([`anthropic_for`]), extracted from `executor.rs` to
//! keep that file under the 500-LOC ceiling. Control-flow types
//! sit alongside in `executor_loop_exit.rs`.

use std::sync::Arc;

use crate::anthropic::{AnthropicClient, CuStepRequest, CuStepResponse, StopReason};
use crate::audit::{AuditLog, CuAuditEvent};
use crate::budget::{BudgetTracker, BudgetVerdict};
use crate::cu_step_runner::CuStepRunner;
use crate::e2b::{decode_inline_screenshot, ActionResult, CuAction};
use crate::executor::AnthropicClientTemplate;
use crate::executor_loop_exit::{stringify_verdict, LoopExit};
use crate::executor_types::{RunRequest, RunResult};
use crate::profile::Profile;
use crate::restart::{self, RestartCoordinator};
use crate::screenshot_persist::ScreenshotPersister;
use crate::CuError;

pub(crate) fn anthropic_for(
    template: &AnthropicClientTemplate,
    profile: &Profile,
) -> Result<AnthropicClient, CuError> {
    AnthropicClient::new(
        template.base_url.clone(),
        template.api_key.clone(),
        profile.model.clone(),
        template.tool_variant.clone(),
        template.per_request_timeout_seconds,
    )
}

/// Per-step CU loop. Drives Anthropic + a platform runner until the
/// model emits a stop reason or the budget is exhausted.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_loop<R>(
    runner: &R,
    template: &AnthropicClientTemplate,
    screenshot_persister: Option<&Arc<dyn ScreenshotPersister>>,
    req: &RunRequest,
    profile: &Profile,
    session: &R::Session,
    audit: &AuditLog,
    tracker: &mut BudgetTracker,
) -> Result<LoopExit, CuError>
where
    R: CuStepRunner + ?Sized,
{
    let anthropic = anthropic_for(template, profile)?;
    let mut state = LoopState::new(req, profile, session.clone());

    // Step 0: capture the initial screenshot before sending
    // the first message.
    runner.preflight().await?;
    state.screenshot = runner.warm_up(&state.session).await?;
    persist_step(screenshot_persister, req, 0, &state.screenshot).await;

    loop {
        let responses = anthropic
            .step(CuStepRequest {
                system_prompt: &state.system_prompt,
                screenshot_png: &state.screenshot,
                prior_actions: &state.prior_actions,
                screen_width: profile.screen_width,
                screen_height: profile.screen_height,
            })
            .await;
        let responses = match responses {
            Ok(r) => r,
            Err(e) if e.is_recoverable_to_handoff() => {
                match recover_after_error(runner, &e, "anthropic_step", profile, req, &mut state)
                    .await?
                {
                    Some(handoff) => return Ok(LoopExit::Handoff(handoff)),
                    None => continue,
                }
            }
            Err(e) => return Err(e),
        };
        match process_turn(
            runner,
            screenshot_persister,
            req,
            profile,
            audit,
            tracker,
            &mut state,
            responses,
        )
        .await?
        {
            TurnFlow::Continue => {}
            TurnFlow::Exit(exit) => return Ok(exit),
        }
    }
}

struct LoopState<S> {
    prior_actions: Vec<String>,
    last_text: Option<String>,
    session: S,
    screenshot: Vec<u8>,
    system_prompt: String,
    coordinator: RestartCoordinator,
}

impl<S> LoopState<S> {
    fn new(req: &RunRequest, profile: &Profile, session: S) -> Self {
        Self {
            prior_actions: Vec::new(),
            last_text: None,
            session,
            screenshot: Vec::new(),
            system_prompt: req.system_prompt.clone(),
            coordinator: RestartCoordinator::new(profile.restart.clone()),
        }
    }
}

enum TurnFlow {
    Continue,
    Exit(LoopExit),
}

enum ToolFlow {
    Recorded(PendingAudit),
    Restarted,
    Exit(LoopExit),
}

#[allow(clippy::too_many_arguments)]
async fn process_turn<R>(
    runner: &R,
    screenshot_persister: Option<&Arc<dyn ScreenshotPersister>>,
    req: &RunRequest,
    profile: &Profile,
    audit: &AuditLog,
    tracker: &mut BudgetTracker,
    state: &mut LoopState<R::Session>,
    responses: Vec<CuStepResponse>,
) -> Result<TurnFlow, CuError>
where
    R: CuStepRunner + ?Sized,
{
    let mut pending: Vec<PendingAudit> = Vec::new();
    let mut early_exit: Option<LoopExit> = None;
    for resp in responses {
        match resp {
            CuStepResponse::Stop {
                reason,
                text,
                tokens_used,
            } => {
                early_exit = Some(handle_stop(reason, text, tokens_used, tracker, state));
                break;
            }
            CuStepResponse::ToolCall {
                action,
                tokens_used,
            } => match handle_tool_call(runner, req, profile, tracker, state, action, tokens_used)
                .await?
            {
                ToolFlow::Recorded(entry) => pending.push(entry),
                ToolFlow::Restarted => return Ok(TurnFlow::Continue),
                ToolFlow::Exit(exit) => {
                    early_exit = Some(exit);
                    break;
                }
            },
        }
    }
    flush_pending(
        runner,
        screenshot_persister,
        req,
        audit,
        tracker,
        state,
        pending,
    )
    .await?;
    Ok(match early_exit {
        Some(exit) => TurnFlow::Exit(exit),
        None => TurnFlow::Continue,
    })
}

fn handle_stop<S>(
    reason: StopReason,
    text: Option<String>,
    tokens_used: u32,
    tracker: &mut BudgetTracker,
    state: &mut LoopState<S>,
) -> LoopExit {
    let verdict = tracker.charge_step(tokens_used);
    if matches!(verdict, BudgetVerdict::Exhausted { .. }) {
        return LoopExit::Handoff(stringify_verdict(verdict));
    }
    if text.is_some() {
        state.last_text = text;
    }
    match reason {
        StopReason::Success => LoopExit::Success(RunResult {
            final_text: state.last_text.clone(),
            actions_taken: state.prior_actions.clone(),
            tokens_used: tracker.tokens_used(),
        }),
        StopReason::Failure => LoopExit::Handoff(format!(
            "model emitted stop_failure: {}",
            state.last_text.clone().unwrap_or_default()
        )),
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_tool_call<R>(
    runner: &R,
    req: &RunRequest,
    profile: &Profile,
    tracker: &mut BudgetTracker,
    state: &mut LoopState<R::Session>,
    action: CuAction,
    tokens_used: u32,
) -> Result<ToolFlow, CuError>
where
    R: CuStepRunner + ?Sized,
{
    let verdict = tracker.charge_step(tokens_used);
    if let BudgetVerdict::Exhausted { .. } = verdict {
        return Ok(ToolFlow::Exit(LoopExit::Handoff(stringify_verdict(
            verdict,
        ))));
    }
    let summary = action.summary();
    let result = match runner.execute_step_recorded(&state.session, &action).await {
        Ok(r) => r,
        Err(e) if e.is_recoverable_to_handoff() => {
            match recover_after_error(runner, &e, "sandbox_action", profile, req, state).await? {
                Some(handoff) => {
                    return Ok(ToolFlow::Exit(LoopExit::Handoff(format!(
                        "{handoff} (step {})",
                        tracker.step()
                    ))));
                }
                None => return Ok(ToolFlow::Restarted),
            }
        }
        Err(e) => return Err(e),
    };
    if let Some(t) = result.text.clone() {
        state.last_text = Some(t);
    }
    let step = tracker.step();
    state.prior_actions.push(format!("step {step}: {summary}"));
    Ok(ToolFlow::Recorded(PendingAudit {
        step,
        action,
        result,
        tokens_used,
    }))
}

async fn recover_after_error<R>(
    runner: &R,
    err: &CuError,
    stage: &'static str,
    profile: &Profile,
    req: &RunRequest,
    state: &mut LoopState<R::Session>,
) -> Result<Option<String>, CuError>
where
    R: CuStepRunner + ?Sized,
{
    runner
        .recover_after_error(
            err,
            stage,
            profile,
            &mut state.coordinator,
            &mut state.session,
            &mut state.screenshot,
            &mut state.system_prompt,
            &req.system_prompt,
        )
        .await
}

async fn flush_pending<R>(
    runner: &R,
    screenshot_persister: Option<&Arc<dyn ScreenshotPersister>>,
    req: &RunRequest,
    audit: &AuditLog,
    tracker: &BudgetTracker,
    state: &mut LoopState<R::Session>,
    pending: Vec<PendingAudit>,
) -> Result<(), CuError>
where
    R: CuStepRunner + ?Sized,
{
    if pending.is_empty() {
        return Ok(());
    }
    state.screenshot = match pending
        .last()
        .and_then(|p| p.result.screenshot_b64.as_deref())
    {
        Some(b64) => decode_inline_screenshot(b64)?,
        None => runner.capture_screenshot(&state.session).await?,
    };
    let final_step = pending
        .last()
        .map(|p| p.step)
        .unwrap_or_else(|| tracker.step());
    persist_step(screenshot_persister, req, final_step, &state.screenshot).await;
    let relpath = audit.save_screenshot(final_step, &state.screenshot)?;
    for entry in pending {
        audit.append(&CuAuditEvent {
            step: entry.step,
            at: chrono::Utc::now().to_rfc3339(),
            action: entry.action,
            result: entry.result,
            screenshot_relpath: relpath.clone(),
            tokens_used: entry.tokens_used,
            budget: tracker.snapshot(),
        })?;
    }
    if state.coordinator.should_snapshot(final_step) {
        state.coordinator.record_snapshot(restart::take_snapshot(
            &state.screenshot,
            final_step,
            &req.system_prompt,
            &state.prior_actions,
        ));
    }
    Ok(())
}

/// Per-action audit info accumulated during a multi-action turn.
/// Flushed after the post-turn screenshot so all rows can
/// reference it.
struct PendingAudit {
    step: u32,
    action: CuAction,
    result: ActionResult,
    tokens_used: u32,
}

async fn persist_step(
    persister: Option<&Arc<dyn ScreenshotPersister>>,
    req: &RunRequest,
    step: u32,
    png: &[u8],
) {
    crate::screenshot_persist::try_persist(
        persister,
        &req.tenant_id,
        &req.action_id,
        req.session_id.as_ref(),
        step,
        png,
    )
    .await;
}
