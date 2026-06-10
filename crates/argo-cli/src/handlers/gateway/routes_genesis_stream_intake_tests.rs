//! Unit tests for the Genesis intake-capture wiring (UF Phase-A · A1-wire).
//!
//! These pin the INPUT seam the A1 / #2711 review surfaced: the operator's
//! schedule / carve-outs / measurable goals, submitted on the
//! `POST /api/v1/genesis` body, must land in `plan.spec` (non-empty) so the
//! `vertical_ready` review surfaces the REAL goals and A2's emit no longer
//! falls back to the default KPI skeleton. Back-compat (text-only path leaves
//! the slots empty) is pinned too.

use super::apply_intake;
use argo_genesis::{materialize_spec, Archetype, BroadRoleDraft, CapabilitySpec, GenesisPlan};
use argo_vertical::{KpiComparator, KpiKind};
use serde_json::json;

/// A real from-scratch plan whose `spec` is `Some` (mirrors the SSE path's
/// broad-role / from-scratch branch).
fn from_scratch_plan() -> GenesisPlan {
    let draft = BroadRoleDraft {
        charter: "a vineyard logistics coordinator who schedules harvests and \
                  dispatches haulage to the winery"
            .into(),
        persona: "vinny".into(),
        persona_description: "Coordinates vineyard harvest scheduling and haulage.".into(),
        action_vocabulary: vec!["schedule_harvest".into(), "send_dispatch_notice".into()],
        success_metrics: vec!["zero missed harvest windows".into()],
        onboarding_script: "Hi, I keep the harvest calendar tight.".into(),
    };
    let spec = CapabilitySpec::from_broad_draft(draft, Archetype::CalendarCoordinator);
    let plan = materialize_spec(&spec).expect("materialize a from-scratch plan");
    // Sanity: a freshly-materialized broad-draft plan has empty intake slots —
    // exactly the bug this brief fixes. (Mirrors
    // `from_broad_draft_leaves_new_intake_slots_empty`.)
    let s = plan.spec.as_ref().expect("from-scratch plan carries a spec");
    assert!(s.cadence.is_none() && s.exceptions.is_empty() && s.success_criteria.is_empty());
    plan
}

#[test]
fn apply_intake_populates_all_three_slots_from_body() {
    let mut plan = from_scratch_plan();
    let body = json!({
        "role_description": "a vineyard logistics coordinator who schedules harvests",
        "cadence": "every weekday at 9am",
        "exceptions": ["never touch legal escalations", "hand off refunds over $5k"],
        "success_criteria": [
            { "name": "on-time dispatch rate", "kind": "percentage", "target": 95, "comparator": "gte" },
            { "name": "missed harvest windows", "kind": "count", "target": 0, "comparator": "lte" }
        ]
    })
    .to_string();

    apply_intake(&body, &mut plan);

    let spec = plan.spec.as_ref().expect("spec present");
    assert_eq!(spec.cadence.as_deref(), Some("every weekday at 9am"));
    assert_eq!(
        spec.exceptions,
        vec![
            "never touch legal escalations".to_string(),
            "hand off refunds over $5k".to_string()
        ]
    );
    // The acceptance check: success_criteria is NON-EMPTY and carries the
    // operator's real typed goals (not the default KPI skeleton).
    assert_eq!(spec.success_criteria.len(), 2);
    assert_eq!(spec.success_criteria[0].name, "on-time dispatch rate");
    assert_eq!(spec.success_criteria[0].kind, KpiKind::Percentage);
    assert_eq!(spec.success_criteria[0].comparator, KpiComparator::Gte);
    assert!((spec.success_criteria[0].target - 95.0).abs() < f64::EPSILON);
    assert_eq!(spec.success_criteria[1].kind, KpiKind::Count);
    assert_eq!(spec.success_criteria[1].comparator, KpiComparator::Lte);
}

#[test]
fn text_only_body_leaves_slots_empty() {
    // Back-compat: a submission with no capture-panel values (the text-only
    // path) deserializes to the empty capture and leaves the spec untouched.
    let mut plan = from_scratch_plan();
    let body = json!({
        "role_description": "a vineyard logistics coordinator who schedules harvests"
    })
    .to_string();

    apply_intake(&body, &mut plan);

    let spec = plan.spec.as_ref().expect("spec present");
    assert!(spec.cadence.is_none());
    assert!(spec.exceptions.is_empty());
    assert!(spec.success_criteria.is_empty());
}

#[test]
fn blank_and_whitespace_values_are_dropped() {
    let mut plan = from_scratch_plan();
    let body = json!({
        "cadence": "   ",
        "exceptions": ["", "  ", "real carve-out"],
        "success_criteria": [
            { "name": "  ", "kind": "count", "target": 0, "comparator": "gte" },
            { "name": "  kept goal  ", "kind": "score", "target": 8, "comparator": "gte" }
        ]
    })
    .to_string();

    apply_intake(&body, &mut plan);

    let spec = plan.spec.as_ref().expect("spec present");
    // Whitespace-only cadence is not "supplied" → stays None.
    assert!(spec.cadence.is_none());
    assert_eq!(spec.exceptions, vec!["real carve-out".to_string()]);
    assert_eq!(spec.success_criteria.len(), 1);
    assert_eq!(spec.success_criteria[0].name, "kept goal");
}

#[test]
fn archetype_path_without_spec_is_a_noop() {
    // The confident-archetype path carries `spec == None`; applying intake must
    // not panic and must leave the plan unchanged (the archetype review surfaces
    // KPIs from the manifest, not the spec).
    let mut plan = from_scratch_plan();
    plan.spec = None;
    let body = json!({ "cadence": "daily", "exceptions": ["x"] }).to_string();

    apply_intake(&body, &mut plan);

    assert!(plan.spec.is_none());
}

#[test]
fn malformed_body_is_safe() {
    let mut plan = from_scratch_plan();
    apply_intake("not json at all {{{", &mut plan);
    let spec = plan.spec.as_ref().expect("spec present");
    assert!(spec.cadence.is_none() && spec.success_criteria.is_empty());
}

#[test]
fn vertical_ready_payload_carries_all_three_intake_slots() {
    // DEC-048 rule 2 (captured-equals-built), gateway seam: the `vertical_ready`
    // SSE payload the SPA renders must surface cadence / exceptions / typed
    // success-criteria so the pre-hire review can show them. `build_final_payload`
    // enriches the derived role contract with `with_intake_slots(plan.spec)`, so
    // every slot applied to the plan reaches the wire.
    use argo_genesis::{
        ComprehensionOutcome, ContractGateInput, EvalFloorOutcome, GateVerdict, SafetySignal,
        SopComprehensionOutcome,
    };

    let mut plan = from_scratch_plan();
    let body = json!({
        "cadence": "every weekday at 5pm",
        "exceptions": ["never touch legal escalations"],
        "success_criteria": [
            { "name": "on-time dispatch rate", "kind": "percentage", "target": 95, "comparator": "gte" }
        ]
    })
    .to_string();
    apply_intake(&body, &mut plan);

    // A composed gate carries the derived role contract (the slots are added by
    // the payload builder, not the gate). Confirmation state is irrelevant here.
    let gate = GateVerdict::compose(
        EvalFloorOutcome::from_counts(2, 2),
        ComprehensionOutcome::from_counts(2, 2),
        SopComprehensionOutcome::not_applicable(),
        SafetySignal::Allow,
        ContractGateInput::from_plan(&plan, None),
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let payload = super::super::routes_genesis_stream_payload::build_final_payload(
        &plan,
        dir.path(),
        "op-x",
        "Vinny",
        3,
        &gate,
        None,
        None,
        None,
    );

    let rc = &payload["role_contract"];
    assert_eq!(rc["cadence"], "every weekday at 5pm");
    assert_eq!(rc["exceptions"][0], "never touch legal escalations");
    assert_eq!(rc["success_criteria"][0]["name"], "on-time dispatch rate");
    assert_eq!(rc["success_criteria"][0]["target"], 95.0);
    assert_eq!(rc["success_criteria"][0]["comparator"], "gte");
}
