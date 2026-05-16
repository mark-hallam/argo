//! Unit tests for the auto-approval gate's cli-side glue.

use super::*;
use crate::handlers::worker_actions::ActionRecord;
use argo_identity::policy::{
    EffectivePolicy, PolicyApplies, PolicyFile, PolicyQuietHours, PolicyRule, QuietHoursBehavior,
};
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
    assert!(matches!(d, AutoApprovalDecision::Ask { .. }));
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

    let effective = effective_from(policy.clone());
    let first_decision = maybe_auto_approve_with_pool(
        &memory,
        ws,
        &first.action_id,
        &first,
        Some(&first_decl),
        &effective,
        &empty_ctx(),
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
        &effective,
        &empty_ctx(),
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
    let effective = effective_from(policy.clone());
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
        &effective,
        &empty_ctx(),
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
        &effective,
        &empty_ctx(),
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
    let effective = effective_from(policy.clone());
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
        &effective,
        &empty_ctx(),
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
    let effective = effective_from(policy.clone());
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
        &effective,
        &empty_ctx(),
        store.pool(),
        today,
    )
    .await
    .unwrap();
    assert!(decision.is_auto_approved());
}

// ───────────────── F-06 P1 regression tests ─────────────────
//
// Five cases prove `EffectivePolicy` resolution + the gate's
// new constraint enforcement layer
// (`apply_effective_gates`). All five would have passed under
// the v0 empty-string call sites because the new fields were
// ignored.

#[test]
fn non_wildcard_tenant_policy_resolves_when_tenant_matches() {
    let base = AutoApprovalPolicy {
        enabled: true,
        ..AutoApprovalPolicy::default()
    };
    let file = policy_file(
        "acme-strict",
        PolicyApplies::Tenant("acme-prod".into()),
        PolicyRule {
            action_kind: "*".into(),
            require_signatures: Some(2),
            ..PolicyRule::default()
        },
    );
    // Non-matching tenant — selector ignored, no signature bump.
    let other = argo_identity::policy::resolve_effective_policy(
        &base,
        std::slice::from_ref(&file),
        "",
        "ws-other",
        "test_action",
    );
    assert_eq!(other.require_signatures, 1);
    assert!(other.source_files.is_empty());
    // Matching tenant — selector applies, signatures escalate.
    let matched = argo_identity::policy::resolve_effective_policy(
        &base,
        std::slice::from_ref(&file),
        "",
        "acme-prod",
        "test_action",
    );
    assert_eq!(matched.require_signatures, 2);
    assert_eq!(matched.source_files, vec!["acme-strict".to_string()]);
}

#[test]
fn role_specific_policy_only_applies_when_role_matches() {
    let base = AutoApprovalPolicy {
        enabled: true,
        ..AutoApprovalPolicy::default()
    };
    let file = policy_file(
        "finance-dual",
        PolicyApplies::Role("finance-*".into()),
        PolicyRule {
            action_kind: "test_action".into(),
            require_signatures: Some(2),
            ..PolicyRule::default()
        },
    );
    // Glob `finance-*` matches `finance-ops` but not `support`.
    let support = argo_identity::policy::resolve_effective_policy(
        &base,
        std::slice::from_ref(&file),
        "support",
        "ws-anywhere",
        "test_action",
    );
    assert_eq!(support.require_signatures, 1);
    let finance = argo_identity::policy::resolve_effective_policy(
        &base,
        std::slice::from_ref(&file),
        "finance-ops",
        "ws-anywhere",
        "test_action",
    );
    assert_eq!(finance.require_signatures, 2);
}

#[test]
fn roster_inheritance_via_tenant_selector_resolves_audit_chain_flag() {
    // A roster-shaped policy file (selector = tenant) layers an
    // audit-chain requirement on every member; the gate sees it
    // via `EffectivePolicy::audit_chain_required`. Roster name
    // matching today flows through tenant selectors — proper
    // roster-name selectors land in a follow-up that extends
    // PolicyApplies::Roster.
    let base = AutoApprovalPolicy {
        enabled: true,
        ..AutoApprovalPolicy::default()
    };
    let file = policy_file(
        "compliance-default",
        PolicyApplies::Tenant("ws-fixture".into()),
        PolicyRule {
            action_kind: "*".into(),
            audit_chain_required: Some(true),
            ..PolicyRule::default()
        },
    );
    let effective = argo_identity::policy::resolve_effective_policy(
        &base, &[file], "", "ws-fixture", "send_email",
    );
    assert!(effective.audit_chain_required);
}

#[test]
fn quiet_hours_edge_window_active_downgrades_to_ask() {
    let base = AutoApprovalPolicy {
        enabled: true,
        ..AutoApprovalPolicy::default()
    };
    // 18:00 → 09:00 wraps midnight; 02:00 UTC sits inside.
    let file = policy_file(
        "afterhours",
        PolicyApplies::Tenant("*".into()),
        PolicyRule {
            action_kind: "*".into(),
            quiet_hours: Some(PolicyQuietHours {
                start: "18:00".into(),
                end: "09:00".into(),
                behavior: QuietHoursBehavior::QueueUntilMorning,
            }),
            ..PolicyRule::default()
        },
    );
    let effective = argo_identity::policy::resolve_effective_policy(
        &base,
        &[file],
        "",
        "any",
        "test_action",
    );
    let auto = AutoApprovalDecision::AutoApprove {
        reason: "policy:low_risk".into(),
    };
    let inside = at(6, 2); // 02:00 UTC — inside 18:00→09:00 wrap
    let downgraded =
        apply_effective_gates("act-qh-1", &effective, &empty_ctx(), auto.clone(), inside).unwrap();
    match downgraded {
        AutoApprovalDecision::Ask { reason, .. } => assert!(reason.contains("quiet_hours")),
        other => panic!("expected Ask, got {other:?}"),
    }
    // Same gate at 12:00 UTC (outside the window) leaves the
    // decision untouched.
    let outside = at(6, 12);
    let kept =
        apply_effective_gates("act-qh-2", &effective, &empty_ctx(), auto, outside).unwrap();
    assert!(kept.is_auto_approved());
}

#[test]
fn require_signatures_shortfall_downgrades_to_ask() {
    let mut effective = effective_from(AutoApprovalPolicy {
        enabled: true,
        ..AutoApprovalPolicy::default()
    });
    effective.require_signatures = 2;
    let auto = AutoApprovalDecision::AutoApprove {
        reason: "policy:low_risk".into(),
    };
    let downgraded = apply_effective_gates(
        "act-sig-1",
        &effective,
        &ctx_with("acme", "ops"),
        auto,
        at(6, 12),
    )
    .unwrap();
    match downgraded {
        AutoApprovalDecision::Ask { reason, .. } => {
            assert!(reason.contains("require_signatures=2"));
        }
        other => panic!("expected Ask, got {other:?}"),
    }
}
