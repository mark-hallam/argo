use crate::executor_loop::run_loop_with_client;
use crate::{MacosCuConfig, MacosCuError, MacosCuExecutor, MacosCuOutcome, MacosCuRunRequest};
use argo_executor_computer_use::anthropic::{
    AnthropicCall, CuStepRequest, CuStepResponse, StopReason,
};
use argo_executor_computer_use::audit::AuditLog;
use argo_executor_computer_use::budget::BudgetTracker;
use argo_executor_computer_use::e2b::{ActionResult, CuAction};
use argo_executor_computer_use::profile::Profile;
use argo_executor_computer_use::restart::RestartCoordinator;
use argo_executor_computer_use::{CuError, CuStepRunner};
use argo_identity::WorkerId;
use argo_types::TenantId;
use async_trait::async_trait;
use std::sync::Mutex;
use std::time::Instant;

#[test]
fn executor_helpers_validate_and_package_outcomes() {
    let mut cfg = MacosCuConfig::for_notes_app();
    cfg.target_app = "com.apple.Terminal".to_string();
    let err = MacosCuExecutor::new_unguarded(cfg).unwrap_err();
    assert!(matches!(err, MacosCuError::Protocol(_)));

    let exec = MacosCuExecutor::new_unguarded(MacosCuConfig::for_notes_app()).unwrap();
    assert!(!exec.budget_exhausted(0, Instant::now()));

    match MacosCuExecutor::handoff("budget", vec![]) {
        MacosCuOutcome::HumanHandoff { reason, steps } => {
            assert_eq!(reason, "budget");
            assert!(steps.is_empty());
        }
        _ => panic!("expected HumanHandoff"),
    }

    match MacosCuExecutor::completed("ok", vec![], std::time::Duration::from_secs(42)) {
        MacosCuOutcome::Completed {
            wallclock_seconds, ..
        } => assert_eq!(wallclock_seconds, 42),
        _ => panic!("expected Completed"),
    }
}

#[test]
fn budget_exhausted_when_steps_reach_max() {
    let exec = MacosCuExecutor::new_unguarded(MacosCuConfig::for_notes_app()).unwrap();
    let started = Instant::now();
    assert!(exec.budget_exhausted(exec.config().max_steps, started));
    assert!(exec.budget_exhausted(exec.config().max_steps + 1, started));
    assert!(!exec.budget_exhausted(exec.config().max_steps - 1, started));
}

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

fn req(dir: &tempfile::TempDir) -> MacosCuRunRequest {
    MacosCuRunRequest {
        tenant_id: TenantId::new(),
        worker_id: WorkerId::new(),
        action_id: "act-mac".into(),
        system_prompt: "Type a short note, then stop_success.".into(),
        audit_dir: dir.path().join("bundle"),
        anthropic_api_key: "test-key".into(),
        // This helper models an APPROVED run (post "Approve & run"), so
        // the loop-mechanics tests exercise ACT execution. The risk-tiered
        // floor (held path) is covered by `act_op_held_when_not_approved`.
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

#[tokio::test]
async fn autonomous_loop_completes_with_mocked_steps() {
    let dir = tempfile::tempdir().unwrap();
    let req = req(&dir);
    let audit = AuditLog::create(req.audit_dir.clone()).unwrap();
    let runner = FakeRunner {
        actions: Mutex::new(Vec::new()),
    };
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
        crate::MacosCuOutcome::Completed { summary, steps, .. } => {
            assert_eq!(summary, "done");
            assert_eq!(steps.len(), 1);
            assert_eq!(steps[0].action_kind, "type");
        }
        crate::MacosCuOutcome::HumanHandoff { reason, .. } => {
            panic!("expected Completed, got HumanHandoff: {reason}");
        }
    }
    assert_eq!(runner.actions.lock().unwrap().len(), 1);
    assert_eq!(audit.rows().unwrap().len(), 1);
    assert!(req.audit_dir.join("step-000.png").exists());
    assert!(req.audit_dir.join("step-001.png").exists());
}

/// DEC-051 acceptance #2: a cu action that reaches an ACT op (here a
/// `click` — typing is autonomous now, the committing click is what is
/// gated) returns `HumanHandoff` (held) at the first ACT op when the run
/// is NOT approved — and the ACT is never executed against the host.
#[tokio::test]
async fn act_op_held_when_not_approved() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = req(&dir);
    req.act_approved = false; // unapproved run: the floor must hold
    let audit = AuditLog::create(req.audit_dir.clone()).unwrap();
    let runner = FakeRunner {
        actions: Mutex::new(Vec::new()),
    };
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
        MacosCuOutcome::HumanHandoff { reason, .. } => {
            assert!(
                reason.contains("ACT operation") && reason.contains("click"),
                "held reason should name the ACT op: {reason}"
            );
        }
        MacosCuOutcome::Completed { summary, .. } => {
            panic!("expected HumanHandoff (held), got Completed: {summary}");
        }
    }
    // The host action was NEVER executed — the floor held before dispatch.
    assert!(
        runner.actions.lock().unwrap().is_empty(),
        "ACT op must not execute while held"
    );
    // No step audit row for the held action (warm-up screenshot only).
    assert!(audit.rows().unwrap().is_empty());
}

/// DEC-051 acceptance #1: an OBSERVE-only run (the model issues no ACT
/// tool calls — screenshots are captured by the loop itself, not as
/// CuActions) runs to completion autonomously even when unapproved. No
/// approval handoff.
#[tokio::test]
async fn observe_only_run_completes_without_approval() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = req(&dir);
    req.act_approved = false; // unapproved — yet observe-only completes
    let audit = AuditLog::create(req.audit_dir.clone()).unwrap();
    let runner = FakeRunner {
        actions: Mutex::new(Vec::new()),
    };
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
        MacosCuOutcome::Completed { summary, steps, .. } => {
            assert_eq!(summary, "looked, nothing to change");
            assert!(steps.is_empty());
        }
        MacosCuOutcome::HumanHandoff { reason, .. } => {
            panic!("observe-only run must complete, got HumanHandoff: {reason}");
        }
    }
    assert!(runner.actions.lock().unwrap().is_empty());
}

/// DEC-051 operator tiers (2026-06-11): a STAGE-input op — `type` — runs
/// autonomously and DISPATCHES to the host even on an UNAPPROVED run.
/// Typing is autonomous because the operation that submits typed text (a
/// `key` press / click) stays in the gated ACT tier; entering text into a
/// field is reversible and commits nothing on its own. This is the direct
/// proof of the operator's call to put typing in the autonomous tier.
#[tokio::test]
async fn type_op_runs_autonomously_when_unapproved() {
    let dir = tempfile::tempdir().unwrap();
    let mut req = req(&dir);
    req.act_approved = false; // unapproved — yet `type` (staging) still runs
    let audit = AuditLog::create(req.audit_dir.clone()).unwrap();
    let runner = FakeRunner {
        actions: Mutex::new(Vec::new()),
    };
    let anthropic = ScriptedAnthropic {
        turns: Mutex::new(vec![
            vec![CuStepResponse::ToolCall {
                action: CuAction::Type {
                    text: "draft text".into(),
                },
                tokens_used: 4,
            }],
            vec![CuStepResponse::Stop {
                reason: StopReason::Success,
                text: Some("typed, awaiting submit".into()),
                tokens_used: 2,
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
        MacosCuOutcome::Completed { summary, steps, .. } => {
            assert_eq!(summary, "typed, awaiting submit");
            assert_eq!(steps.len(), 1);
            assert_eq!(steps[0].action_kind, "type");
        }
        MacosCuOutcome::HumanHandoff { reason, .. } => {
            panic!("`type` is autonomous — must NOT hold for approval: {reason}");
        }
    }
    // The staging op WAS dispatched to the host (autonomous), unlike an ACT op.
    assert_eq!(runner.actions.lock().unwrap().len(), 1);
}
