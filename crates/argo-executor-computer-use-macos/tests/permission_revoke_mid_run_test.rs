//! Partial-failure audit-chain integrity for the macOS CU run loop.
//!
//! Source (evaluator round-4, 2026-05-22):
//!
//! > `mission-cu-macos-permission-revoke-mid-run-test`: a tokio test
//! > that simulates the operator revoking screen-recording permission
//! > mid-step, asserting the executor returns
//! > `MacosCuError::PermissionLost(...)` cleanly without leaking
//! > partial screenshots into the audit chain.
//!
//! Sensible deviations from the spec:
//!
//! 1. **No `MacosCuError::PermissionLost` variant exists.** Per the
//!    brief's "Files you must NOT touch" clause, this test does not
//!    add one. The run loop bubbles the screen-capture failure up as
//!    the underlying `CuError::Protocol(...)` carrying the revoke
//!    message, and we assert against that. If a conductor later
//!    introduces a typed variant, the assertions can tighten.
//! 2. **No `crate::audit_chain::verify_audit_chain_for_run` exists.**
//!    We verify chain integrity by walking `audit.rows()` and
//!    asserting every referenced `step-NNN.png` file is on disk and
//!    that no row past the failure point exists.
//!
//! See PR body for the full deviation note.

use argo_executor_computer_use::anthropic::{
    AnthropicCall, CuStepRequest, CuStepResponse, StopReason,
};
use argo_executor_computer_use::audit::AuditLog;
use argo_executor_computer_use::budget::BudgetTracker;
use argo_executor_computer_use::e2b::{ActionResult, CuAction};
use argo_executor_computer_use::profile::Profile;
use argo_executor_computer_use::restart::RestartCoordinator;
use argo_executor_computer_use::{CuError, CuStepRunner};
use argo_executor_computer_use_macos::executor_loop::run_loop_with_client;
use argo_executor_computer_use_macos::MacosCuRunRequest;
use async_trait::async_trait;
use std::sync::Mutex;
use std::time::Instant;

/// Permission-revoke sentinel the mock returns from `capture_screenshot`
/// once the operator has "flipped" the System Settings switch off.
const REVOKE_MSG: &str = "screen recording permission revoked mid-run";

/// `CuStepRunner` whose `capture_screenshot` succeeds on the first
/// post-warm-up call (step 1's follow-up screenshot) and then fails
/// on the second call (step 2's follow-up screenshot) with a
/// permission-revoke error. Mirrors what the real executor would see
/// if the operator toggled Screen Recording off in System Settings
/// between two `screencapture` invocations.
struct PermissionRevokeRunner {
    capture_calls: Mutex<u32>,
    actions: Mutex<Vec<CuAction>>,
}

impl PermissionRevokeRunner {
    fn new() -> Self {
        Self {
            capture_calls: Mutex::new(0),
            actions: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl CuStepRunner for PermissionRevokeRunner {
    type Session = ();

    async fn warm_up(&self, _session: &Self::Session) -> Result<Vec<u8>, CuError> {
        Ok(b"warm-png".to_vec())
    }

    async fn capture_screenshot(&self, _session: &Self::Session) -> Result<Vec<u8>, CuError> {
        let mut calls = self.capture_calls.lock().unwrap();
        *calls += 1;
        if *calls >= 2 {
            return Err(CuError::Protocol(REVOKE_MSG.into()));
        }
        Ok(b"step-1-png".to_vec())
    }

    async fn execute_step_recorded(
        &self,
        _session: &Self::Session,
        action: &CuAction,
    ) -> Result<ActionResult, CuError> {
        self.actions.lock().unwrap().push(action.clone());
        Ok(ActionResult {
            ok: true,
            text: Some("typed".into()),
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
        Ok(Some(format!("{stage}: {err}")))
    }
}

struct ScriptedAnthropic {
    turns: Mutex<Vec<Vec<CuStepResponse>>>,
}

#[async_trait]
impl AnthropicCall for ScriptedAnthropic {
    async fn step<'a>(&self, _req: CuStepRequest<'a>) -> Result<Vec<CuStepResponse>, CuError> {
        let mut turns = self.turns.lock().unwrap();
        if turns.is_empty() {
            return Err(CuError::Protocol(
                "ScriptedAnthropic ran out of scripted turns".into(),
            ));
        }
        Ok(turns.remove(0))
    }
}

fn profile() -> Profile {
    Profile {
        max_tokens_per_action: 1_000,
        max_wallclock_seconds: 60,
        max_steps: 8,
        screen_width: 64,
        screen_height: 64,
        ..Profile::default()
    }
}

fn run_request(audit_dir: std::path::PathBuf) -> MacosCuRunRequest {
    MacosCuRunRequest {
        action_id: "act-permission-revoke".into(),
        system_prompt: "Type two notes; the second is doomed.".into(),
        audit_dir,
        anthropic_api_key: "test-key".into(),
    }
}

#[tokio::test]
async fn permission_revoke_mid_run_keeps_audit_chain_intact() {
    let dir = tempfile::tempdir().unwrap();
    let bundle_dir = dir.path().join("bundle");
    let req = run_request(bundle_dir.clone());
    let audit = AuditLog::create(req.audit_dir.clone()).unwrap();
    let runner = PermissionRevokeRunner::new();
    // Two ToolCall turns scripted: step 1 completes, step 2's
    // post-action capture is where the mock injects the revoke.
    let anthropic = ScriptedAnthropic {
        turns: Mutex::new(vec![
            vec![CuStepResponse::ToolCall {
                action: CuAction::Type {
                    text: "note one".into(),
                },
                tokens_used: 5,
            }],
            vec![CuStepResponse::ToolCall {
                action: CuAction::Type {
                    text: "note two".into(),
                },
                tokens_used: 5,
            }],
            // Defensive: if the loop somehow recovers and asks again,
            // it must not silently succeed.
            vec![CuStepResponse::Stop {
                reason: StopReason::Failure,
                text: Some("unexpected third turn".into()),
                tokens_used: 1,
            }],
        ]),
    };
    let profile = profile();
    let mut tracker = BudgetTracker::new(&profile);

    let outcome = run_loop_with_client(
        &runner,
        &anthropic,
        &req,
        &profile,
        &(),
        &audit,
        &mut tracker,
        Instant::now(),
    )
    .await;

    // The loop must surface the failure — not swallow it into a
    // Completed outcome.
    let err = outcome.expect_err("expected loop to propagate revoke error");
    let msg = err.to_string();
    assert!(
        msg.contains(REVOKE_MSG),
        "error message did not carry revoke sentinel: {msg}"
    );

    // Exactly ONE action got dispatched (step 1). Step 2's action
    // dispatched too but its follow-up capture failed before
    // append_step ran, so the action list reflects two dispatches —
    // and that's fine. What matters is the audit chain.
    let action_count = runner.actions.lock().unwrap().len();
    assert_eq!(
        action_count, 2,
        "expected both ToolCall actions to be dispatched before the revoke fires"
    );

    // Exactly ONE completed step row in the audit chain (step 1).
    // Step 2 must NOT have a row: append_step never ran for it
    // because capture_screenshot returned Err first.
    let rows = audit.rows().unwrap();
    assert_eq!(
        rows.len(),
        1,
        "audit chain must contain exactly the completed step-1 row, found {}",
        rows.len()
    );
    assert_eq!(rows[0].step, 1, "the surviving row must be step 1");
    assert_eq!(rows[0].screenshot_relpath, "step-001.png");

    // No partial step-2 screenshot file on disk.
    assert!(
        !bundle_dir.join("step-002.png").exists(),
        "step-002.png leaked onto disk despite the mid-run revoke"
    );

    // Audit-chain integrity check (the load-bearing assertion per
    // the brief): every row's `screenshot_relpath` resolves to a
    // real file on disk, and no extra `step-NNN.png` exists past
    // the highest-numbered row.
    for row in &rows {
        let p = bundle_dir.join(&row.screenshot_relpath);
        assert!(
            p.exists(),
            "audit row references missing screenshot {}",
            p.display()
        );
    }
    let highest_row_step = rows.iter().map(|r| r.step).max().unwrap_or(0);
    for entry in std::fs::read_dir(&bundle_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // step-000.png is the warm-up screenshot — pre-loop, no row.
        // step-NNN.png for N > highest_row_step would be a leak.
        if let Some(rest) = name.strip_prefix("step-") {
            if let Some(num_str) = rest.strip_suffix(".png") {
                let n: u32 = num_str.parse().unwrap_or(0);
                if n > highest_row_step {
                    panic!(
                        "stray screenshot {name} (step {n}) past last audit row (step {highest_row_step})"
                    );
                }
            }
        }
    }
}
