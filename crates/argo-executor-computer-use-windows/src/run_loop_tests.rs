//! Hermetic loop-plumbing tests for the Windows CU `run()`
//! (win-cu-executor-windows-run-loop). Mirrors the macOS
//! `run_loop_tests.rs`: a `FakeRunner` (a `CuStepRunner` with a trivial
//! session) + a `ScriptedAnthropic` (`AnthropicCall`) drive
//! `run_loop_with_client` with **NO API key and NO live desktop**.
//!
//! NO THEATER: this exercises the loop PLUMBING only (turn-taking, the
//! risk-tiered ACT floor, budgeting, audit rows). It does NOT inject real
//! input or call a real model — that is the now-unblocked
//! `win-cu-executor-end-to-end-run` (operator key + live desktop).

use crate::executor_loop::run_loop_with_client;
use crate::{WindowsCuOutcome, WindowsCuRunRequest};
use argo_executor_computer_use::anthropic::{AnthropicCall, CuStepRequest, CuStepResponse, StopReason};
use argo_executor_computer_use::audit::AuditLog;
use argo_executor_computer_use::budget::BudgetTracker;
use argo_executor_computer_use::e2b::{ActionResult, CuAction};
use argo_executor_computer_use::profile::Profile;
use argo_executor_computer_use::restart::RestartCoordinator;
use argo_executor_computer_use::{CuError, CuStepRunner};
use async_trait::async_trait;
use std::sync::Mutex;
use std::time::Instant;

struct FakeRunner {
    actions: Mutex<Vec<CuAction>>,
}

#[async_trait]
impl CuStepRunner for FakeRunner {
    type Session = ();

    async fn warm_up(&self, _session: &Self::Session) -> Result<Vec<u8>, CuError> {
        Ok(b"warm-png".to_vec())
    }

    async fn capture_screenshot(&self, _session: &Self::Session) -> Result<Vec<u8>, CuError> {
        Ok(b"after-png".to_vec())
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
        Ok(self.turns.lock().unwrap().remove(0))
    }
}

fn req(dir: &tempfile::TempDir) -> WindowsCuRunRequest {
    WindowsCuRunRequest {
        action_id: "act-win".into(),
        system_prompt: "Type a short note, then stop_success.".into(),
        audit_dir: dir.path().join("bundle"),
        anthropic_api_key: "test-key".into(),
        // APPROVED run (post "Approve & run") so the loop exercises ACT
        // execution. The held path is covered by `act_op_held_when_not_approved`.
        act_approved: true,
    }
}

fn profile() -> Profile {
    Profile {
        max_tokens_per_action: 100,
        max_wallclock_seconds: 60,
        max_steps: 4,
        screen_width: 64,
        screen_height: 64,
        ..Profile::default()
    }
}

fn fake() -> FakeRunner {
    FakeRunner {
        actions: Mutex::new(Vec::new()),
    }
}

#[tokio::test]
async fn autonomous_loop_completes_with_mocked_steps() {
    let dir = tempfile::tempdir().unwrap();
    let req = req(&dir);
    let audit = AuditLog::create(req.audit_dir.clone()).unwrap();
    let runner = fake();
    let anthropic = ScriptedAnthropic {
        turns: Mutex::new(vec![
            vec![CuStepResponse::ToolCall {
                action: CuAction::Type {
                    text: "hello".into(),
                },
                tokens_used: 5,
            }],
            vec![CuStepResponse::Stop {
                reason: StopReason::Success,
                text: Some("done".into()),
                tokens_used: 3,
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
    .await
    .unwrap();

    match outcome {
        WindowsCuOutcome::Completed { summary, steps, .. } => {
            assert_eq!(summary, "done");
            assert_eq!(steps.len(), 1);
            assert_eq!(steps[0].action_kind, "type");
        }
        WindowsCuOutcome::HumanHandoff { reason, .. } => {
            panic!("expected Completed, got HumanHandoff: {reason}")
        }
    }
    assert_eq!(runner.actions.lock().unwrap().len(), 1);
    assert_eq!(audit.rows().unwrap().len(), 1);
    assert!(req.audit_dir.join("step-000.png").exists());
    assert!(req.audit_dir.join("step-001.png").exists());
}

#[tokio::test]
async fn act_op_held_when_not_approved() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = req(&dir);
    req.act_approved = false; // unapproved: the risk-tiered floor must hold
    let audit = AuditLog::create(req.audit_dir.clone()).unwrap();
    let runner = fake();
    let anthropic = ScriptedAnthropic {
        turns: Mutex::new(vec![vec![CuStepResponse::ToolCall {
            action: CuAction::Click { x: 10, y: 20 },
            tokens_used: 5,
        }]]),
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
    .await
    .unwrap();

    match outcome {
        WindowsCuOutcome::HumanHandoff { reason, .. } => assert!(
            reason.contains("ACT operation") && reason.contains("click"),
            "held reason should name the ACT op: {reason}"
        ),
        WindowsCuOutcome::Completed { summary, .. } => {
            panic!("expected HumanHandoff (held), got Completed: {summary}")
        }
    }
    // The host action was NEVER executed — the floor held before dispatch.
    assert!(runner.actions.lock().unwrap().is_empty());
    assert!(audit.rows().unwrap().is_empty());
}

#[tokio::test]
async fn observe_only_run_completes_without_approval() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = req(&dir);
    req.act_approved = false; // observe-only completes even unapproved
    let audit = AuditLog::create(req.audit_dir.clone()).unwrap();
    let runner = fake();
    let anthropic = ScriptedAnthropic {
        turns: Mutex::new(vec![vec![CuStepResponse::Stop {
            reason: StopReason::Success,
            text: Some("looked, nothing to change".into()),
            tokens_used: 2,
        }]]),
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
    .await
    .unwrap();

    match outcome {
        WindowsCuOutcome::Completed { summary, steps, .. } => {
            assert_eq!(summary, "looked, nothing to change");
            assert!(steps.is_empty());
        }
        WindowsCuOutcome::HumanHandoff { reason, .. } => {
            panic!("observe-only must complete, got HumanHandoff: {reason}")
        }
    }
    assert!(runner.actions.lock().unwrap().is_empty());
}

#[tokio::test]
async fn budget_exhaustion_hands_off() {
    let dir = tempfile::tempdir().unwrap();
    let req = req(&dir); // approved — so the ACT floor doesn't pre-empt the budget path
    let audit = AuditLog::create(req.audit_dir.clone()).unwrap();
    let runner = fake();
    // A single tool call whose token cost blows past `max_tokens_per_action`
    // (100) → the tracker reports Exhausted → HumanHandoff, action not run.
    let anthropic = ScriptedAnthropic {
        turns: Mutex::new(vec![vec![CuStepResponse::ToolCall {
            action: CuAction::Type {
                text: "x".into(),
            },
            tokens_used: 10_000,
        }]]),
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
    .await
    .unwrap();

    assert!(
        matches!(outcome, WindowsCuOutcome::HumanHandoff { .. }),
        "budget exhaustion must hand off"
    );
    assert!(
        runner.actions.lock().unwrap().is_empty(),
        "exhausted budget must not dispatch the action"
    );
}
