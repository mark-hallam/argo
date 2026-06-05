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
    // First terminal step is Confirm (operator has not confirmed yet).
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
