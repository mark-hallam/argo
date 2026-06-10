//! Unit tests for the DEC-048 per-slot quality bar.

use super::{cadence_is_vague, exceptions_are_vague, needs_followup};
use crate::intake_machine::{IntakeSlot, IntakeState};

#[test]
fn vague_cadences_are_flagged() {
    for vague in ["Weekly", "monthly", "regularly", "sometimes", "often"] {
        assert!(cadence_is_vague(vague), "{vague} should be vague");
    }
}

#[test]
fn concrete_cadences_pass() {
    for ok in [
        "every weekday at 5pm",
        "daily at 9",
        "Mondays",
        "weekday mornings",
        "on each inbound ticket",
        "whenever a new email arrives",
        "9am sharp",
    ] {
        assert!(!cadence_is_vague(ok), "{ok} should be concrete");
    }
}

#[test]
fn short_exceptions_are_vague_long_ones_pass() {
    assert!(exceptions_are_vague(&["legal stuff".to_string()]));
    assert!(exceptions_are_vague(&[
        "refunds".to_string(),
        "VIPs".to_string()
    ]));
    // One concrete item rescues the set — we only re-ask when NOTHING is
    // actionable.
    assert!(!exceptions_are_vague(&[
        "legal".to_string(),
        "never email clients marked sensitive without my approval".to_string()
    ]));
    assert!(!exceptions_are_vague(&[]));
}

#[test]
fn needs_followup_caps_to_quality_slots() {
    let state = IntakeState {
        cadence: Some("Weekly".to_string()),
        exceptions: vec!["legal stuff".to_string()],
        ..IntakeState::default()
    };
    assert!(needs_followup(&state, IntakeSlot::Cadence).is_some());
    assert!(needs_followup(&state, IntakeSlot::Exceptions).is_some());
    // No quality bar on other slots.
    assert!(needs_followup(&state, IntakeSlot::Charter).is_none());
    assert!(needs_followup(&state, IntakeSlot::Access).is_none());
}

#[test]
fn needs_followup_clears_on_concrete_values() {
    let state = IntakeState {
        cadence: Some("every weekday at 5pm".to_string()),
        exceptions: vec!["never act on invoices over $500 without approval".to_string()],
        ..IntakeState::default()
    };
    assert!(needs_followup(&state, IntakeSlot::Cadence).is_none());
    assert!(needs_followup(&state, IntakeSlot::Exceptions).is_none());
}

#[test]
fn required_followup_pending_only_in_the_followup_window() {
    use super::required_followup_pending;
    let mut s = IntakeState {
        cadence: Some("Weekly".to_string()),
        ..IntakeState::default()
    };
    // Ordinary pending on a filled slot without a used follow-up: no match.
    s.pending_slot = Some(IntakeSlot::Cadence);
    assert_eq!(required_followup_pending(&s), None);
    // The follow-up window: pending + used + filled.
    s.followups_used.push(IntakeSlot::Cadence);
    assert_eq!(required_followup_pending(&s), Some(IntakeSlot::Cadence));
    // Optional slots never defer confirm.
    s.pending_slot = Some(IntakeSlot::Access);
    s.followups_used.push(IntakeSlot::Access);
    s.access_integrations.push("Xero".to_string());
    assert_eq!(required_followup_pending(&s), None);
    // Answered (pending cleared): window closes.
    s.pending_slot = None;
    assert_eq!(required_followup_pending(&s), None);
}
