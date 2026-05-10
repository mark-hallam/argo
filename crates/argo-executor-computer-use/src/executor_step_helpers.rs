//! CU per-step loop ([`run_loop`]) + per-profile Anthropic-client
//! builder ([`anthropic_for`]), extracted from `executor.rs` to
//! keep that file under the 500-LOC ceiling. Control-flow types
//! sit alongside in `executor_loop_exit.rs`.

use std::sync::Arc;

use crate::anthropic::{AnthropicClient, CuStepRequest, CuStepResponse, StopReason};
use crate::audit::{AuditLog, AuditRow};
use crate::budget::{BudgetTracker, BudgetVerdict};
use crate::e2b::{E2bClient, Sandbox};
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
    sandbox: &Sandbox,
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

    loop {
        let resp = anthropic
            .step(CuStepRequest {
                system_prompt: &system_prompt,
                screenshot_png: &screenshot,
                prior_actions: &prior_actions,
                screen_width: profile.screen_width,
                screen_height: profile.screen_height,
            })
            .await;
        let resp = match resp {
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

        match resp {
            CuStepResponse::Stop {
                reason,
                text,
                tokens_used,
            } => {
                let verdict = tracker.charge_step(tokens_used);
                if matches!(verdict, BudgetVerdict::Exhausted { .. }) {
                    return Ok(LoopExit::Handoff(stringify_verdict(verdict)));
                }
                last_text = text.or(last_text);
                return Ok(match reason {
                    StopReason::Success => LoopExit::Success(RunResult {
                        final_text: last_text,
                        actions_taken: prior_actions,
                        tokens_used: tracker.tokens_used(),
                    }),
                    StopReason::Failure => LoopExit::Handoff(format!(
                        "model emitted stop_failure: {}",
                        last_text.unwrap_or_default()
                    )),
                });
            }
            CuStepResponse::ToolCall {
                action,
                tokens_used,
            } => {
                let verdict = tracker.charge_step(tokens_used);
                if let BudgetVerdict::Exhausted { .. } = verdict {
                    return Ok(LoopExit::Handoff(stringify_verdict(verdict)));
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
                            None => continue,
                        }
                    }
                    Err(e) => return Err(e),
                };
                if let Some(t) = result.text.clone() {
                    last_text = Some(t);
                }
                // Capture a fresh screenshot for next step.
                screenshot = match result.screenshot_b64.as_deref() {
                    Some(_b64) => e2b.screenshot(&sandbox).await?,
                    None => e2b.screenshot(&sandbox).await?,
                };
                persist_step(screenshot_persister, req, tracker.step(), &screenshot).await;
                let relpath = audit.save_screenshot(tracker.step(), &screenshot)?;
                audit.append(&AuditRow {
                    step: tracker.step(),
                    at: chrono::Utc::now().to_rfc3339(),
                    action,
                    result,
                    screenshot_relpath: relpath,
                    tokens_used,
                    budget: tracker.snapshot(),
                })?;
                prior_actions.push(format!("step {}: {summary}", tracker.step()));
                // Take a snapshot at the configured cadence so a
                // SnapshotResetRetry has fresh state to seed from.
                if coordinator.should_snapshot(tracker.step()) {
                    coordinator.record_snapshot(restart::take_snapshot(
                        &screenshot,
                        tracker.step(),
                        &req.system_prompt,
                        &prior_actions,
                    ));
                }
            }
        }
    }
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
