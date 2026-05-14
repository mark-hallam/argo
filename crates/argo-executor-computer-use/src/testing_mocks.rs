//! Scripted [`AnthropicCall`] + [`E2bCall`] doubles for the CU
//! test harness. Extracted from [`crate::testing`] under the
//! 500-LOC per-file invariant so future helpers can be added to
//! either side without tripping the ceiling.
//!
//! Re-exported from [`crate::testing`] so callers see the same
//! public surface (`argo_executor_computer_use::testing::Mock*`).

use crate::anthropic::{AnthropicCall, CuStepRequest, CuStepResponse};
use crate::e2b::{ActionResult, CuAction, E2bCall, E2bSandbox};
use crate::e2b_reap::E2bSandboxInfo;
use crate::CuError;
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

/// One canned reply the [`MockAnthropic`] returns for a step call.
/// `Ok` plays back the ordered sequence of parsed step responses
/// for the turn (one entry per `tool_use` block, plus an optional
/// terminal stop signal); `Protocol` plays back a
/// [`CuError::Protocol`] to simulate a malformed model reply.
#[derive(Debug, Clone)]
pub enum CannedResponse {
    /// Replay these parsed step responses, in order, as one turn.
    Ok(Vec<CuStepResponse>),
    /// Replay this protocol-error string.
    Protocol(String),
}

impl CannedResponse {
    /// Convenience constructor: wrap a single [`CuStepResponse`]
    /// as a one-entry [`CannedResponse::Ok`]. Most scripted turns
    /// emit a single action; this keeps test fixtures readable.
    #[must_use]
    pub fn one(resp: CuStepResponse) -> Self {
        CannedResponse::Ok(vec![resp])
    }
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
    async fn step<'a>(
        &self,
        _req: CuStepRequest<'a>,
    ) -> Result<Vec<CuStepResponse>, CuError> {
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
    screenshot_calls: Mutex<u32>,
    sandboxes: Mutex<Vec<E2bSandboxInfo>>,
    close_errors: Mutex<HashMap<String, String>>,
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
            screenshot_calls: Mutex::new(0),
            sandboxes: Mutex::new(Vec::new()),
            close_errors: Mutex::new(HashMap::new()),
        }
    }

    /// Script the rows [`E2bCall::list_sandboxes`] returns.
    #[must_use]
    pub fn with_sandboxes(self, sandboxes: Vec<E2bSandboxInfo>) -> Self {
        if let Ok(mut g) = self.sandboxes.lock() { *g = sandboxes; }
        self
    }

    /// Configure [`E2bCall::close_sandbox`] to fail for `id` with `message`.
    #[must_use]
    pub fn with_close_error(self, id: impl Into<String>, message: impl Into<String>) -> Self {
        if let Ok(mut g) = self.close_errors.lock() { g.insert(id.into(), message.into()); }
        self
    }

    /// CU actions the mock observed in order. The harness reads
    /// this back into the [`super::TestRun`] for tests to assert against.
    #[must_use]
    pub fn received_actions(&self) -> Vec<CuAction> {
        match self.received.lock() {
            Ok(g) => g.clone(),
            Err(p) => p.into_inner().clone(),
        }
    }

    /// Number of `screenshot()` HTTP calls the mock observed. The
    /// double-screenshot regression guard reads this back via
    /// [`super::TestRun::screenshot_calls`] to confirm the loop reuses
    /// inline `screenshot_b64` instead of paying a second round-trip.
    #[must_use]
    pub fn screenshot_calls(&self) -> u32 {
        self.screenshot_calls
            .lock()
            .map(|g| *g)
            .unwrap_or(0)
    }
}

#[async_trait]
impl E2bCall for MockE2bClient {
    async fn open_sandbox(&self, _template: &str) -> Result<E2bSandbox, CuError> {
        Ok(E2bSandbox {
            id: "mock-sandbox".into(),
        })
    }

    async fn screenshot(&self, _sandbox: &E2bSandbox) -> Result<Vec<u8>, CuError> {
        if let Ok(mut g) = self.screenshot_calls.lock() {
            *g += 1;
        }
        Ok(self.screenshot_png.clone())
    }

    async fn execute_action(
        &self,
        _sandbox: &E2bSandbox,
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

    async fn close_sandbox(&self, sandbox: &E2bSandbox) -> Result<(), CuError> {
        if let Ok(g) = self.close_errors.lock() {
            if let Some(msg) = g.get(&sandbox.id) {
                return Err(CuError::Protocol(msg.clone()));
            }
        }
        Ok(())
    }

    async fn list_sandboxes(&self) -> Result<Vec<E2bSandboxInfo>, CuError> {
        Ok(self.sandboxes.lock().map(|g| g.clone()).unwrap_or_default())
    }
}
