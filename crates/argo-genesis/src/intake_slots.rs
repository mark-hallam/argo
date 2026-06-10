//! The conversational-intake slot vocabulary (DEC-039 + DEC-048).
//!
//! Extracted from [`crate::intake_machine`] as a sibling module (the
//! 500-line file-budget split pattern); the public path is unchanged —
//! [`crate::intake_machine`] re-exports [`IntakeSlot`], and the crate root
//! re-exports it from there. REQUIRED slots gate completion; OPTIONAL slots
//! (Access / Escalation, DEC-048) are asked but skippable and can never
//! wedge the confirm transition.

use serde::{Deserialize, Serialize};

/// One required slot the conversation must fill before a spec can be assembled.
/// The declaration order IS the deterministic ask sequence and the wizard
/// fallback order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntakeSlot {
    /// The role description in the operator's own words (becomes `charter`).
    Charter,
    /// What to call the teammate (becomes the persona slug).
    Persona,
    /// Day-to-day responsibilities in plain language.
    Responsibilities,
    /// WHEN the teammate runs — a schedule or a trigger.
    Cadence,
    /// Carve-outs the teammate must never act on / must hand off.
    Exceptions,
    /// HOW success is measured — typed, measurable criteria.
    SuccessCriteria,
    /// What systems/accounts the teammate will need (draft-only at birth —
    /// DEC-048; asked but skippable, never gates completion).
    Access,
    /// When to stop and escalate — failure count / threshold + who to.
    /// Asked but skippable; feeds the EXISTING [`crate::spec::EscalationSpec`].
    Escalation,
}

impl IntakeSlot {
    /// The required slots, in deterministic ask order.
    pub const REQUIRED: &'static [IntakeSlot] = &[
        IntakeSlot::Charter,
        IntakeSlot::Persona,
        IntakeSlot::Responsibilities,
        IntakeSlot::Cadence,
        IntakeSlot::Exceptions,
        IntakeSlot::SuccessCriteria,
    ];

    /// Optional slots, asked after the required set is complete (DEC-048 rule
    /// 4). An explicit skip resolves them; they NEVER gate [`IntakeState::is_complete`]
    /// or the confirm transition, so they cannot wedge the build.
    pub const OPTIONAL: &'static [IntakeSlot] = &[IntakeSlot::Access, IntakeSlot::Escalation];

    /// Stable identifier for the slot (wire + LLM extraction key).
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            IntakeSlot::Charter => "charter",
            IntakeSlot::Persona => "persona",
            IntakeSlot::Responsibilities => "responsibilities",
            IntakeSlot::Cadence => "cadence",
            IntakeSlot::Exceptions => "exceptions",
            IntakeSlot::SuccessCriteria => "success_criteria",
            IntakeSlot::Access => "access",
            IntakeSlot::Escalation => "escalation",
        }
    }

    /// The deterministic wizard question for this slot — the fail-safe asked
    /// verbatim when the provider is unavailable. Plain English, one ask each.
    #[must_use]
    pub fn wizard_question(self) -> &'static str {
        match self {
            IntakeSlot::Charter => {
                "What role do you want to hire for, and what should they take off your plate?"
            }
            IntakeSlot::Persona => "What would you like to call them? (a first name is perfect)",
            IntakeSlot::Responsibilities => {
                "What are their main day-to-day responsibilities? (one per line is fine)"
            }
            IntakeSlot::Cadence => {
                "When should they work — on a set schedule, or in response to something happening?"
            }
            IntakeSlot::Exceptions => {
                "Anything they should never do on their own, or always hand to a person first?"
            }
            IntakeSlot::SuccessCriteria => {
                "How will you know they're doing a great job? What does success look like?"
            }
            IntakeSlot::Access => {
                "What systems or accounts will they need to look at or use — email, a CRM, \
                 a finance tool? They start draft-only; nothing is connected without you. \
                 (Say 'skip' if you're not sure yet.)"
            }
            IntakeSlot::Escalation => {
                "When should they stop and bring something to you — after how many failed \
                 tries, or above what amount? (Say 'skip' to keep the default: after 3 \
                 failures, to you.)"
            }
        }
    }
}

/// Who spoke a transcript turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Speaker {
    /// The Genesis intake assistant (a proposed question).
    Assistant,
    /// The operator (an answer).
    Operator,
}

/// One captured dialogue turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntakeTurn {
    /// Who spoke.
    pub speaker: Speaker,
    /// The verbatim text.
    pub text: String,
}
