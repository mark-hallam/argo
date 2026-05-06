//! Unit tests for the declarative-policy data model + chain
//! resolver (Stream `claude-policy-as-code`). Sibling test file
//! so `policy.rs` stays under the per-file LOC budget.
use super::{
    glob_match, load_policy_dir, parse_policy_yaml, resolve_effective_policy, validate_policy_file,
    EffectivePolicy, PolicyApplies, PolicyError, PolicyFile, PolicyRule, QuietHours,
    QuietHoursBehavior,
};
use crate::AutoApprovalPolicy;

fn finance_strict_yaml() -> &'static str {
    "
applies_to:
  - !role finance-*
  - !tenant acme-*
rules:
  - action_kind: stripe_charge
    require_signatures: 2
    quiet_hours:
      start: '18:00'
      end: '09:00'
      behavior: block
    spend_cap_per_action_usd: 100
  - action_kind: gmail_send
    require_signatures: 1
  - action_kind: '*'
    audit_chain_required: true
"
}

#[test]
fn parse_policy_yaml_round_trips_full_payload() {
    let file =
        parse_policy_yaml("finance-strict", finance_strict_yaml()).expect("parse");
    assert_eq!(file.name, "finance-strict");
    assert_eq!(file.applies_to.len(), 2);
    assert_eq!(file.rules.len(), 3);
    assert_eq!(file.rules[0].action_kind, "stripe_charge");
    assert_eq!(file.rules[0].require_signatures, Some(2));
    assert_eq!(file.rules[0].spend_cap_per_action_usd, Some(100.0));
    let qh = file.rules[0].quiet_hours.as_ref().expect("quiet hours");
    assert_eq!(qh.start, "18:00");
    assert_eq!(qh.behavior, QuietHoursBehavior::Block);
}

#[test]
fn validate_rejects_empty_applies_to() {
    let file = PolicyFile {
        name: "bad".into(),
        applies_to: Vec::new(),
        rules: vec![PolicyRule {
            action_kind: "x".into(),
            ..Default::default()
        }],
    };
    let err = validate_policy_file(&file).expect_err("empty applies_to");
    match err {
        PolicyError::Validation { detail, .. } => assert!(detail.contains("applies_to")),
        other => panic!("wrong error: {other:?}"),
    }
}

#[test]
fn validate_rejects_empty_rules() {
    let file = PolicyFile {
        name: "bad".into(),
        applies_to: vec![PolicyApplies::Role("x".into())],
        rules: Vec::new(),
    };
    let err = validate_policy_file(&file).expect_err("empty rules");
    match err {
        PolicyError::Validation { detail, .. } => assert!(detail.contains("rules")),
        other => panic!("wrong error: {other:?}"),
    }
}

#[test]
fn validate_rejects_negative_spend_cap() {
    let file = PolicyFile {
        name: "bad".into(),
        applies_to: vec![PolicyApplies::Role("x".into())],
        rules: vec![PolicyRule {
            action_kind: "x".into(),
            spend_cap_per_action_usd: Some(-1.0),
            ..Default::default()
        }],
    };
    let err = validate_policy_file(&file).expect_err("negative cap");
    match err {
        PolicyError::Validation { detail, .. } => assert!(detail.contains("spend_cap")),
        other => panic!("wrong error: {other:?}"),
    }
}

#[test]
fn validate_rejects_bad_quiet_hours_time() {
    let file = PolicyFile {
        name: "bad".into(),
        applies_to: vec![PolicyApplies::Role("x".into())],
        rules: vec![PolicyRule {
            action_kind: "x".into(),
            quiet_hours: Some(QuietHours {
                start: "25:99".into(),
                end: "09:00".into(),
                behavior: QuietHoursBehavior::Block,
            }),
            ..Default::default()
        }],
    };
    let err = validate_policy_file(&file).expect_err("bad time");
    assert!(matches!(err, PolicyError::QuietHoursTimeParse { .. }));
}

#[test]
fn applies_to_role_matches_exact_and_prefix_glob() {
    assert!(PolicyApplies::Role("finance-*".into()).matches("finance-ar", "any-tenant"));
    assert!(PolicyApplies::Role("finance-*".into()).matches("finance-ap", "any-tenant"));
    assert!(!PolicyApplies::Role("finance-*".into()).matches("marketing", "any-tenant"));
    assert!(PolicyApplies::Role("finance-ar".into()).matches("finance-ar", "any-tenant"));
    assert!(!PolicyApplies::Role("finance-ar".into()).matches("finance-ap", "any-tenant"));
}

#[test]
fn applies_to_tenant_matches_glob() {
    assert!(PolicyApplies::Tenant("acme-*".into()).matches("any-role", "acme-prod"));
    assert!(!PolicyApplies::Tenant("acme-*".into()).matches("any-role", "globex"));
}

#[test]
fn glob_match_handles_empty_prefix_wildcard() {
    assert!(glob_match("*", "anything"));
    assert!(glob_match("*", ""));
    assert!(glob_match("foo", "foo"));
    assert!(!glob_match("foo", "bar"));
}

#[test]
fn resolve_returns_base_when_no_files_apply() {
    let base = AutoApprovalPolicy {
        enabled: true,
        max_usd_micros_per_action: Some(5_000_000),
        ..Default::default()
    };
    let files = vec![parse_policy_yaml("finance-strict", finance_strict_yaml()).expect("parse")];
    // Worker is in marketing, NOT finance-* — so no file applies.
    let effective =
        resolve_effective_policy(&base, &files, "marketing-intern", "globex", "stripe_charge");
    assert_eq!(effective.auto_approval, base);
    assert_eq!(effective.require_signatures, 1);
    assert!(effective.quiet_hours.is_none());
    assert!(!effective.audit_chain_required);
    assert!(effective.source_files.is_empty());
}

#[test]
fn resolve_action_specific_rule_wins_over_wildcard() {
    let base = AutoApprovalPolicy {
        enabled: true,
        ..Default::default()
    };
    let files = vec![parse_policy_yaml("finance-strict", finance_strict_yaml()).expect("parse")];
    let effective =
        resolve_effective_policy(&base, &files, "finance-ar", "acme-prod", "gmail_send");
    // The gmail_send rule says require_signatures: 1; the wildcard
    // rule sets audit_chain_required=true. The wildcard rule
    // doesn't fire because the action-specific gmail_send rule was
    // found first.
    assert_eq!(effective.require_signatures, 1);
    assert!(!effective.audit_chain_required);
}

#[test]
fn resolve_lowers_spend_cap_but_never_raises() {
    let base = AutoApprovalPolicy {
        enabled: true,
        max_usd_micros_per_action: Some(50_000_000_u64), // $50
        ..Default::default()
    };
    // Policy says cap = $100 — that's HIGHER than the worker's $50.
    // Resolver must keep $50 (never raise).
    let files = vec![parse_policy_yaml("finance-strict", finance_strict_yaml()).expect("parse")];
    let effective =
        resolve_effective_policy(&base, &files, "finance-ar", "acme-prod", "stripe_charge");
    assert_eq!(
        effective.auto_approval.max_usd_micros_per_action,
        Some(50_000_000_u64),
        "policy file must not raise the worker's existing cap"
    );

    // Now flip: worker cap is $200, policy cap is $100 → resolver
    // tightens to $100.
    let base = AutoApprovalPolicy {
        max_usd_micros_per_action: Some(200_000_000),
        ..Default::default()
    };
    let effective =
        resolve_effective_policy(&base, &files, "finance-ar", "acme-prod", "stripe_charge");
    assert_eq!(
        effective.auto_approval.max_usd_micros_per_action,
        Some(100_000_000_u64),
        "policy file must tighten to $100 when worker is $200"
    );
}

#[test]
fn resolve_escalates_signatures_to_max_across_files() {
    let base = AutoApprovalPolicy {
        enabled: true,
        ..Default::default()
    };
    let two_sig = "
applies_to: [!role any-*]
rules:
  - action_kind: stripe_charge
    require_signatures: 2
";
    let three_sig = "
applies_to: [!tenant acme-*]
rules:
  - action_kind: stripe_charge
    require_signatures: 3
";
    let files = vec![
        parse_policy_yaml("two-sig", two_sig).expect("parse"),
        parse_policy_yaml("three-sig", three_sig).expect("parse"),
    ];
    let effective = resolve_effective_policy(
        &base,
        &files,
        "any-finance",
        "acme-prod",
        "stripe_charge",
    );
    assert_eq!(effective.require_signatures, 3);
    assert_eq!(effective.source_files, vec!["two-sig", "three-sig"]);
}

#[test]
fn resolve_records_source_files_for_audit() {
    let base = AutoApprovalPolicy::default();
    let files = vec![parse_policy_yaml("finance-strict", finance_strict_yaml()).expect("parse")];
    let effective =
        resolve_effective_policy(&base, &files, "finance-ar", "acme-prod", "stripe_charge");
    assert_eq!(effective.source_files, vec!["finance-strict"]);
}

#[test]
fn resolve_quiet_hours_applies_when_rule_specifies_window() {
    let base = AutoApprovalPolicy::default();
    let files = vec![parse_policy_yaml("finance-strict", finance_strict_yaml()).expect("parse")];
    let effective =
        resolve_effective_policy(&base, &files, "finance-ar", "acme-prod", "stripe_charge");
    let qh = effective.quiet_hours.expect("quiet hours present");
    assert_eq!(qh.start, "18:00");
    assert_eq!(qh.end, "09:00");
}

#[test]
fn resolve_audit_chain_required_propagates_from_wildcard_rule() {
    // Use a tenant glob that matches the wildcard rule but no
    // action-specific rule, so the wildcard fires.
    let yaml = "
applies_to: [!tenant audit-*]
rules:
  - action_kind: '*'
    audit_chain_required: true
";
    let files = vec![parse_policy_yaml("audit-everything", yaml).expect("parse")];
    let base = AutoApprovalPolicy::default();
    let effective =
        resolve_effective_policy(&base, &files, "any-role", "audit-prod", "any_action");
    assert!(effective.audit_chain_required);
}

#[test]
fn from_base_constructs_unmodified_baseline() {
    let base = AutoApprovalPolicy {
        enabled: true,
        max_usd_micros_per_action: Some(1_000_000),
        ..Default::default()
    };
    let effective = EffectivePolicy::from_base(base.clone());
    assert_eq!(effective.auto_approval, base);
    assert_eq!(effective.require_signatures, 1);
    assert!(effective.quiet_hours.is_none());
    assert!(!effective.audit_chain_required);
    assert!(effective.source_files.is_empty());
}

#[test]
fn parse_returns_error_on_invalid_yaml() {
    let err = parse_policy_yaml("bad", "this: is: not: valid").expect_err("bad yaml");
    match err {
        PolicyError::Parse { name, .. } => assert_eq!(name, "bad"),
        other => panic!("wrong error: {other:?}"),
    }
}

#[test]
fn integration_full_chain_resolution_with_dir() {
    // Integration test the brief specifically calls for: load a
    // policy directory from disk, then resolve over it.
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("01-finance-strict.yaml"),
        finance_strict_yaml(),
    )
    .expect("write");
    std::fs::write(
        dir.path().join("02-default-audit.yaml"),
        "
applies_to: [!tenant '*']
rules:
  - action_kind: '*'
    audit_chain_required: true
",
    )
    .expect("write");
    // Non-yaml files are ignored.
    std::fs::write(dir.path().join("README.md"), "hi").expect("write");
    let files = load_policy_dir(dir.path()).expect("load");
    assert_eq!(files.len(), 2, "two yaml files loaded, README.md ignored");
    // Order is sorted by filename, which is the operator's
    // priority signal — `01-` first.
    assert_eq!(files[0].name, "01-finance-strict");
    assert_eq!(files[1].name, "02-default-audit");

    let base = AutoApprovalPolicy {
        enabled: true,
        max_usd_micros_per_action: Some(500_000_000), // $500 — laxer than policy
        ..Default::default()
    };
    let effective =
        resolve_effective_policy(&base, &files, "finance-ar", "acme-prod", "stripe_charge");
    assert_eq!(
        effective.auto_approval.max_usd_micros_per_action,
        Some(100_000_000),
        "finance-strict tightens cap to $100"
    );
    assert_eq!(effective.require_signatures, 2);
    assert!(effective.quiet_hours.is_some());
    // The default-audit file's wildcard rule didn't fire because
    // finance-strict's stripe_charge rule won first. To get
    // audit_chain_required=true here we'd need an action_kind
    // that finance-strict doesn't cover; let's verify with a
    // non-finance action.
    let other = resolve_effective_policy(
        &base,
        &files,
        "marketing-intern",
        "acme-prod",
        "tweet_post",
    );
    // marketing-intern doesn't match finance-* but tenant
    // acme-prod DOES match Tenant(acme-*), so finance-strict's
    // applies_to fires (OR semantics across applies_to entries).
    // Then its wildcard rule (action_kind '*') catches tweet_post
    // and sets audit_chain_required. The default-audit file also
    // matches and contributes the same flag — both source files
    // recorded.
    assert!(
        other.audit_chain_required,
        "wildcard rules catch non-finance actions"
    );
    assert_eq!(
        other.source_files,
        vec!["01-finance-strict", "02-default-audit"]
    );
}

#[test]
fn load_policy_dir_returns_empty_when_directory_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("does-not-exist");
    let files = load_policy_dir(&missing).expect("missing dir is fine");
    assert!(files.is_empty());
}

#[test]
fn load_policy_dir_returns_error_when_yaml_is_invalid() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("bad.yaml"), "this: is: not: valid").expect("write");
    let err = load_policy_dir(dir.path()).expect_err("invalid yaml");
    match err {
        PolicyError::Parse { name, .. } => assert_eq!(name, "bad"),
        other => panic!("wrong error: {other:?}"),
    }
}
