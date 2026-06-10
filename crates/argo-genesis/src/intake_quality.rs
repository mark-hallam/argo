//! Per-slot quality bar for the conversational intake (DEC-048 rule 4).
//!
//! Deterministic, provider-free vagueness heuristics: after a slot fills, the
//! route asks AT MOST ONE follow-up when the captured value is too vague to
//! mint a useful teammate ("Weekly" as a cadence, "legal stuff" as an
//! exception). The cap lives in [`crate::intake_machine::IntakeState::followups_used`];
//! a second vague answer is ACCEPTED — depth must never wedge intake, and a
//! provider failure can never block it because nothing here calls one.

use crate::intake_machine::{IntakeSlot, IntakeState};

/// Tokens that make a cadence answer concrete: a day-of-week / day-band…
const DAY_TOKENS: &[&str] = &[
    "monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday", "mon", "tue",
    "wed", "thu", "fri", "sat", "sun", "weekday", "weekend", "daily", "hourly", "morning",
    "afternoon", "evening", "noon", "midnight", "night",
];

/// …or an event trigger ("on each inbound ticket") — trigger cadences carry no
/// clock time and must NOT be flagged vague.
const TRIGGER_TOKENS: &[&str] = &[
    "when", "whenever", "each", "every time", "on new", "inbound", "incoming", "arrives",
    "arrival", "trigger", "ticket", "email", "message", "request", "webhook", "mention",
];

/// A cadence is vague when it names neither a time (no digit), nor a
/// day/band, nor a trigger — e.g. "Weekly", "sometimes", "regularly".
///
/// Day tokens match as WHOLE words ("mon" must not match "monthly"); the
/// multi-word trigger phrases match as substrings.
#[must_use]
pub fn cadence_is_vague(cadence: &str) -> bool {
    let lower = cadence.to_lowercase();
    let has_digit = lower.chars().any(|c| c.is_ascii_digit());
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    // Short tokens (mon/tue/…/noon) match exactly so "mon" never matches
    // "monthly"; longer tokens allow a plural/suffix ("Mondays", "weekday
    // mornings") via prefix match.
    let has_day = DAY_TOKENS.iter().any(|t| {
        words
            .iter()
            .any(|w| if t.len() <= 4 { w == t } else { w.starts_with(t) })
    });
    let has_trigger = TRIGGER_TOKENS.iter().any(|t| {
        if t.contains(' ') {
            lower.contains(t)
        } else {
            words.contains(t)
        }
    });
    !(has_digit || has_day || has_trigger)
}

/// Exceptions are vague when every captured item is under three words —
/// "legal stuff" tells the teammate nothing actionable about WHAT to hand off.
#[must_use]
pub fn exceptions_are_vague(exceptions: &[String]) -> bool {
    let items: Vec<&String> = exceptions.iter().filter(|e| !e.trim().is_empty()).collect();
    !items.is_empty() && items.iter().all(|e| e.split_whitespace().count() < 3)
}

/// The ONE follow-up question to ask when `slot` just filled vaguely, or
/// `None` when the captured value is concrete enough (or the slot has no
/// quality bar). Pure — the re-ask cap is enforced by the caller via
/// [`IntakeState::followups_used`].
///
/// # Examples
///
/// ```
/// use argo_genesis::{needs_followup, IntakeSlot, IntakeState};
/// let mut state = IntakeState::default();
/// state.cadence = Some("Weekly".to_string());
/// assert!(needs_followup(&state, IntakeSlot::Cadence).is_some());
/// state.cadence = Some("every weekday at 5pm".to_string());
/// assert!(needs_followup(&state, IntakeSlot::Cadence).is_none());
/// ```
#[must_use]
pub fn needs_followup(state: &IntakeState, slot: IntakeSlot) -> Option<&'static str> {
    match slot {
        IntakeSlot::Cadence => {
            let cadence = state.cadence.as_deref()?;
            cadence_is_vague(cadence).then_some(
                "Which day(s) and what time should that be — e.g. 'every weekday at 5pm' — \
                 or is it triggered by something, like 'each new ticket'?",
            )
        }
        IntakeSlot::Exceptions => exceptions_are_vague(&state.exceptions).then_some(
            "Could you spell those out a little — what specifically should they hand to \
             you instead of acting on themselves?",
        ),
        _ => None,
    }
}

/// The REQUIRED slot whose quality follow-up is currently awaiting an answer,
/// or `None`. True only in the follow-up window: the last asked question
/// targets a REQUIRED slot that is already filled and has used its one
/// follow-up. Optional slots never match — they must stay confirm-bypassable
/// (DEC-048), while a pending REQUIRED follow-up defers `confirm` so the
/// quality bar cannot be silently skipped (adversarial-review P1, 2026-06-10).
#[must_use]
pub fn required_followup_pending(state: &IntakeState) -> Option<IntakeSlot> {
    let slot = state.pending_slot?;
    (IntakeSlot::REQUIRED.contains(&slot)
        && state.followups_used.contains(&slot)
        && state.slot_filled(slot))
    .then_some(slot)
}

#[cfg(test)]
#[path = "intake_quality_tests.rs"]
mod tests;
