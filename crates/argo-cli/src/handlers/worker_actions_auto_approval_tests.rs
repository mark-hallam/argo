//! Test entry point + shared helpers for the auto-approval gate.
//!
//! The actual scenario groups live in sibling files:
//! - [`unit`](worker_actions_auto_approval_tests_unit) — pure-fn / no-DB tests.
//! - [`persistence`](worker_actions_auto_approval_tests_persistence) — async
//!   persisted-spend tests.
//! - [`regression`](worker_actions_auto_approval_tests_regression) — F-06 P1
//!   `EffectivePolicy` regression suite.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use argo_identity::policy::{EffectivePolicy, PolicyApplies, PolicyFile, PolicyRule};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::WorkspaceId;
use argo_vertical::{ActionDeclaration, ActionRiskLevel, ExecutorDeclaration, ExecutorTagged};
use chrono::{TimeZone, Utc};
use std::path::PathBuf;
use std::sync::Arc;

const FIXTURE_VERTICAL: &str = "gtm-engineer";

fn rec(action_id: &str, risk: &str, requires_approval: bool) -> ActionRecord {
    ActionRecord {
        schema_key: ActionRecord::SCHEMA_KEY.to_string(),
        action_id: action_id.into(),
        job_id: "job-1".into(),
        worker_id: argo_identity::WorkerId::nil(),
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

fn empty_ctx() -> SessionContext {
    SessionContext::default()
}

fn ctx_with(tenant: &str, role: &str) -> SessionContext {
    SessionContext {
        tenant_str: tenant.into(),
        role_str: role.into(),
        worker_id_str: "worker-a".into(),
        roster_name: String::new(),
    }
}

fn effective_from(policy: AutoApprovalPolicy) -> EffectivePolicy {
    EffectivePolicy::from_base(policy)
}

fn policy_file(name: &str, applies: PolicyApplies, rule: PolicyRule) -> PolicyFile {
    PolicyFile {
        name: name.into(),
        applies_to: vec![applies],
        rules: vec![rule],
    }
}

#[path = "worker_actions_auto_approval_tests_unit.rs"]
mod unit;

#[path = "worker_actions_auto_approval_tests_persistence.rs"]
mod persistence;

#[path = "worker_actions_auto_approval_tests_regression.rs"]
mod regression;
