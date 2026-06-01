//! Unit tests for the [`CapabilitySpec`] IR.

use super::*;
use crate::BroadRoleDraft;

fn vineyard_draft() -> BroadRoleDraft {
    BroadRoleDraft {
        charter: "a vineyard logistics coordinator who schedules harvests and \
                  dispatches trucking to the winery"
            .into(),
        persona: "vinny".into(),
        persona_description: "Coordinates vineyard harvest scheduling and trucking logistics."
            .into(),
        action_vocabulary: vec![
            "schedule_harvest".into(),
            "draft_truck_plan".into(),
            "send_dispatch_notice".into(),
            "wire_funds_to_haulier".into(),
        ],
        success_metrics: vec![
            "zero missed harvest windows".into(),
            "all trucks dispatched on time".into(),
        ],
        onboarding_script: "Hi, I keep the harvest calendar tight and the trucks moving.".into(),
    }
}

#[test]
fn from_broad_draft_types_every_action() {
    let spec = CapabilitySpec::from_broad_draft(vineyard_draft(), Archetype::CalendarCoordinator);
    assert_eq!(spec.persona, "vinny");
    assert_eq!(spec.capabilities.len(), 4);
    assert_eq!(spec.nearest_archetype, Archetype::CalendarCoordinator);
    assert!(spec.from_scratch, "broad-draft spec is from-scratch");
    assert_eq!(spec.success_metrics.len(), 2);
}

#[test]
fn sensitive_and_irreversible_actions_are_forced_to_require_approval() {
    let spec = CapabilitySpec::from_broad_draft(vineyard_draft(), Archetype::CalendarCoordinator);
    let by_name = |n: &str| {
        spec.capabilities
            .iter()
            .find(|c| c.action_name == n)
            .unwrap_or_else(|| panic!("missing {n}"))
            .clone()
    };
    // `send_dispatch_notice` carries the sensitive verb `send`.
    let send = by_name("send_dispatch_notice");
    assert!(send.requires_approval, "sensitive verb must require approval");
    assert_eq!(send.risk, ActionRiskLevel::High);
    // `wire_funds_to_haulier` carries the irreversible verb `wire_funds`.
    let wire = by_name("wire_funds_to_haulier");
    assert!(wire.requires_approval, "irreversible must require approval");
    assert!(!wire.reversible, "wire_funds is irreversible");
    assert_eq!(wire.risk, ActionRiskLevel::High);
    // A plain action stays low/no-approval.
    let sched = by_name("schedule_harvest");
    assert!(!sched.requires_approval);
    assert_eq!(sched.risk, ActionRiskLevel::Low);
}

#[test]
fn escalation_set_is_the_union_of_gated_actions() {
    let spec = CapabilitySpec::from_broad_draft(vineyard_draft(), Archetype::CalendarCoordinator);
    let gated = &spec.escalation.human_approval_required;
    assert!(gated.contains(&"send_dispatch_notice".to_string()));
    assert!(gated.contains(&"wire_funds_to_haulier".to_string()));
    assert!(!gated.contains(&"schedule_harvest".to_string()));
    assert_eq!(spec.escalation.escalate_after_failures, Some(3));
    assert_eq!(spec.escalation.escalate_to.as_deref(), Some("the operator"));
}

#[test]
fn sanitize_action_name_normalises_to_snake_case() {
    assert_eq!(sanitize_action_name("Schedule Harvest!"), "schedule_harvest");
    assert_eq!(sanitize_action_name("send-dispatch"), "send_dispatch");
    assert_eq!(sanitize_action_name("  __weird__  "), "weird");
    assert_eq!(sanitize_action_name("!!!"), "perform_task");
    assert_eq!(sanitize_action_name("ALREADY_SNAKE"), "already_snake");
}

#[test]
fn action_names_preserves_declaration_order() {
    let spec = CapabilitySpec::from_broad_draft(vineyard_draft(), Archetype::CalendarCoordinator);
    assert_eq!(
        spec.action_names(),
        vec![
            "schedule_harvest",
            "draft_truck_plan",
            "send_dispatch_notice",
            "wire_funds_to_haulier",
        ]
    );
}

#[test]
fn executor_intent_round_trips_through_serde() {
    let mcp = ExecutorIntent::Mcp {
        server: "gmail".into(),
        tool: Some("drafts.send".into()),
    };
    let json = serde_json::to_string(&mcp).expect("serialize");
    let back: ExecutorIntent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(mcp, back);
}

#[test]
fn capability_description_titlecases_the_worker_name() {
    // Regression for the LIVE walkthrough: a charter "...success lead
    // named Maya..." produced "as the success lead named maya" (the name
    // lowercased) on every generated capability description. The proper
    // name following a naming trigger must keep its Title case.
    // Persona handle differs from the in-charter name (the real Genesis
    // case the walkthrough hit — the descriptor drops the persona handle,
    // so a name equal to the persona would be deduped out entirely).
    let draft = BroadRoleDraft {
        charter: "the success lead named Maya who keeps customers live".into(),
        persona: "success-lead".into(),
        persona_description: "Customer success lead.".into(),
        action_vocabulary: vec!["draft_welcome".into()],
        success_metrics: vec![],
        onboarding_script: String::new(),
    };
    let spec = CapabilitySpec::from_broad_draft(draft, Archetype::CalendarCoordinator);
    let desc = &spec.capabilities[0].intent;
    assert!(
        desc.contains("named Maya"),
        "expected Title-cased name in description, got {desc:?}"
    );
    assert!(
        !desc.contains("named maya"),
        "the name must NOT be lowercased, got {desc:?}"
    );
}

#[test]
fn role_descriptor_titlecases_lowercased_operator_name() {
    // Even when the operator typed the name in lower case ("named maya"),
    // the descriptor renders it Title-cased.
    let draft = BroadRoleDraft {
        charter: "the success lead named maya for customer onboarding".into(),
        persona: "operator".into(),
        persona_description: "x".into(),
        action_vocabulary: vec![],
        success_metrics: vec![],
        onboarding_script: String::new(),
    };
    let descriptor = role_descriptor(&draft);
    assert!(
        descriptor.contains("Maya"),
        "lowercase operator name must be Title-cased, got {descriptor:?}"
    );
}
