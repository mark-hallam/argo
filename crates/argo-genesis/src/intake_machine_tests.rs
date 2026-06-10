//! Tests for the pure conversational-intake state machine (DEC-039).

use super::*;
use crate::materialize::materialize_spec;

/// A fully-walked conversation, filled via the deterministic wizard path.
fn complete_state() -> IntakeState {
    let mut s = IntakeState::default();
    s.apply_raw_answer(IntakeSlot::Charter, "Be my support agent who clears the inbound ticket queue");
    s.apply_raw_answer(IntakeSlot::Persona, "Sam");
    s.apply_raw_answer(
        IntakeSlot::Responsibilities,
        "Answer tier-1 tickets\nTriage by urgency\nEscalate refunds",
    );
    s.apply_raw_answer(IntakeSlot::Cadence, "during business hours, on each inbound ticket");
    s.apply_raw_answer(IntakeSlot::Exceptions, "never issue a refund alone\nnever touch legal");
    s.apply_raw_answer(IntakeSlot::SuccessCriteria, "first response under 5 minutes\nCSAT above 90");
    s
}

#[test]
fn asks_required_slots_in_deterministic_order() {
    let mut s = IntakeState::default();
    // Charter is first.
    match s.next_step() {
        NextStep::Ask { slot, question } => {
            assert_eq!(slot, IntakeSlot::Charter);
            assert!(!question.is_empty());
        }
        other => panic!("expected Ask(Charter), got {other:?}"),
    }
    // Fill charter → persona is next.
    s.apply_raw_answer(IntakeSlot::Charter, "Be my support agent for the ticket queue");
    assert!(matches!(
        s.next_step(),
        NextStep::Ask { slot: IntakeSlot::Persona, .. }
    ));
}

#[test]
fn wizard_path_completes_to_confirm_then_done() {
    let mut s = complete_state();
    assert!(s.is_complete(), "all required slots should be filled");
    // DEC-048: with the required set complete, the optional Access /
    // Escalation questions come next (in OPTIONAL order)…
    assert!(matches!(
        s.next_step(),
        NextStep::Ask { slot: IntakeSlot::Access, .. }
    ));
    s.mark_optional_resolved(IntakeSlot::Access);
    assert!(matches!(
        s.next_step(),
        NextStep::Ask { slot: IntakeSlot::Escalation, .. }
    ));
    s.mark_optional_resolved(IntakeSlot::Escalation);
    // …then the terminal Confirm (operator has not confirmed yet).
    let spec = match s.next_step() {
        NextStep::Confirm(spec) => spec,
        other => panic!("expected Confirm, got {other:?}"),
    };
    assert_eq!(spec.persona, "sam");
    assert!(!spec.capabilities.is_empty());
    // After confirmation the same state yields Done with an equivalent spec.
    s.confirmed = true;
    match s.next_step() {
        NextStep::Done(done) => assert_eq!(done.persona, spec.persona),
        other => panic!("expected Done, got {other:?}"),
    }
}

#[test]
fn confirm_bypasses_unresolved_optional_slots() {
    // DEC-048: optional questions must NEVER wedge the build — a confirmed
    // state goes straight to Done even with Access/Escalation unasked.
    let mut s = complete_state();
    s.confirmed = true;
    assert!(matches!(s.next_step(), NextStep::Done(_)));
}

#[test]
fn optional_answers_fill_and_override_escalation_defaults() {
    let mut s = complete_state();
    s.apply_raw_answer(IntakeSlot::Access, "Xero, Gmail and Slack");
    assert_eq!(s.access_integrations, vec!["Xero", "Gmail", "Slack"]);
    assert!(s.slot_filled(IntakeSlot::Access));
    // Number answer → threshold; the seeded default is overridden on assembly.
    s.apply_raw_answer(IntakeSlot::Escalation, "after 2 tries");
    assert_eq!(s.escalation_failures_threshold, Some(2));
    let spec = s.assemble_spec();
    assert_eq!(spec.escalation.escalate_after_failures, Some(2));
    assert_eq!(spec.access_integrations, vec!["Xero", "Gmail", "Slack"]);
    // Digit-free answer → "who to" free text.
    let mut t = complete_state();
    t.apply_raw_answer(IntakeSlot::Escalation, "my ops lead");
    assert_eq!(t.escalation_to.as_deref(), Some("my ops lead"));
    assert!(t.slot_filled(IntakeSlot::Escalation));
}

#[test]
fn skipped_optionals_keep_seeded_escalation_defaults() {
    let mut s = complete_state();
    s.mark_optional_resolved(IntakeSlot::Access);
    s.mark_optional_resolved(IntakeSlot::Escalation);
    let spec = s.assemble_spec();
    assert_eq!(spec.escalation.escalate_after_failures, Some(3));
    assert_eq!(spec.escalation.escalate_to.as_deref(), Some("the operator"));
    assert!(spec.access_integrations.is_empty());
}

#[test]
fn delta_bounds_escalation_threshold_to_sane_band() {
    let mut s = IntakeState::default();
    s.apply_delta(SlotDelta {
        escalation_failures_threshold: Some(500),
        ..SlotDelta::default()
    });
    assert_eq!(s.escalation_failures_threshold, None, "out-of-band rejected");
    s.apply_delta(SlotDelta {
        escalation_failures_threshold: Some(5),
        ..SlotDelta::default()
    });
    assert_eq!(s.escalation_failures_threshold, Some(5));
}

#[test]
fn legacy_state_json_without_dec048_fields_deserializes() {
    // A pre-DEC-048 conversation persisted on disk: none of the new fields.
    let legacy = r#"{
        "charter": "Clear the queue",
        "persona": "Sam",
        "responsibilities": ["triage"],
        "cadence": "weekdays at 9",
        "exceptions": ["never refund alone"],
        "success_criteria": [],
        "transcript": [],
        "confirmed": false
    }"#;
    let s: IntakeState = serde_json::from_str(legacy).expect("legacy state deserializes");
    assert_eq!(s.pending_slot, None, "serde default");
    assert!(s.access_integrations.is_empty());
    assert!(s.optional_resolved.is_empty());
    assert!(s.followups_used.is_empty());
    // The legacy answer-targeting fallback still works.
    assert_eq!(s.first_unfilled_slot(), Some(IntakeSlot::SuccessCriteria));
}

#[test]
fn assembled_spec_carries_every_captured_slot() {
    let spec = complete_state().assemble_spec();
    assert!(spec.charter.contains("support agent"));
    assert_eq!(spec.responsibilities.len(), 3);
    assert_eq!(spec.cadence.as_deref(), Some("during business hours, on each inbound ticket"));
    assert_eq!(spec.exceptions.len(), 2);
    assert_eq!(spec.success_criteria.len(), 2);
    // Reuses the canonical typed criterion — NOT a parallel type.
    assert_eq!(spec.success_criteria[0].name, "first response under 5 minutes");
}

#[test]
fn assembled_spec_materializes_via_the_one_shot_materializer() {
    // The load-bearing round-trip: the conversation's spec goes through the
    // IDENTICAL materialize_spec the one-shot from-scratch path uses (no parallel
    // materializer), and produces a valid GenesisPlan.
    let spec = complete_state().assemble_spec();
    let plan = materialize_spec(&spec).expect("conversation spec must materialize like one-shot");
    assert_eq!(plan.persona, "sam");
    assert!(plan.spec.is_some(), "from-scratch plan carries the spec");
    let actions = plan
        .manifest
        .actions
        .as_ref()
        .map(|a| a.actions.len())
        .unwrap_or(0);
    assert!(actions >= 1, "materialized teammate has at least one action");
}

#[test]
fn apply_delta_merges_scalar_and_list_slots() {
    let mut s = IntakeState::default();
    s.apply_delta(SlotDelta {
        charter: Some("  Be my recruiter screener  ".to_string()),
        persona: Some("Riley".to_string()),
        responsibilities: vec!["screen resumes".to_string(), "  ".to_string()],
        cadence: None,
        exceptions: vec!["never reject without review".to_string()],
        success_criteria: vec![],
        ..SlotDelta::default()
    });
    assert_eq!(s.charter.as_deref(), Some("Be my recruiter screener"));
    assert_eq!(s.persona.as_deref(), Some("Riley"));
    assert_eq!(s.responsibilities, vec!["screen resumes".to_string()]); // blank dropped
    assert_eq!(s.exceptions.len(), 1);
}

#[test]
fn empty_or_whitespace_answers_do_not_fill_a_slot() {
    let mut s = IntakeState::default();
    s.apply_raw_answer(IntakeSlot::Charter, "    ");
    assert!(!s.slot_filled(IntakeSlot::Charter));
    assert_eq!(s.first_unfilled_slot(), Some(IntakeSlot::Charter));
}

#[test]
fn slot_delta_is_empty_detects_no_usable_value() {
    assert!(SlotDelta::default().is_empty());
    let blank = SlotDelta {
        charter: Some("   ".to_string()),
        responsibilities: vec!["  ".to_string()],
        ..SlotDelta::default()
    };
    assert!(blank.is_empty());
    let real = SlotDelta {
        cadence: Some("weekdays".to_string()),
        ..SlotDelta::default()
    };
    assert!(!real.is_empty());
}

#[test]
fn transcript_records_both_speakers() {
    let mut s = IntakeState::default();
    s.record_assistant("What role?");
    s.record_operator("A support agent");
    assert_eq!(s.transcript.len(), 2);
    assert_eq!(s.transcript[0].speaker, Speaker::Assistant);
    assert_eq!(s.transcript[1].speaker, Speaker::Operator);
}
