//! CU per-step loop ([`run_loop`]) + per-profile Anthropic-client
//! builder ([`anthropic_for`]), extracted from `executor.rs` to
//! keep that file under the 500-LOC ceiling. Control-flow types
//! sit alongside in `executor_loop_exit.rs`.

use std::sync::Arc;

use crate::anthropic::{AnthropicClient, CuStepRequest, CuStepResponse, StopReason};
use crate::audit::{AuditLog, AuditRow};
use crate::budget::{BudgetTracker, BudgetVerdict};
use crate::e2b::{
    decode_inline_screenshot, ActionResult, CuAction, E2bClient, E2bSandbox,
};
use crate::executor::AnthropicClientTemplate;
use crate::executor_loop_exit::{handle_restart, stringify_verdict, LoopExit};
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

/// Per-step CU loop. Drives Anthropic + E2B until the model emits
/// a stop reason or the budget is exhausted.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_loop(
    e2b: &E2bClient,
    template: &AnthropicClientTemplate,
    screenshot_persister: Option<&Arc<dyn ScreenshotPersister>>,
    req: &RunRequest,
    profile: &Profile,
    sandbox: &E2bSandbox,
    audit: &AuditLog,
    tracker: &mut BudgetTracker,
) -> Result<LoopExit, CuError> {
    let anthropic = anthropic_for(template, profile)?;
    let mut prior_actions: Vec<String> = Vec::new();
    let mut last_text: Option<String> = None;
    let mut sandbox = sandbox.clone();
    let mut system_prompt = req.system_prompt.clone();
    let mut coordinator = RestartCoordinator::new(profile.restart.clone());

    // Step 0: capture the initial screenshot before sending
    // the first message.
    let mut screenshot = e2b.screenshot(&sandbox).await?;
    persist_step(screenshot_persister, req, 0, &screenshot).await;

    'outer: loop {
        let responses = anthropic
            .step(CuStepRequest {
                system_prompt: &system_prompt,
                screenshot_png: &screenshot,
                prior_actions: &prior_actions,
                screen_width: profile.screen_width,
                screen_height: profile.screen_height,
            })
            .await;
        let responses = match responses {
            Ok(r) => r,
            Err(e) if e.is_recoverable_to_handoff() => {
                match handle_restart(
                    e2b,
                    &e,
                    "anthropic_step",
                    profile,
                    &mut coordinator,
                    &mut sandbox,
                    &mut screenshot,
                    &mut system_prompt,
                    &req.system_prompt,
                )
                .await?
                {
                    Some(handoff) => return Ok(LoopExit::Handoff(handoff)),
                    None => continue,
                }
            }
            Err(e) => return Err(e),
        };

        // The model may emit several `tool_use` blocks per turn
        // (e.g. `mouse_move` then `left_click`). Dispatch each
        // sequentially, accumulate audit rows, then take ONE
        // post-turn screenshot the rows reference. A Stop or
        // budget-exhaust mid-Vec audits the actions that already
        // ran before exiting.
        let mut pending: Vec<PendingAudit> = Vec::new();
        let mut early_exit: Option<LoopExit> = None;
        for resp in responses {
            match resp {
                CuStepResponse::Stop {
                    reason,
                    text,
                    tokens_used,
                } => {
                    let verdict = tracker.charge_step(tokens_used);
                    if matches!(verdict, BudgetVerdict::Exhausted { .. }) {
                        early_exit = Some(LoopExit::Handoff(stringify_verdict(verdict)));
                        break;
                    }
                    last_text = text.or(last_text);
                    early_exit = Some(match reason {
                        StopReason::Success => LoopExit::Success(RunResult {
                            final_text: last_text.clone(),
                            actions_taken: prior_actions.clone(),
                            tokens_used: tracker.tokens_used(),
                        }),
                        StopReason::Failure => LoopExit::Handoff(format!(
                            "model emitted stop_failure: {}",
                            last_text.clone().unwrap_or_default()
                        )),
                    });
                    break;
                }
                CuStepResponse::ToolCall {
                    action,
                    tokens_used,
                } => {
                    let verdict = tracker.charge_step(tokens_used);
                    if let BudgetVerdict::Exhausted { .. } = verdict {
                        early_exit = Some(LoopExit::Handoff(stringify_verdict(verdict)));
                        break;
                    }
                    let summary = action.summary();
                    let result = match e2b.execute_action(&sandbox, &action).await {
                        Ok(r) => r,
                        Err(e) if e.is_recoverable_to_handoff() => {
                            match handle_restart(
                                e2b,
                                &e,
                                "sandbox_action",
                                profile,
                                &mut coordinator,
                                &mut sandbox,
                                &mut screenshot,
                                &mut system_prompt,
                                &req.system_prompt,
                            )
                            .await?
                            {
                                Some(handoff) => {
                                    return Ok(LoopExit::Handoff(format!(
                                        "{handoff} (step {})",
                                        tracker.step()
                                    )))
                                }
                                // Restart restored state; the
                                // partial audit for this turn is
                                // dropped (sandbox is wedged-then-
                                // recovered, those rows would not
                                // reflect on-disk truth).
                                None => continue 'outer,
                            }
                        }
                        Err(e) => return Err(e),
                    };
                    if let Some(t) = result.text.clone() {
                        last_text = Some(t);
                    }
                    let step_num = tracker.step();
                    pending.push(PendingAudit {
                        step: step_num,
                        action,
                        result,
                        tokens_used,
                    });
                    prior_actions.push(format!("step {step_num}: {summary}"));
                }
            }
        }

        // After the turn: if any actions ran, take ONE screenshot
        // and audit each row referencing it. If E2B shipped a
        // screenshot inline with the last action's response, reuse
        // it; only fall back to a fresh HTTP fetch when absent.
        // Saves one round-trip per turn.
        if !pending.is_empty() {
            screenshot = match pending
                .last()
                .and_then(|p| p.result.screenshot_b64.as_deref())
            {
                Some(b64) => decode_inline_screenshot(b64)?,
                None => e2b.screenshot(&sandbox).await?,
            };
            let final_step = pending
                .last()
                .map(|p| p.step)
                .unwrap_or_else(|| tracker.step());
            persist_step(screenshot_persister, req, final_step, &screenshot).await;
            let relpath = audit.save_screenshot(final_step, &screenshot)?;
            for entry in pending.drain(..) {
                audit.append(&AuditRow {
                    step: entry.step,
                    at: chrono::Utc::now().to_rfc3339(),
                    action: entry.action,
                    result: entry.result,
                    screenshot_relpath: relpath.clone(),
                    tokens_used: entry.tokens_used,
                    budget: tracker.snapshot(),
                })?;
            }
            // Take a snapshot at the configured cadence so a
            // SnapshotResetRetry has fresh state to seed from.
            if coordinator.should_snapshot(final_step) {
                coordinator.record_snapshot(restart::take_snapshot(
                    &screenshot,
                    final_step,
                    &req.system_prompt,
                    &prior_actions,
                ));
            }
        }

        if let Some(exit) = early_exit {
            return Ok(exit);
        }
    }
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
