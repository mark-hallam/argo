//! Pure, transport-free slot state machine for the Genesis conversational
//! intake — the "Talk it through" mode locked by DEC-039.
//!
//! DEC-039 is **additive**: the one-shot "Describe it" flow
//! ([`crate::pipeline::plan`] + the `/api/v1/genesis` SSE) stays unchanged. This
//! module elicits the SAME [`CapabilitySpec`] slots that one-shot already fills —
//! `responsibilities` / `cadence` / `exceptions` / `success_criteria` plus
//! charter + persona — but by DIALOGUE, for operators who can't describe a role
//! cold.
//!
//! ## No parallel materializer
//!
//! The machine NEVER forks the spec types or the materialize path. It assembles
//! a [`CapabilitySpec`] via the EXISTING [`CapabilitySpec::from_broad_draft`]
//! (the archetype-prior constructor the one-shot deterministic path also uses),
//! then overlays the captured intake slots. The route then runs the IDENTICAL
//! [`crate::materialize::materialize_spec`] → review → hire path. The capability
//! synthesis is the deterministic archetype seed, so the assembled spec is valid
//! with NO provider call — that is the fail-safe wizard guarantee (DEC-039: a
//! no-LLM demo still completes).
//!
//! ## Pure
//!
//! No transport edge here (mirrors [`crate::author`] / [`crate::rubric`]). The
//! LLM enrichment (a better next question + a smarter answer→slot extraction)
//! lives in [`crate::intake_llm`]; on ANY provider failure the caller falls back
//! to [`IntakeSlot::wizard_question`] + [`IntakeState::apply_raw_answer`] here,
//! so the conversation always advances deterministically.

use crate::archetype::{classify_charter, Archetype};
use crate::broad_role::BroadRoleDraft;
use crate::charter_dialogue::name_to_persona_slug;
use crate::spec::{CapabilitySpec, SuccessCriterion};
use argo_vertical::{KpiComparator, KpiKind};
use serde::{Deserialize, Serialize};

/// Archetype used when the deterministic classifier cannot pick one (only
/// possible for an empty charter, which the machine never assembles on). A
/// general support agent is the most neutral seed.
const DEFAULT_ARCHETYPE: Archetype = Archetype::CustomerSupportAgent;

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

/// A bundle of extracted slot values to merge into [`IntakeState`]. The LLM
/// extraction path ([`crate::intake_llm`]) produces this; the wizard path uses
/// [`IntakeState::apply_raw_answer`] instead. Every field is optional/additive,
/// so a partial extraction merges cleanly.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct SlotDelta {
    /// New / refined charter text.
    #[serde(default)]
    pub charter: Option<String>,
    /// New / refined persona name.
    #[serde(default)]
    pub persona: Option<String>,
    /// Responsibilities to append.
    #[serde(default)]
    pub responsibilities: Vec<String>,
    /// New / refined cadence.
    #[serde(default)]
    pub cadence: Option<String>,
    /// Exceptions to append.
    #[serde(default)]
    pub exceptions: Vec<String>,
    /// Typed success criteria to append.
    #[serde(default)]
    pub success_criteria: Vec<SuccessCriterion>,
}

impl SlotDelta {
    /// `true` when the delta carries no usable value (so the caller can fall
    /// back to filling the current slot from the raw answer text).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.charter.as_deref().is_none_or(|c| c.trim().is_empty())
            && self.persona.as_deref().is_none_or(|p| p.trim().is_empty())
            && self.cadence.as_deref().is_none_or(|c| c.trim().is_empty())
            && self.responsibilities.iter().all(|r| r.trim().is_empty())
            && self.exceptions.iter().all(|e| e.trim().is_empty())
            && self
                .success_criteria
                .iter()
                .all(|c| c.name.trim().is_empty())
    }
}

/// The full conversation state — captured slots + the running transcript. Serde
/// round-trips it for the per-tenant persistence ([`crate::intake_store`]).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IntakeState {
    /// The role description in the operator's words.
    #[serde(default)]
    pub charter: Option<String>,
    /// The teammate's name (pre-slug).
    #[serde(default)]
    pub persona: Option<String>,
    /// Day-to-day responsibilities.
    #[serde(default)]
    pub responsibilities: Vec<String>,
    /// WHEN the teammate runs.
    #[serde(default)]
    pub cadence: Option<String>,
    /// Carve-outs the teammate must hand off.
    #[serde(default)]
    pub exceptions: Vec<String>,
    /// Typed, measurable success criteria.
    #[serde(default)]
    pub success_criteria: Vec<SuccessCriterion>,
    /// The dialogue, oldest first.
    #[serde(default)]
    pub transcript: Vec<IntakeTurn>,
    /// Set once the operator confirms the assembled spec — the `Confirm` →
    /// `Done` transition.
    #[serde(default)]
    pub confirmed: bool,
}

/// What the conversation should do next, given the current [`IntakeState`].
#[derive(Debug, Clone, PartialEq)]
pub enum NextStep {
    /// Ask the operator the next question. `question` is the deterministic
    /// wizard text by default; the route overlays an LLM-proposed phrasing when
    /// the provider is available.
    Ask {
        /// Which slot this question fills.
        slot: IntakeSlot,
        /// The question to show the operator.
        question: String,
    },
    /// Every required slot is filled — present the assembled spec for the
    /// operator to confirm before hire.
    Confirm(Box<CapabilitySpec>),
    /// The operator confirmed; the spec is ready for the materialize → hire path.
    Done(Box<CapabilitySpec>),
}

impl IntakeState {
    /// Record an assistant turn (a proposed question) on the transcript.
    pub fn record_assistant(&mut self, text: impl Into<String>) {
        self.transcript.push(IntakeTurn {
            speaker: Speaker::Assistant,
            text: text.into(),
        });
    }

    /// Record an operator turn (an answer) on the transcript.
    pub fn record_operator(&mut self, text: impl Into<String>) {
        self.transcript.push(IntakeTurn {
            speaker: Speaker::Operator,
            text: text.into(),
        });
    }

    /// The first required slot still empty, in deterministic order; `None` when
    /// every required slot is filled.
    #[must_use]
    pub fn first_unfilled_slot(&self) -> Option<IntakeSlot> {
        IntakeSlot::REQUIRED
            .iter()
            .copied()
            .find(|slot| !self.slot_filled(*slot))
    }

    /// `true` when `slot` already carries a non-empty value.
    #[must_use]
    pub fn slot_filled(&self, slot: IntakeSlot) -> bool {
        match slot {
            IntakeSlot::Charter => self.charter.as_deref().is_some_and(|c| !c.trim().is_empty()),
            IntakeSlot::Persona => self.persona.as_deref().is_some_and(|p| !p.trim().is_empty()),
            IntakeSlot::Responsibilities => self.responsibilities.iter().any(|r| !r.trim().is_empty()),
            IntakeSlot::Cadence => self.cadence.as_deref().is_some_and(|c| !c.trim().is_empty()),
            IntakeSlot::Exceptions => self.exceptions.iter().any(|e| !e.trim().is_empty()),
            IntakeSlot::SuccessCriteria => self.success_criteria.iter().any(|c| !c.name.trim().is_empty()),
        }
    }

    /// `true` when every required slot is filled.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.first_unfilled_slot().is_none()
    }

    /// Merge an extracted [`SlotDelta`] (the LLM path). Non-empty scalar fields
    /// overwrite; list fields append (trimmed, blanks dropped).
    pub fn apply_delta(&mut self, delta: SlotDelta) {
        if let Some(charter) = non_empty(delta.charter) {
            self.charter = Some(charter);
        }
        if let Some(persona) = non_empty(delta.persona) {
            self.persona = Some(persona);
        }
        if let Some(cadence) = non_empty(delta.cadence) {
            self.cadence = Some(cadence);
        }
        extend_trimmed(&mut self.responsibilities, delta.responsibilities);
        extend_trimmed(&mut self.exceptions, delta.exceptions);
        for mut criterion in delta.success_criteria {
            criterion.name = criterion.name.trim().to_string();
            if !criterion.name.is_empty() {
                self.success_criteria.push(criterion);
            }
        }
    }

    /// Fill a SPECIFIC slot from the operator's raw answer text — the
    /// deterministic wizard path used when the LLM extraction is unavailable or
    /// empty. List slots split the answer on newlines; success-criteria lines
    /// become count-based `>= 1` criteria (the honest "captured but not yet
    /// typed" default the operator can refine on the review screen).
    pub fn apply_raw_answer(&mut self, slot: IntakeSlot, raw: &str) {
        let raw = raw.trim();
        if raw.is_empty() {
            return;
        }
        match slot {
            IntakeSlot::Charter => self.charter = Some(raw.to_string()),
            IntakeSlot::Persona => self.persona = Some(raw.to_string()),
            IntakeSlot::Responsibilities => extend_trimmed(&mut self.responsibilities, split_lines(raw)),
            IntakeSlot::Cadence => self.cadence = Some(raw.to_string()),
            IntakeSlot::Exceptions => extend_trimmed(&mut self.exceptions, split_lines(raw)),
            IntakeSlot::SuccessCriteria => {
                for line in split_lines(raw) {
                    self.success_criteria.push(wizard_criterion(&line));
                }
            }
        }
    }

    /// The next step the conversation should take. Pure + deterministic: ask the
    /// next unfilled slot's wizard question, else present the assembled spec for
    /// confirmation, else (post-confirm) hand it off as `Done`.
    #[must_use]
    pub fn next_step(&self) -> NextStep {
        match self.first_unfilled_slot() {
            Some(slot) => NextStep::Ask {
                slot,
                question: slot.wizard_question().to_string(),
            },
            None => {
                let spec = Box::new(self.assemble_spec());
                if self.confirmed {
                    NextStep::Done(spec)
                } else {
                    NextStep::Confirm(spec)
                }
            }
        }
    }

    /// Assemble a valid [`CapabilitySpec`] from the captured slots, reusing the
    /// EXISTING [`CapabilitySpec::from_broad_draft`] constructor + the
    /// deterministic archetype seed for the capability set. The captured intake
    /// slots (responsibilities / cadence / exceptions / success_criteria) are
    /// overlaid on top. No provider call — this is the fail-safe path.
    #[must_use]
    pub fn assemble_spec(&self) -> CapabilitySpec {
        let charter = self.charter.clone().unwrap_or_default();
        let persona = name_to_persona_slug(self.persona.as_deref().unwrap_or("worker"));
        let archetype = classify_charter(&charter).unwrap_or(DEFAULT_ARCHETYPE);
        let draft = BroadRoleDraft {
            charter,
            persona,
            persona_description: String::new(),
            action_vocabulary: seed_action_names(archetype),
            success_metrics: self
                .success_criteria
                .iter()
                .map(|c| c.name.clone())
                .collect(),
            onboarding_script: String::new(),
        };
        let mut spec = CapabilitySpec::from_broad_draft(draft, archetype);
        spec.responsibilities = self.responsibilities.clone();
        spec.cadence = self.cadence.clone();
        spec.exceptions = self.exceptions.clone();
        spec.success_criteria = self.success_criteria.clone();
        spec
    }
}

/// The deterministic seed action names for an archetype, reused from the
/// existing archetype-metadata surface so the conversation path declares the
/// SAME capabilities the one-shot archetype path would. Falls back to a minimal
/// safe trio if the metadata is ever empty (so `materialize_spec`, which
/// requires ≥1 capability, never fails).
fn seed_action_names(archetype: Archetype) -> Vec<String> {
    let names: Vec<String> = crate::archetype_meta::show_archetype(archetype.slug())
        .map(|m| m.seed_actions.into_iter().map(|a| a.name).collect())
        .unwrap_or_default();
    if names.is_empty() {
        vec![
            "draft_message".to_string(),
            "summarize_update".to_string(),
            "flag_for_review".to_string(),
        ]
    } else {
        names
    }
}

/// Build a captured-but-untyped success criterion from a wizard answer line: a
/// count metric with a `>= 1` floor. Honest about being a placeholder target the
/// operator refines on the review screen.
fn wizard_criterion(name: &str) -> SuccessCriterion {
    SuccessCriterion {
        name: name.trim().to_string(),
        kind: KpiKind::Count,
        target: 1.0,
        comparator: KpiComparator::Gte,
    }
}

/// Split a multi-line answer into trimmed, non-empty lines.
fn split_lines(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// Append trimmed, non-blank items to `target`.
fn extend_trimmed(target: &mut Vec<String>, items: Vec<String>) {
    for item in items {
        let trimmed = item.trim();
        if !trimmed.is_empty() {
            target.push(trimmed.to_string());
        }
    }
}

/// `Some(trimmed)` when the option holds non-blank text, else `None`.
fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
#[path = "intake_machine_tests.rs"]
mod tests;
