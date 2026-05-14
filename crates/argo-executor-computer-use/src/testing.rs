//! Offline test harness for the CU loop. Wraps `MockAnthropic`
//! and `MockE2bClient` (scripted, deterministic stand-ins for
//! `AnthropicClient` and `E2bClient`) and exposes
//! [`cu_harness_run`] — a parallel loop that mirrors the
//! production `ComputerUseExecutor::run_loop` control flow against
//! the mocks.
//!
//! The harness drives a parallel loop rather than wrapping
//! [`crate::ComputerUseExecutor`] directly because `executor.rs`
//! is at 497/500 LOC; refactoring it to be generic over the new
//! [`AnthropicCall`] / [`E2bCall`] traits would push it past the
//! per-file invariant. The harness still exercises the parser,
//! audit log, budget tracker, action types, and the decision
//! logic (Stop vs ToolCall vs budget exhaustion vs Protocol
//! error) that drove the Mission Claim 5 regressions this brief
//! is protecting against.
//!
//! Gated behind `#[cfg(any(test, feature = "test-harness"))]`
//! so it only compiles in test builds or when a downstream crate
//! opts in via the `test-harness` feature.
//!
//! The scripted [`MockAnthropic`] / [`MockE2bClient`] doubles live
//! in [`crate::testing_mocks`] and are re-exported here so callers
//! continue to see `argo_executor_computer_use::testing::Mock*`.

use crate::anthropic::{AnthropicCall, CuStepRequest, CuStepResponse, StopReason};
use crate::audit::{AuditLog, AuditRow};
use crate::budget::{BudgetTracker, BudgetVerdict};
use crate::e2b::{decode_inline_screenshot, ActionResult, CuAction, E2bCall};
use crate::executor::{Outcome, RunRequest, RunResult};
use crate::profile::Profile;
use crate::CuError;
use std::path::Path;

pub use crate::testing_mocks::{CannedResponse, MockAnthropic, MockE2bClient, ScriptedAction};

/// Full set of inputs the harness needs for one run.
#[derive(Debug)]
pub struct TestScript {
    /// Synthetic RunRequest piped into the parallel loop.
    pub request: RunRequest,
    /// Profile used for budget caps + screen dimensions. Tests
    /// can lower caps to provoke budget exhaustion deterministically.
    pub profile: Profile,
    /// Bytes returned for every `screenshot` call. Non-empty so
    /// downstream b64-encoding doesn't degenerate.
    pub screenshot_png: Vec<u8>,
    /// Scripted Anthropic replies, played in order.
    pub anthropic: Vec<CannedResponse>,
    /// Scripted (expected, response) action pairs, played in order.
    pub e2b: Vec<ScriptedAction>,
}

impl TestScript {
    /// Convenience: default profile + a `demo` RunRequest, with
    /// caller-supplied Anthropic + E2B sequences.
    #[must_use]
    pub fn new(anthropic: Vec<CannedResponse>, e2b: Vec<ScriptedAction>) -> Self {
        Self {
            request: RunRequest::demo("act-harness-1", "marketplace_browser"),
            profile: Profile::default(),
            screenshot_png: b"\x89PNG\r\nharness".to_vec(),
            anthropic,
            e2b,
        }
    }
}

/// What the harness captured from one run.
#[derive(Debug)]
pub struct TestRun {
    /// Outcome the parallel loop produced, or the propagated
    /// `CuError` if the loop bailed.
    pub outcome: Result<Outcome, CuError>,
    /// CU actions the mock E2B client recorded in order.
    pub received_actions: Vec<CuAction>,
    /// Number of `step` calls the mock Anthropic client observed.
    pub anthropic_calls: u32,
    /// Audit rows captured into a tempdir during the run.
    pub audit_rows: Vec<AuditRow>,
    /// Final step count when the loop returned.
    pub steps: u32,
    /// Number of `screenshot()` HTTP calls the mock observed.
    /// Used by `mission-cu-double-screenshot-fix` regression tests
    /// to assert the loop reuses inline `screenshot_b64` instead
    /// of paying a second round-trip per action.
    pub screenshot_calls: u32,
}

/// Drive one harness invocation end-to-end. Builds tempdir-backed
/// audit storage, plumbs the mocks through a control-flow mirror
/// of [`crate::executor::ComputerUseExecutor`]'s run loop, and
/// returns the captured outcome + trail.
pub async fn cu_harness_run(script: TestScript) -> TestRun {
    let mock_anthropic = MockAnthropic::scripted(script.anthropic);
    let mock_e2b = MockE2bClient::scripted(script.e2b, script.screenshot_png);
    let (outcome, audit_rows, steps) = drive(
        &mock_anthropic,
        &mock_e2b,
        &script.request,
        &script.profile,
    )
    .await;
    TestRun {
        outcome,
        received_actions: mock_e2b.received_actions(),
        anthropic_calls: mock_anthropic.calls_observed(),
        audit_rows,
        steps,
        screenshot_calls: mock_e2b.screenshot_calls(),
    }
}

async fn drive(
    anthropic: &dyn AnthropicCall,
    e2b: &dyn E2bCall,
    req: &RunRequest,
    profile: &Profile,
) -> (Result<Outcome, CuError>, Vec<AuditRow>, u32) {
    let bundle_dir = harness_bundle_dir();
    let audit = match AuditLog::create(bundle_dir.clone()) {
        Ok(a) => a,
        Err(e) => return (Err(e), Vec::new(), 0),
    };
    let mut tracker = BudgetTracker::new(profile);
    if let Some(usd) = req.budget_usd {
        tracker = tracker.with_usd_cap(usd, 100_000);
    }
    let outcome = drive_loop(
        anthropic,
        e2b,
        req,
        profile,
        &audit,
        &mut tracker,
        &bundle_dir,
    )
    .await;
    let rows = audit.rows().unwrap_or_default();
    let _ = std::fs::remove_dir_all(&bundle_dir);
    (outcome, rows, tracker.step())
}

/// Per-call unique bundle dir under the OS temp root. Avoids
/// pulling `tempfile` into the lib's main dep graph (it's a
/// dev-only dep used by the production-path unit tests).
fn harness_bundle_dir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "argo-cu-harness-{pid}-{nanos}-{n}",
        pid = std::process::id(),
    ))
}

async fn drive_loop(
    anthropic: &dyn AnthropicCall,
    e2b: &dyn E2bCall,
    req: &RunRequest,
    profile: &Profile,
    audit: &AuditLog,
    tracker: &mut BudgetTracker,
    bundle_dir: &Path,
) -> Result<Outcome, CuError> {
    let sandbox = e2b.open_sandbox(&profile.sandbox_template).await?;
    let mut prior_actions: Vec<String> = Vec::new();
    let mut last_text: Option<String> = None;
    let mut screenshot = e2b.screenshot(&sandbox).await?;

    loop {
        let responses = anthropic
            .step(CuStepRequest {
                system_prompt: &req.system_prompt,
                screenshot_png: &screenshot,
                prior_actions: &prior_actions,
                screen_width: profile.screen_width,
                screen_height: profile.screen_height,
            })
            .await?;
        // The model may emit multiple actions in one turn (e.g.
        // `mouse_move` then `left_click`). Dispatch each in
        // order, accumulate per-action audit info, then take ONE
        // post-turn screenshot all rows reference.
        let mut pending: Vec<(u32, CuAction, ActionResult, u32)> = Vec::new();
        let mut early: Option<Outcome> = None;
        for resp in responses {
            match resp {
                CuStepResponse::Stop {
                    reason,
                    text,
                    tokens_used,
                } => {
                    let verdict = tracker.charge_step(tokens_used);
                    if let BudgetVerdict::Exhausted { reason: r } = verdict {
                        early = Some(Outcome::HumanHandoff {
                            reason: r,
                            bundle_dir: bundle_dir.to_path_buf(),
                            budget: tracker.snapshot(),
                        });
                        break;
                    }
                    last_text = text.or(last_text);
                    let snap = tracker.snapshot();
                    early = Some(match reason {
                        StopReason::Success => Outcome::Completed {
                            result: RunResult {
                                final_text: last_text.clone(),
                                actions_taken: prior_actions.clone(),
                                tokens_used: tracker.tokens_used(),
                            },
                            bundle_dir: bundle_dir.to_path_buf(),
                            budget: snap,
                        },
                        StopReason::Failure => Outcome::HumanHandoff {
                            reason: format!(
                                "model emitted stop_failure: {}",
                                last_text.clone().unwrap_or_default()
                            ),
                            bundle_dir: bundle_dir.to_path_buf(),
                            budget: snap,
                        },
                    });
                    break;
                }
                CuStepResponse::ToolCall {
                    action,
                    tokens_used,
                } => {
                    let verdict = tracker.charge_step(tokens_used);
                    if let BudgetVerdict::Exhausted { reason: r } = verdict {
                        early = Some(Outcome::HumanHandoff {
                            reason: r,
                            bundle_dir: bundle_dir.to_path_buf(),
                            budget: tracker.snapshot(),
                        });
                        break;
                    }
                    let summary = action.summary();
                    let result = e2b.execute_action(&sandbox, &action).await?;
                    if let Some(t) = result.text.clone() {
                        last_text = Some(t);
                    }
                    let step_num = tracker.step();
                    pending.push((step_num, action, result, tokens_used));
                    prior_actions.push(format!("step {step_num}: {summary}"));
                }
            }
        }

        if !pending.is_empty() {
            // Mirror production: if the sandbox shipped a
            // screenshot inline with the last action's response,
            // reuse it; otherwise fall back to a fresh HTTP fetch.
            // Keeps the harness's call cadence in sync with
            // `run_loop` so cu_harness_offline regression tests
            // catch double-fetch regressions.
            screenshot = match pending
                .last()
                .and_then(|(_, _, result, _)| result.screenshot_b64.as_deref())
            {
                Some(b64) => decode_inline_screenshot(b64)?,
                None => e2b.screenshot(&sandbox).await?,
            };
            let final_step = pending
                .last()
                .map(|(s, _, _, _)| *s)
                .unwrap_or_else(|| tracker.step());
            let relpath = audit.save_screenshot(final_step, &screenshot)?;
            for (step, action, result, tokens_used) in pending.drain(..) {
                audit.append(&AuditRow {
                    step,
                    at: chrono::Utc::now().to_rfc3339(),
                    action,
                    result,
                    screenshot_relpath: relpath.clone(),
                    tokens_used,
                    budget: tracker.snapshot(),
                })?;
            }
        }

        if let Some(o) = early {
            return Ok(o);
        }
    }
}
