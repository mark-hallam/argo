use crate::executor::run_loop_with_client;
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
use async_trait::async_trait;
use std::sync::Mutex;
use std::time::Instant;

#[test]
fn executor_helpers_validate_and_package_outcomes() {
    let mut cfg = MacosCuConfig::for_notes_app();
    cfg.target_app = "com.apple.Terminal".to_string();
    let err = MacosCuExecutor::new(cfg).unwrap_err();
    assert!(matches!(err, MacosCuError::Protocol(_)));

    let exec = MacosCuExecutor::new(MacosCuConfig::for_notes_app()).unwrap();
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
    let exec = MacosCuExecutor::new(MacosCuConfig::for_notes_app()).unwrap();
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
        action_id: "act-mac".into(),
        system_prompt: "Type a short note, then stop_success.".into(),
        audit_dir: dir.path().join("bundle"),
        anthropic_api_key: "test-key".into(),
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
