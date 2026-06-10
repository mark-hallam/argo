//! Unit + integration tests for the gateway-run-path inline executor
//! dispatcher (`cu2-wire-executor-dispatch-into-gateway-run-path`,
//! DEC-050 G2).
//!
//! Two seams are exercised without any API keys or env mutation (env
//! is process-global and races across the parallel test binary, so the
//! flag is passed explicitly to the pure gate and the executor is
//! injected as a mock into the `run_one` seam — same discipline
//! `worker_actions_executor_promotion` uses):
//!
//! * [`super::is_auto_eligible_with_flag`] — the static gate.
//! * [`super::run_one`] — transition + dispatch + settle, with a mock
//!   [`Executor`] standing in for the real (key-requiring)
//!   `computer_use` executor.
//!
//! Loaded via `#[cfg(test)] #[path = ...] mod tests;` from
//! `work_dispatch_executor_inline.rs`.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use crate::handlers::worker_actions_executor::{ActionOutcome, ExecutorKind};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, WorkspaceId};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const FIXTURE_VERTICAL: &str = "gtm-engineer";

/// Point `ARGO_VERTICALS_DIR` at the workspace's real verticals dir so
/// `find_action`'s `load_dir` walks `gtm-engineer` (the namespace the
/// seed rows live under). Mirrors `worker_actions_transitions_tests`.
fn set_verticals_dir() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .expect("workspace root");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root.join("verticals"));
}

async fn open_test_memory() -> (MemoryStore, WorkspaceId) {
    set_verticals_dir();
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let store = Arc::new(store);
    let (_tenant, workspace) = store.ensure_workspace("ws", "/tmp/ws").await.unwrap();
    (MemoryStore::new(store), workspace)
}

/// A low-risk autonomous-safe pending action whose declared executor is
/// `computer_use` — the acceptance-#1 shape.
fn autonomous_low_record(action_id: &str) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.to_string(),
        job_id: "job-1".to_string(),
        worker_id: argo_identity::WorkerId::nil(),
        item_id: None,
        action_type: "verify_price".into(),
        risk_level: "low".into(),
        requires_approval: false,
        state: "pending".to_string(),
        payload: serde_json::json!({}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: Vec::new(),
        blocked_reason: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

async fn seed(memory: &MemoryStore, ws: WorkspaceId, rec: &ActionRecord) {
    let body = serde_json::to_string(rec).unwrap();
    memory
        .append_for_vertical(ws, MemoryFile::Memory, body, None, FIXTURE_VERTICAL)
        .await
        .unwrap();
}

async fn current_state(memory: &MemoryStore, ws: WorkspaceId, action_id: &str) -> String {
    worker_actions_transitions::find_action(memory, ws, action_id)
        .await
        .unwrap()
        .expect("action present")
        .record
        .state
}

/// Mock executor standing in for the real `computer_use` one (which
/// needs E2B + Anthropic keys). The variant it returns is set per-test.
struct MockExecutor {
    outcome: MockOutcome,
}

enum MockOutcome {
    Completed,
    HumanHandoff,
    Error,
}

impl Executor for MockExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::ComputerUse
    }

    fn dispatch(&self, action: &ActionRecord, cwd: &Path) -> ArgoResult<ActionOutcome> {
        let bundle = cwd.join("mock").join(&action.action_id);
        match self.outcome {
            MockOutcome::Completed => Ok(ActionOutcome::Completed {
                bundle_path: bundle,
                result_json: serde_json::json!({ "ok": true, "by": "mock" }),
            }),
            MockOutcome::HumanHandoff => Ok(ActionOutcome::HumanHandoff { bundle_path: bundle }),
            MockOutcome::Error => Err(ArgoError::Provider("mock dispatch boom".into())),
        }
    }
}

// ──────────────────────── gate ─────────────────────────

#[test]
fn low_risk_autonomous_pending_is_eligible_only_with_flag() {
    let rec = autonomous_low_record("a");
    // Acceptance #2: auto-executes ONLY with ARGO_EXECUTION_GRADUATION on.
    assert!(is_auto_eligible_with_flag(&rec, true));
    assert!(!is_auto_eligible_with_flag(&rec, false));
}

#[test]
fn approval_required_is_held_even_with_flag() {
    // Acceptance #1: an approval-required action is NOT dispatched.
    let mut rec = autonomous_low_record("a");
    rec.requires_approval = true;
    rec.risk_level = "medium".into();
    assert!(!is_auto_eligible_with_flag(&rec, true));
}

#[test]
fn blocked_red_line_is_held() {
    // DEC-028 red lines emit a `blocked` row — never auto-run.
    let mut rec = autonomous_low_record("a");
    rec.state = "blocked".into();
    rec.blocked_reason = Some("red line".into());
    assert!(!is_auto_eligible_with_flag(&rec, true));
}

#[test]
fn high_risk_autonomous_is_held() {
    // Defense in depth: only the low tier auto-runs inline.
    let mut rec = autonomous_low_record("a");
    rec.risk_level = "high".into();
    assert!(!is_auto_eligible_with_flag(&rec, true));
}

// ──────────────────── dispatch seam ─────────────────────

#[tokio::test]
async fn completed_outcome_writes_completed_row() {
    // Acceptance #1: the inline dispatcher calls executor.dispatch()
    // and writes `completed`.
    let (memory, ws) = open_test_memory().await;
    let rec = autonomous_low_record("act-complete");
    seed(&memory, ws, &rec).await;
    let cwd = std::env::temp_dir();
    let exec = MockExecutor {
        outcome: MockOutcome::Completed,
    };

    run_one(&memory, ws, &rec, &exec, &cwd)
        .await
        .expect("run_one ok");

    assert_eq!(current_state(&memory, ws, "act-complete").await, "completed");
    // The executor's result_json is stamped onto the completed row.
    let located = worker_actions_transitions::find_action(&memory, ws, "act-complete")
        .await
        .unwrap()
        .unwrap();
    let result = located.record.result.expect("result present");
    assert_eq!(result.get("ok"), Some(&serde_json::json!(true)));
}

#[tokio::test]
async fn human_handoff_leaves_row_executing() {
    let (memory, ws) = open_test_memory().await;
    let rec = autonomous_low_record("act-handoff");
    seed(&memory, ws, &rec).await;
    let cwd = std::env::temp_dir();
    let exec = MockExecutor {
        outcome: MockOutcome::HumanHandoff,
    };

    run_one(&memory, ws, &rec, &exec, &cwd)
        .await
        .expect("run_one ok");

    assert_eq!(current_state(&memory, ws, "act-handoff").await, "executing");
}

#[tokio::test]
async fn dispatch_error_fails_the_row() {
    let (memory, ws) = open_test_memory().await;
    let rec = autonomous_low_record("act-fail");
    seed(&memory, ws, &rec).await;
    let cwd = std::env::temp_dir();
    let exec = MockExecutor {
        outcome: MockOutcome::Error,
    };

    let err = run_one(&memory, ws, &rec, &exec, &cwd).await;
    assert!(err.is_err(), "dispatch error propagates");
    // …but the ledger records the failure rather than silently dropping.
    assert_eq!(current_state(&memory, ws, "act-fail").await, "failed");
}
