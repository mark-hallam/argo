//! Unit tests for the [`CapabilitySpec`] IR.

use super::*;
use crate::BroadRoleDraft;
use argo_vertical::{KpiComparator, KpiKind};

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
fn fallback_description_is_a_grammatical_verb_phrase_not_an_as_the_concatenation() {
    // Regression for the live `/genesis` finding: the templated fallback used
    // to append "as the {role_descriptor}" — producing ungrammatical,
    // repetitive, camera-visible text like
    // "Archive article as the assistant named Remy reads". The fallback (used
    // only when the LLM author provided no intent) is now a plain capitalized
    // verb phrase ending in a period.
    let draft = BroadRoleDraft {
        charter: "the success lead named Maya who keeps customers live".into(),
        persona: "success-lead".into(),
        persona_description: "Customer success lead.".into(),
        action_vocabulary: vec!["archive_article".into(), "extract_takeaways".into()],
        success_metrics: vec![],
        onboarding_script: String::new(),
    };
    let spec = CapabilitySpec::from_broad_draft(draft, Archetype::CalendarCoordinator);
    let descs: Vec<&str> = spec.capabilities.iter().map(|c| c.intent.as_str()).collect();
    assert_eq!(descs, vec!["Archive article.", "Extract takeaways."]);
    for desc in &descs {
        assert!(
            !desc.contains(" as the "),
            "the fallback must not carry the 'as the …' tail, got {desc:?}"
        );
        assert!(
            desc.ends_with('.'),
            "the fallback must be a grammatical sentence, got {desc:?}"
        );
    }
}

#[test]
fn degenerate_action_name_still_yields_a_grammatical_fallback() {
    // A label that sanitises to a degenerate verb phrase still renders a clean
    // sentence (never a blank or dangling description).
    let draft = BroadRoleDraft {
        charter: "general assistant".into(),
        persona: "helper".into(),
        persona_description: "x".into(),
        action_vocabulary: vec!["!!!".into()],
        success_metrics: vec![],
        onboarding_script: String::new(),
    };
    let spec = CapabilitySpec::from_broad_draft(draft, Archetype::CalendarCoordinator);
    assert_eq!(spec.capabilities[0].intent, "Perform a task.");
}

#[test]
fn from_broad_draft_leaves_new_intake_slots_empty() {
    // The legacy broad-draft path carries no cadence / exceptions / typed
    // success-criteria — the richer intake (capture panel) populates them.
    let spec = CapabilitySpec::from_broad_draft(vineyard_draft(), Archetype::CalendarCoordinator);
    assert_eq!(spec.cadence, None);
    assert!(spec.exceptions.is_empty());
    assert!(spec.success_criteria.is_empty());
}

#[test]
fn cadence_exceptions_success_criteria_round_trip() {
    // A spec that DID capture all three intake slots survives a JSON
    // serialize → deserialize round-trip byte-for-byte (typed fields, not
    // free text). `target` is an `f64` reusing the manifest KPI vocabulary.
    let mut spec =
        CapabilitySpec::from_broad_draft(vineyard_draft(), Archetype::CalendarCoordinator);
    spec.cadence = Some("every weekday at 9am".into());
    spec.exceptions = vec![
        "never dispatch trucks during a frost warning".into(),
        "hand off any contract over $50k to the operator".into(),
    ];
    spec.success_criteria = vec![
        SuccessCriterion {
            name: "missed harvest windows".into(),
            kind: KpiKind::Count,
            target: 0.0,
            comparator: KpiComparator::Lte,
        },
        SuccessCriterion {
            name: "on-time dispatch rate".into(),
            kind: KpiKind::Percentage,
            target: 95.0,
            comparator: KpiComparator::Gte,
        },
    ];

    let json = serde_json::to_string(&spec).expect("serialize spec");
    let back: CapabilitySpec = serde_json::from_str(&json).expect("deserialize spec");
    assert_eq!(spec, back, "all three intake slots round-trip");
    assert_eq!(back.success_criteria.len(), 2);
    assert_eq!(back.success_criteria[1].kind, KpiKind::Percentage);
    assert_eq!(back.success_criteria[1].comparator, KpiComparator::Gte);
    assert!((back.success_criteria[1].target - 95.0).abs() < f64::EPSILON);
}

#[test]
fn old_spec_without_new_fields_still_deserializes() {
    // Back-compat: a spec JSON authored BEFORE the three slots existed (no
    // `cadence` / `exceptions` / `success_criteria` keys) must still
    // deserialize — `#[serde(default)]` fills the gaps.
    let legacy = r#"{
        "charter": "a vineyard logistics coordinator",
        "persona": "vinny",
        "persona_description": "Coordinates harvest logistics.",
        "responsibilities": [],
        "capabilities": [],
        "success_metrics": ["zero missed windows"],
        "onboarding_script": "Hi.",
        "escalation": {
            "human_approval_required": [],
            "escalate_after_failures": 3,
            "escalate_to": "the operator"
        },
        "nearest_archetype": "calendar_coordinator",
        "from_scratch": true
    }"#;
    let spec: CapabilitySpec = serde_json::from_str(legacy).expect("legacy spec deserializes");
    assert_eq!(spec.persona, "vinny");
    assert_eq!(spec.cadence, None);
    assert!(spec.exceptions.is_empty());
    assert!(spec.success_criteria.is_empty());
}
