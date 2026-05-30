//! Tests for the operator-confirmable role contract (Genesis P4).

use super::*;
use crate::pipeline::plan;
use argo_vertical::{
    ActionDeclaration, ActionRiskLevel, ActionsSection, EscalationSection, ExecutorDeclaration,
    ExecutorTagged, McpServerDeclaration, McpServersSection, VerticalManifest,
};
use std::collections::BTreeMap;

/// Build a minimal manifest with a known action set for precise assertions.
fn manifest_with(
    actions: &[(&str, ActionRiskLevel, bool, ExecutorDeclaration)],
    mcp_servers: &[&str],
    escalation: Option<EscalationSection>,
) -> VerticalManifest {
    let mut map: BTreeMap<String, ActionDeclaration> = BTreeMap::new();
    for (name, risk, requires_approval, executor) in actions {
        map.insert(
            (*name).to_string(),
            ActionDeclaration {
                risk: *risk,
                requires_approval: *requires_approval,
                description: format!("Do {name}"),
                executor: executor.clone(),
                ..ActionDeclaration::default()
            },
        );
    }
    let mcp = if mcp_servers.is_empty() {
        None
    } else {
        let mut servers: BTreeMap<String, McpServerDeclaration> = BTreeMap::new();
        for s in mcp_servers {
            servers.insert((*s).to_string(), McpServerDeclaration::default());
        }
        Some(McpServersSection { servers })
    };
    VerticalManifest {
        actions: Some(ActionsSection { actions: map }),
        mcp_servers: mcp,
        escalation,
        ..VerticalManifest::default()
    }
}

#[test]
fn derive_segregates_approval_gated_from_autonomous() {
    let m = manifest_with(
        &[
            ("draft_email", ActionRiskLevel::Medium, false, ExecutorDeclaration::default()),
            ("send_email", ActionRiskLevel::High, true, ExecutorDeclaration::default()),
            ("research", ActionRiskLevel::Low, false, ExecutorDeclaration::default()),
        ],
        &[],
        None,
    );
    let c = RoleContract::derive_from_manifest(&m);
    assert_eq!(c.approval_required, vec!["send_email".to_string()]);
    assert_eq!(
        c.autonomous,
        vec!["draft_email".to_string(), "research".to_string()]
    );
    // Every bound action appears exactly once across the two sets.
    assert_eq!(c.actions.len(), 3);
    assert_eq!(c.approval_required.len() + c.autonomous.len(), c.actions.len());
}

#[test]
fn approval_set_is_the_union_of_action_flag_and_escalation_list() {
    // `notify` is flagged via the escalation section ONLY (not its action
    // flag); the contract must still gate it — proving the union, so a
    // contract can't understate what needs approval.
    let m = manifest_with(
        &[
            ("notify", ActionRiskLevel::Low, false, ExecutorDeclaration::default()),
            ("send", ActionRiskLevel::High, true, ExecutorDeclaration::default()),
        ],
        &[],
        Some(EscalationSection {
            human_approval_required: vec!["notify".to_string()],
            escalate_after_failures: Some(3),
            escalate_to: Some("the operator".to_string()),
        }),
    );
    let c = RoleContract::derive_from_manifest(&m);
    assert_eq!(
        c.approval_required,
        vec!["notify".to_string(), "send".to_string()]
    );
    assert!(c.autonomous.is_empty(), "both actions are gated");
}

#[test]
fn draft_integrations_surface_but_actions_stay_human_at_birth() {
    let mut tagged = ExecutorTagged {
        kind: "human".to_string(),
        ..Default::default()
    };
    tagged.server = Some("gmail".to_string());
    let m = manifest_with(
        &[(
            "send_email",
            ActionRiskLevel::High,
            true,
            ExecutorDeclaration::Tagged(tagged),
        )],
        &["gmail", "slack"],
        None,
    );
    let c = RoleContract::derive_from_manifest(&m);
    assert_eq!(
        c.draft_integrations,
        vec!["gmail".to_string(), "slack".to_string()]
    );
    // The will-not list names the draft integrations as un-transmittable.
    assert!(
        c.will_not.iter().any(|w| w.contains("gmail") && w.contains("draft-only")),
        "will_not must flag draft integrations: {:?}",
        c.will_not
    );
}

#[test]
fn will_not_names_the_approval_gated_actions_and_the_no_integration_case() {
    let m = manifest_with(
        &[
            ("draft", ActionRiskLevel::Low, false, ExecutorDeclaration::default()),
            ("send", ActionRiskLevel::High, true, ExecutorDeclaration::default()),
        ],
        &[],
        None,
    );
    let c = RoleContract::derive_from_manifest(&m);
    assert!(
        c.will_not.iter().any(|w| w.contains("send")),
        "will_not must name the approval-gated action: {:?}",
        c.will_not
    );
    assert!(
        c.will_not.iter().any(|w| w.contains("none bound")),
        "no-integration case must state it cannot transmit: {:?}",
        c.will_not
    );
}

#[test]
fn escalation_clause_reflects_the_manifest_routing() {
    let m = manifest_with(
        &[("act", ActionRiskLevel::Low, false, ExecutorDeclaration::default())],
        &[],
        Some(EscalationSection {
            human_approval_required: vec![],
            escalate_after_failures: Some(5),
            escalate_to: Some("the on-call lead".to_string()),
        }),
    );
    let c = RoleContract::derive_from_manifest(&m);
    assert!(c.escalation.contains("the on-call lead"));
    assert!(c.escalation.contains('5'));
}

#[test]
fn verify_contract_matches_truthful_and_rejects_forgeries() {
    let m = manifest_with(
        &[
            ("draft", ActionRiskLevel::Low, false, ExecutorDeclaration::default()),
            ("wire_funds", ActionRiskLevel::High, true, ExecutorDeclaration::default()),
        ],
        &["stripe"],
        None,
    );
    let truthful = RoleContract::derive_from_manifest(&m);
    assert!(verify_contract_matches(&truthful, &m));

    // Forgery 1: hide the approval-gated action.
    let mut hide_approval = truthful.clone();
    hide_approval.approval_required.clear();
    assert!(
        !verify_contract_matches(&hide_approval, &m),
        "a contract that hides wire_funds approval must be rejected"
    );

    // Forgery 2: claim an action is autonomous when the manifest gates it.
    let mut lie_autonomous = truthful.clone();
    lie_autonomous.autonomous.push("wire_funds".to_string());
    assert!(!verify_contract_matches(&lie_autonomous, &m));

    // Forgery 3: drop a draft integration so the operator can't see it.
    let mut hide_integration = truthful.clone();
    hide_integration.draft_integrations.clear();
    assert!(!verify_contract_matches(&hide_integration, &m));
}

#[test]
fn digest_is_deterministic_and_changes_when_capabilities_change() {
    let benign = manifest_with(
        &[("draft", ActionRiskLevel::Low, false, ExecutorDeclaration::default())],
        &[],
        None,
    );
    let broad = manifest_with(
        &[
            ("draft", ActionRiskLevel::Low, false, ExecutorDeclaration::default()),
            ("wire_funds", ActionRiskLevel::High, true, ExecutorDeclaration::default()),
        ],
        &["stripe"],
        None,
    );
    let d_benign = RoleContract::derive_from_manifest(&benign).digest();
    let d_broad = RoleContract::derive_from_manifest(&broad).digest();
    assert_eq!(d_benign.len(), 64);
    assert_eq!(
        d_benign,
        RoleContract::derive_from_manifest(&benign).digest(),
        "digest is deterministic"
    );
    assert_ne!(
        d_benign, d_broad,
        "a teammate with broader capability must have a different digest — \
         a stale confirmation of the benign contract cannot activate the broad one"
    );
}

#[test]
fn digest_is_stable_across_a_cosmetic_description_reword() {
    // The digest covers the governance projection, not prose, so re-wording an
    // action's description does not invalidate an operator confirmation while
    // any capability change does.
    let mut a = manifest_with(
        &[("send", ActionRiskLevel::High, true, ExecutorDeclaration::default())],
        &[],
        None,
    );
    let d1 = RoleContract::derive_from_manifest(&a).digest();
    if let Some(section) = a.actions.as_mut() {
        if let Some(decl) = section.actions.get_mut("send") {
            decl.description = "A totally reworded description".to_string();
        }
    }
    let d2 = RoleContract::derive_from_manifest(&a).digest();
    assert_eq!(d1, d2, "a description reword must not invalidate confirmation");
}

#[test]
fn real_archetype_plan_produces_a_truthful_self_consistent_contract() {
    let p = plan("be my marketing intern, run weekly campaigns to cold leads", "maya")
        .expect("plan builds");
    let c = RoleContract::derive_from_manifest(&p.manifest);
    // Self-consistency: the contract re-derived from the same manifest verifies.
    assert!(verify_contract_matches(&c, &p.manifest));
    // The marketing intern's sensitive verbs (post/send) are approval-gated and
    // MUST appear in the approval set, never in the autonomous set.
    assert!(
        c.approval_required.iter().any(|a| a == "post_campaign"),
        "post_campaign must be approval-gated: {:?}",
        c.approval_required
    );
    assert!(!c.autonomous.iter().any(|a| a == "post_campaign"));
    assert!(!c.actions.is_empty());
    assert_eq!(c.role_name, p.manifest.vertical.name);
}

#[test]
fn round_trips_through_serde_json() {
    let p = plan("be my marketing intern, run weekly campaigns", "maya").expect("plan builds");
    let c = RoleContract::derive_from_manifest(&p.manifest);
    let json = serde_json::to_string(&c).expect("serialize");
    let back: RoleContract = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(c, back);
    assert_eq!(c.digest(), back.digest());
}

#[test]
fn empty_manifest_yields_an_empty_but_honest_contract() {
    let m = VerticalManifest::default();
    let c = RoleContract::derive_from_manifest(&m);
    assert!(c.actions.is_empty());
    assert!(c.approval_required.is_empty());
    assert!(c.autonomous.is_empty());
    // Even with no actions, the will-not list still asserts the vocabulary
    // gate + the no-integration boundary (honest by construction).
    assert!(c.will_not.iter().any(|w| w.contains("outside its declared vocabulary")));
    assert!(c.will_not.iter().any(|w| w.contains("none bound")));
}
