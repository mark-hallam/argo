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

use crate::anthropic::{AnthropicCall, CuStepRequest, CuStepResponse, StopReason};
use crate::audit::{AuditLog, AuditRow};
use crate::budget::{BudgetTracker, BudgetVerdict};
use crate::e2b::{ActionResult, CuAction, E2bCall, Sandbox};
use crate::executor::{Outcome, RunRequest, RunResult};
use crate::profile::Profile;
use crate::CuError;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Mutex;

/// One canned reply the [`MockAnthropic`] returns for a step call.
/// `Ok` plays back a parsed [`CuStepResponse`]; `Protocol` plays
/// back a [`CuError::Protocol`] to simulate a malformed model
/// reply.
#[derive(Debug, Clone)]
pub enum CannedResponse {
    /// Replay this parsed step response.
    Ok(CuStepResponse),
    /// Replay this protocol-error string.
    Protocol(String),
}

/// Scripted [`AnthropicCall`] implementation. Each `step` call
/// pops the next entry. When the script is empty, returns a
/// `Protocol` error so the failing-test panic carries an
/// actionable signal ("script exhausted at step N").
#[derive(Debug)]
pub struct MockAnthropic {
    script: Mutex<VecDeque<CannedResponse>>,
    received: Mutex<u32>,
}

impl MockAnthropic {
    /// Build a scripted mock from the given sequence of replies.
    #[must_use]
    pub fn scripted(responses: Vec<CannedResponse>) -> Self {
        Self {
            script: Mutex::new(responses.into_iter().collect()),
            received: Mutex::new(0),
        }
    }

    /// Number of `step` calls observed so far.
    #[must_use]
    pub fn calls_observed(&self) -> u32 {
        self.received
            .lock()
            .map(|c| *c)
            .unwrap_or(0)
    }
}

#[async_trait]
impl AnthropicCall for MockAnthropic {
    async fn step<'a>(&self, _req: CuStepRequest<'a>) -> Result<CuStepResponse, CuError> {
        if let Ok(mut c) = self.received.lock() {
            *c += 1;
        }
        let mut script = match self.script.lock() {
            Ok(s) => s,
            Err(p) => p.into_inner(),
        };
        match script.pop_front() {
            Some(CannedResponse::Ok(r)) => Ok(r),
            Some(CannedResponse::Protocol(msg)) => Err(CuError::Protocol(msg)),
            None => Err(CuError::Protocol(
                "MockAnthropic script exhausted — test expected fewer turns".into(),
            )),
        }
    }
}

/// One `(expected, response)` pair the [`MockE2bClient`] uses for
/// each `execute_action`. If the loop emits a different action at
/// this position, the mock panics with the expected-vs-received
/// diff — that's the regression-guard signal for parser bugs.
#[derive(Debug, Clone)]
pub struct ScriptedAction {
    /// The CU action the harness expects at this step.
    pub expected: CuAction,
    /// The synthesised sandbox reply.
    pub response: ActionResult,
}

/// Scripted [`E2bCall`] implementation. Holds a queue of
/// [`ScriptedAction`] entries + a fixed PNG that every
/// `screenshot` call returns. Records the action sequence so
/// tests can assert the trail in order.
#[derive(Debug)]
pub struct MockE2bClient {
    actions: Mutex<VecDeque<ScriptedAction>>,
    received: Mutex<Vec<CuAction>>,
    screenshot_png: Vec<u8>,
}

impl MockE2bClient {
    /// Build a scripted mock with the given expected-action /
    /// response pairs and a fixed PNG returned by every
    /// `screenshot` call.
    #[must_use]
    pub fn scripted(actions: Vec<ScriptedAction>, screenshot_png: Vec<u8>) -> Self {
        Self {
            actions: Mutex::new(actions.into_iter().collect()),
            received: Mutex::new(Vec::new()),
            screenshot_png,
        }
    }

    /// CU actions the mock observed in order. The harness reads
    /// this back into the [`TestRun`] for tests to assert against.
    #[must_use]
    pub fn received_actions(&self) -> Vec<CuAction> {
        match self.received.lock() {
            Ok(g) => g.clone(),
            Err(p) => p.into_inner().clone(),
        }
    }
}

#[async_trait]
impl E2bCall for MockE2bClient {
    async fn open_sandbox(&self, _template: &str) -> Result<Sandbox, CuError> {
        Ok(Sandbox {
            id: "mock-sandbox".into(),
        })
    }

    async fn screenshot(&self, _sandbox: &Sandbox) -> Result<Vec<u8>, CuError> {
        Ok(self.screenshot_png.clone())
    }

    async fn execute_action(
        &self,
        _sandbox: &Sandbox,
        action: &CuAction,
    ) -> Result<ActionResult, CuError> {
        let mut actions = match self.actions.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let scripted = actions.pop_front().unwrap_or_else(|| {
            panic!(
                "MockE2bClient: received unexpected action {action:?} \
                 — script exhausted (received already: {received:?})",
                received = self.received_actions()
            )
        });
        assert_eq!(
            &scripted.expected, action,
            "MockE2bClient: action sequence mismatch — expected vs received differ",
        );
        if let Ok(mut g) = self.received.lock() {
            g.push(action.clone());
        }
        Ok(scripted.response)
    }

    async fn close_sandbox(&self, _sandbox: &Sandbox) -> Result<(), CuError> {
        Ok(())
    }
}

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
        let resp = anthropic
            .step(CuStepRequest {
                system_prompt: &req.system_prompt,
                screenshot_png: &screenshot,
                prior_actions: &prior_actions,
                screen_width: profile.screen_width,
                screen_height: profile.screen_height,
            })
            .await?;
        match resp {
            CuStepResponse::Stop {
                reason,
                text,
                tokens_used,
            } => {
                let verdict = tracker.charge_step(tokens_used);
                if let BudgetVerdict::Exhausted { reason: r } = verdict {
                    return Ok(Outcome::HumanHandoff {
                        reason: r,
                        bundle_dir: bundle_dir.to_path_buf(),
                        budget: tracker.snapshot(),
                    });
                }
                last_text = text.or(last_text);
                let snap = tracker.snapshot();
                return Ok(match reason {
                    StopReason::Success => Outcome::Completed {
                        result: RunResult {
                            final_text: last_text,
                            actions_taken: prior_actions,
                            tokens_used: tracker.tokens_used(),
                        },
                        bundle_dir: bundle_dir.to_path_buf(),
                        budget: snap,
                    },
                    StopReason::Failure => Outcome::HumanHandoff {
                        reason: format!(
                            "model emitted stop_failure: {}",
                            last_text.unwrap_or_default()
                        ),
                        bundle_dir: bundle_dir.to_path_buf(),
                        budget: snap,
                    },
                });
            }
            CuStepResponse::ToolCall {
                action,
                tokens_used,
            } => {
                let verdict = tracker.charge_step(tokens_used);
                if let BudgetVerdict::Exhausted { reason: r } = verdict {
                    return Ok(Outcome::HumanHandoff {
                        reason: r,
                        bundle_dir: bundle_dir.to_path_buf(),
                        budget: tracker.snapshot(),
                    });
                }
                let summary = action.summary();
                let result = e2b.execute_action(&sandbox, &action).await?;
                if let Some(t) = result.text.clone() {
                    last_text = Some(t);
                }
                // Always re-screenshot — mirrors the production
                // loop's behaviour (both inline-b64 and absent
                // branches do the same thing today).
                screenshot = e2b.screenshot(&sandbox).await?;
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
            }
        }
    }
}

