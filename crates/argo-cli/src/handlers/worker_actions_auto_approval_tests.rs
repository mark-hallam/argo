//! Unit tests for the auto-approval gate's cli-side glue.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use argo_identity::worker_budget;
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::WorkspaceId;
use argo_vertical::{ActionDeclaration, ActionRiskLevel, ExecutorDeclaration, ExecutorTagged};
use chrono::{TimeZone, Utc};
use std::path::PathBuf;
use std::sync::Arc;

fn rec(action_id: &str, risk: &str, requires_approval: bool) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.into(),
        job_id: "job-1".into(),
        item_id: None,
        action_type: "test_action".into(),
        risk_level: risk.into(),
        requires_approval,
        state: "pending".into(),
        payload: serde_json::json!({}),
        approved_by: None,
        approved_at: None,
        executed_at: None,
        platform: None,
        result: None,
        evidence: vec![],
        blocked_reason: None,
        created_at: "2026-05-06T12:00:00Z".into(),
    }
}

const FIXTURE_VERTICAL: &str = "gtm-engineer";

fn set_verticals_dir() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap();
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root.join("verticals"));
}

async fn open_test_memory() -> (Arc<SessionStore>, MemoryStore, WorkspaceId) {
    set_verticals_dir();
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let (_tenant, workspace) = store.ensure_workspace("ws", "/tmp/ws").await.unwrap();
    (Arc::clone(&store), MemoryStore::new(store), workspace)
}

async fn seed(memory: &MemoryStore, ws: WorkspaceId, record: &ActionRecord) {
    let body = serde_json::to_string(record).unwrap();
    memory
        .append_for_vertical(ws, MemoryFile::Memory, body, None, FIXTURE_VERTICAL)
        .await
        .unwrap();
}

fn rec_for_worker(action_id: &str, worker_id: &str) -> ActionRecord {
    let mut r = rec(action_id, "low", true);
    r.payload = serde_json::json!({ "worker_id": worker_id });
    r
}

fn cost_decl(cost: Option<f64>) -> ActionDeclaration {
    let executor = ExecutorTagged {
        kind: "computer_use".into(),
        budget_usd: cost,
        ..ExecutorTagged::default()
    };
    ActionDeclaration {
        risk: ActionRiskLevel::Low,
        requires_approval: true,
        executor: ExecutorDeclaration::Tagged(executor),
        ..ActionDeclaration::default()
    }
}

fn enabled_policy(daily_cap: u64) -> AutoApprovalPolicy {
    AutoApprovalPolicy {
        enabled: true,
        daily_budget_usd_micros: Some(daily_cap),
        ..AutoApprovalPolicy::default()
    }
}

fn at(day: u32, hour: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, day, hour, 0, 0).unwrap()
}

#[test]
fn load_workspace_policy_returns_default_when_file_missing() {
    let cwd = tempfile::tempdir().unwrap();
    let policy = load_workspace_policy(cwd.path()).unwrap();
    assert!(!policy.enabled);
}

#[test]
fn load_workspace_policy_parses_basic_toml() {
    let cwd = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(cwd.path().join(".skycot")).unwrap();
    std::fs::write(
        cwd.path().join(POLICY_RELPATH),
        r#"
enabled = true
max_usd_micros_per_action = 500000
daily_budget_usd_micros = 50000000
wake_action_types = ["page_oncall"]
"#,
    )
    .unwrap();
    let policy = load_workspace_policy(cwd.path()).unwrap();
    assert!(policy.enabled);
    assert_eq!(policy.max_usd_micros_per_action, Some(500_000));
    assert_eq!(policy.daily_budget_usd_micros, Some(50_000_000));
    assert_eq!(policy.wake_action_types, vec!["page_oncall".to_string()]);
}

#[test]
fn load_workspace_policy_errors_on_invalid_toml() {
    let cwd = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(cwd.path().join(".skycot")).unwrap();
    std::fs::write(cwd.path().join(POLICY_RELPATH), "garbage = ::: invalid").unwrap();
    let err = load_workspace_policy(cwd.path()).expect_err("invalid toml");
    assert!(err.to_string().contains("not valid TOML"));
}

#[test]
fn decide_returns_auto_approve_for_autonomous_action() {
    let policy = AutoApprovalPolicy::default();
    let r = rec("act-1", "low", false);
    let d = decide(&policy, &r, None, SpendSnapshot::zero());
    assert!(d.is_auto_approved());
}

#[test]
fn decide_with_disabled_policy_asks_for_approval_gated_action() {
    let policy = AutoApprovalPolicy::default();
    let r = rec("act-2", "low", true);
    let d = decide(&policy, &r, None, SpendSnapshot::zero());
    assert!(matches!(d, Decision::Ask { .. }));
}

#[test]
fn decide_with_enabled_policy_auto_approves_low_risk_within_caps() {
    let policy = AutoApprovalPolicy {
        enabled: true,
        ..AutoApprovalPolicy::default()
    };
    let r = rec("act-3", "low", true);
    let d = decide(&policy, &r, None, SpendSnapshot::zero());
    assert!(d.is_auto_approved());
}

#[test]
fn auto_approver_token_carries_policy_reason() {
    let token = auto_approver_token("policy:risk=low_within_caps");
    assert_eq!(token, "auto:policy:risk=low_within_caps");
    assert!(token.starts_with("auto:"));
}

#[test]
fn block_to_error_includes_action_id_and_reason() {
    let err = block_to_error("act-7", "policy:daily_cap exceeded");
    let msg = err.to_string();
    assert!(msg.contains("act-7"));
    assert!(msg.contains("daily_cap"));
    assert!(msg.contains("blocked by auto-approval policy"));
}

#[tokio::test]
async fn persisted_spend_blocks_second_action_over_daily_cap() {
    let (store, memory, ws) = open_test_memory().await;
    let policy = enabled_policy(1_000_000);
    let first = rec_for_worker("act-cap-1", "worker-a");
    seed(&memory, ws, &first).await;
    let first_decl = cost_decl(Some(0.70));
    let now = at(6, 12);

    let first_decision = maybe_auto_approve_with_pool(
        &memory,
        ws,
        &first.action_id,
        &first,
        Some(&first_decl),
        &policy,
        store.pool(),
        now,
    )
    .await
    .unwrap();
    assert!(first_decision.is_auto_approved());
    assert_eq!(
        worker_budget::spend_today_for_worker(store.pool(), "worker-a", now)
            .await
            .unwrap(),
        700_000
    );

    let second = rec_for_worker("act-cap-2", "worker-a");
    seed(&memory, ws, &second).await;
    let second_decl = cost_decl(Some(0.40));
    let err = maybe_auto_approve_with_pool(
        &memory,
        ws,
        &second.action_id,
        &second,
        Some(&second_decl),
        &policy,
        store.pool(),
        now,
    )
    .await
    .expect_err("second action should cross daily cap");
    assert!(err.to_string().contains("daily_cap"));
}

#[tokio::test]
async fn exactly_at_cap_still_allows_zero_cost_action() {
    let (store, memory, ws) = open_test_memory().await;
    let policy = enabled_policy(1_000_000);
    let now = at(6, 12);
    let paid = rec_for_worker("act-exact-1", "worker-a");
    seed(&memory, ws, &paid).await;
    let paid_decl = cost_decl(Some(1.0));
    maybe_auto_approve_with_pool(
        &memory,
        ws,
        &paid.action_id,
        &paid,
        Some(&paid_decl),
        &policy,
        store.pool(),
        now,
    )
    .await
    .unwrap();

    let free = rec_for_worker("act-exact-2", "worker-a");
    seed(&memory, ws, &free).await;
    let free_decl = cost_decl(None);
    let decision = maybe_auto_approve_with_pool(
        &memory,
        ws,
        &free.action_id,
        &free,
        Some(&free_decl),
        &policy,
        store.pool(),
        now,
    )
    .await
    .unwrap();
    assert!(decision.is_auto_approved());
}

#[tokio::test]
async fn persisted_spend_is_isolated_by_worker() {
    let (store, memory, ws) = open_test_memory().await;
    let policy = enabled_policy(1_000_000);
    let now = at(6, 12);
    worker_budget::record_charge(store.pool(), "seed-a", "worker-a", 1_000_000, now)
        .await
        .unwrap();

    let action = rec_for_worker("act-worker-b", "worker-b");
    seed(&memory, ws, &action).await;
    let decl = cost_decl(Some(1.0));
    let decision = maybe_auto_approve_with_pool(
        &memory,
        ws,
        &action.action_id,
        &action,
        Some(&decl),
        &policy,
        store.pool(),
        now,
    )
    .await
    .unwrap();
    assert!(decision.is_auto_approved());
    assert_eq!(
        worker_budget::spend_today_for_worker(store.pool(), "worker-b", now)
            .await
            .unwrap(),
        1_000_000
    );
}

#[tokio::test]
async fn yesterday_spend_does_not_count_toward_today() {
    let (store, memory, ws) = open_test_memory().await;
    let policy = enabled_policy(1_000_000);
    let today = at(6, 12);
    worker_budget::record_charge(
        store.pool(),
        "seed-yesterday",
        "worker-a",
        1_000_000,
        at(5, 12),
    )
    .await
    .unwrap();
    assert_eq!(
        worker_budget::spend_today_for_worker(store.pool(), "worker-a", today)
            .await
            .unwrap(),
        0
    );

    let action = rec_for_worker("act-today", "worker-a");
    seed(&memory, ws, &action).await;
    let decl = cost_decl(Some(1.0));
    let decision = maybe_auto_approve_with_pool(
        &memory,
        ws,
        &action.action_id,
        &action,
        Some(&decl),
        &policy,
        store.pool(),
        today,
    )
    .await
    .unwrap();
    assert!(decision.is_auto_approved());
}
