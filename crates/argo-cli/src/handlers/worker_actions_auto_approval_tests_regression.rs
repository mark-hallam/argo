//! F-06 P1 regression tests for `EffectivePolicy` resolution + the gate's
//! constraint-enforcement layer (`apply_effective_gates`).
//!
//! All five cases would have passed under the v0 empty-string call sites
//! because the new fields were ignored.

use super::*;
use argo_identity::policy::{PolicyApplies, PolicyQuietHours, PolicyRule, QuietHoursBehavior};

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
    // A roster-shaped policy file (selector = tenant) layers an audit-chain
    // requirement on every member; the gate sees it via
    // `EffectivePolicy::audit_chain_required`. Roster name matching today
    // flows through tenant selectors — proper roster-name selectors land
    // in a follow-up that extends PolicyApplies::Roster.
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
    // Same gate at 12:00 UTC (outside the window) leaves the decision
    // untouched.
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
