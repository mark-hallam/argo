//! Dangerous-capability fence tests for the hire gate (the synonym-blindness
//! fix, layer 3). Split out of `hire_gate_tests.rs` to keep that file under
//! the per-file LOC budget.
//!
//! Proves the gate refuses a contract that presents a money / destructive /
//! external-egress / credential action as AUTONOMOUS, AND that the five
//! dangerous role descriptions the stress-test found now gate their dangerous
//! actions through the REAL synthesis seam — while a benign support/CS role
//! still passes cleanly (the false-positive guard).

use super::*;
use crate::comprehension::ComprehensionOutcome;
use crate::materialize::materialize_spec;
use crate::role_contract::RoleContract;
use crate::spec::CapabilitySpec;
use crate::sop::SopComprehensionOutcome;
use crate::{Archetype, BroadRoleDraft};

/// A clean comprehension pass (all 5 questions answered) — every gate axis
/// except the one under test is green.
fn comp_pass() -> ComprehensionOutcome {
    ComprehensionOutcome::from_counts(5, 5)
}

/// A manifest carrying one autonomous (un-gated) dangerous action plus a
/// benign one — the false-negative the deterministic floor is meant to
/// prevent, used to prove the gate refuses such a contract.
fn under_gated_dangerous_manifest() -> argo_vertical::VerticalManifest {
    use argo_vertical::{
        ActionDeclaration, ActionRiskLevel, ActionsSection, ExecutorDeclaration, VerticalManifest,
    };
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, ActionDeclaration> = BTreeMap::new();
    for (name, ra) in [("draft_summary", false), ("transfer_funds", false)] {
        map.insert(
            name.to_string(),
            ActionDeclaration {
                risk: ActionRiskLevel::Low,
                requires_approval: ra,
                description: format!("Do {name}"),
                executor: ExecutorDeclaration::default(),
                ..ActionDeclaration::default()
            },
        );
    }
    VerticalManifest {
        actions: Some(ActionsSection { actions: map }),
        ..VerticalManifest::default()
    }
}

#[test]
fn compose_blocks_when_a_dangerous_action_is_autonomous_even_if_confirmed() {
    // The load-bearing layer-3 guarantee: a money/destructive/egress/
    // credential action that is NOT approval-gated makes the gate hard-block,
    // independent of every other axis — even a fully CONFIRMED contract with
    // all evals + comprehension green + judge allow. A contract that
    // under-gates a dangerous action must never reach Pass.
    let m = under_gated_dangerous_manifest();
    let contract = RoleContract::derive_from_manifest(&m);
    let digest = contract.digest();
    let input = ContractGateInput::resolve(Some(contract), Some(&digest));
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Allow,
        input,
    );
    assert_eq!(
        v.state,
        GateState::Blocked,
        "an autonomous dangerous action must hard-block the gate"
    );
    assert!(!v.state.allows_hire());
    assert!(
        v.reasons.iter().any(|r| r.contains("capability fence violated")
            && r.contains("transfer_funds")),
        "expected the fence-violation reason naming transfer_funds; got: {:?}",
        v.reasons
    );
}

#[test]
fn compose_passes_a_correctly_gated_dangerous_role() {
    // Control: the SAME dangerous action, correctly approval-gated, passes —
    // proving the fence flags only the under-gated case, not the danger itself.
    use argo_vertical::{
        ActionDeclaration, ActionRiskLevel, ActionsSection, EscalationSection, ExecutorDeclaration,
        VerticalManifest,
    };
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, ActionDeclaration> = BTreeMap::new();
    map.insert(
        "draft_summary".to_string(),
        ActionDeclaration {
            risk: ActionRiskLevel::Low,
            requires_approval: false,
            description: "Draft a summary".to_string(),
            executor: ExecutorDeclaration::default(),
            ..ActionDeclaration::default()
        },
    );
    map.insert(
        "transfer_funds".to_string(),
        ActionDeclaration {
            risk: ActionRiskLevel::High,
            requires_approval: true,
            description: "Transfer funds".to_string(),
            executor: ExecutorDeclaration::default(),
            ..ActionDeclaration::default()
        },
    );
    let m = VerticalManifest {
        actions: Some(ActionsSection { actions: map }),
        escalation: Some(EscalationSection {
            human_approval_required: vec!["transfer_funds".to_string()],
            escalate_after_failures: Some(3),
            escalate_to: Some("the operator".to_string()),
        }),
        ..VerticalManifest::default()
    };
    let contract = RoleContract::derive_from_manifest(&m);
    let digest = contract.digest();
    let input = ContractGateInput::resolve(Some(contract), Some(&digest));
    let v = GateVerdict::compose(
        EvalFloorOutcome::from_counts(10, 10),
        comp_pass(),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Allow,
        input,
    );
    assert_eq!(
        v.state,
        GateState::Pass,
        "a correctly-gated dangerous role must still pass; got reasons: {:?}",
        v.reasons
    );
}

/// Materialize a from-scratch teammate from a role description + action
/// vocabulary through the SAME `BroadRoleDraft → CapabilitySpec →
/// materialize_spec` seam the live `POST /api/v1/genesis` page uses, then
/// derive its truthful role contract. This is the exact path the stress-test
/// exercised.
fn broad_role_contract(charter: &str, persona: &str, actions: &[&str]) -> RoleContract {
    let draft = BroadRoleDraft {
        charter: charter.into(),
        persona: persona.into(),
        persona_description: format!("{charter}."),
        action_vocabulary: actions.iter().map(|s| (*s).to_string()).collect(),
        success_metrics: vec!["operates within policy".into()],
        onboarding_script: "Hi, here to help.".into(),
    };
    let spec = CapabilitySpec::from_broad_draft(draft, Archetype::ResearchAnalyst);
    let plan = materialize_spec(&spec).expect("dangerous-role teammate materializes");
    RoleContract::derive_from_manifest(&plan.manifest)
}

#[test]
fn live_synthesis_gates_the_five_stress_test_dangerous_roles() {
    // RE-RUN EVIDENCE for the synonym-blindness fix. Each of the five
    // dangerous role descriptions the stress-test found surfacing AUTONOMOUS /
    // no-approval / low-risk must now, through the real synthesis seam, surface
    // its dangerous action(s) as REQUIRES_APPROVAL + high-risk, land in the
    // approval-gated "will NOT do" contract, and produce ZERO
    // dangerous-autonomous flags (the gate would otherwise hard-block).
    let cases: &[(&str, &str, &[&str])] = &[
        ("an accounts-payable assistant that pays vendor bills", "ap", &["process_payment", "review_invoice"]),
        ("a treasury assistant that moves money between accounts", "treasury", &["transfer_funds", "reconcile_ledger"]),
        ("a data ops assistant that ships customer data to partners", "dataops", &["extract_customer_records", "upload_to_partner_system"]),
        ("a portal assistant that logs into the bank with stored creds", "portal", &["authenticate_portal", "fetch_statement"]),
        ("a storage assistant that purges archived data", "storage", &["empty_archived_storage", "list_buckets"]),
    ];
    let dangerous_per_case = [
        "process_payment",
        "transfer_funds",
        "extract_customer_records",
        "authenticate_portal",
        "empty_archived_storage",
    ];
    for ((charter, persona, actions), dangerous) in cases.iter().zip(dangerous_per_case) {
        let c = broad_role_contract(charter, persona, actions);
        // The dangerous action is approval-gated (NOT autonomous).
        assert!(
            c.approval_required.iter().any(|a| a == dangerous),
            "[{persona}] {dangerous} must be approval-gated; approval_required={:?}",
            c.approval_required
        );
        assert!(
            !c.autonomous.iter().any(|a| a == dangerous),
            "[{persona}] {dangerous} must NOT be autonomous; autonomous={:?}",
            c.autonomous
        );
        // It is high-risk in the bound action list.
        let decl = c
            .actions
            .iter()
            .find(|a| a.action == *dangerous)
            .unwrap_or_else(|| panic!("[{persona}] {dangerous} missing from contract actions"));
        assert!(decl.requires_approval, "[{persona}] {dangerous} must require approval");
        assert_eq!(decl.risk, "high", "[{persona}] {dangerous} must be high-risk");
        // It is named in the operator-facing "what it will NOT do" contract.
        assert!(
            c.will_not.iter().any(|w| w.contains(dangerous)),
            "[{persona}] will_not must name {dangerous}; will_not={:?}",
            c.will_not
        );
        // The gate's dangerous-autonomous fence is satisfied (nothing un-gated).
        assert!(
            c.dangerous_autonomous_actions().is_empty(),
            "[{persona}] no dangerous action may be autonomous; got {:?}",
            c.dangerous_autonomous_actions()
        );
    }
}

#[test]
fn live_synthesis_control_support_role_passes_without_over_flagging() {
    // CONTROL (must NOT be theater): a legit customer-success / support role
    // synthesizes cleanly — its benign actions stay AUTONOMOUS, none are
    // wrongly approval-gated, and the dangerous-autonomous fence is empty. This
    // is the false-positive guard the brief requires alongside the fix.
    let c = broad_role_contract(
        "a customer success teammate who triages tickets and drafts replies",
        "casey",
        &["triage_ticket", "draft_response", "summarize_thread", "tag_account"],
    );
    for benign in ["triage_ticket", "draft_response", "summarize_thread", "tag_account"] {
        assert!(
            c.autonomous.iter().any(|a| a == benign),
            "{benign} should be autonomous for a support role; autonomous={:?}",
            c.autonomous
        );
        assert!(
            !c.approval_required.iter().any(|a| a == benign),
            "{benign} must NOT be wrongly approval-gated; approval_required={:?}",
            c.approval_required
        );
    }
    assert!(
        c.dangerous_autonomous_actions().is_empty(),
        "a benign support role must trip no dangerous flag: {:?}",
        c.dangerous_autonomous_actions()
    );
}
