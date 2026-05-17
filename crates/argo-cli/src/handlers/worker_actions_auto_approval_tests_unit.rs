//! Pure-function / no-DB tests for the auto-approval gate.

use super::*;

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
