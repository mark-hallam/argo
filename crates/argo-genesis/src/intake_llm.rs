//! LLM enrichment for the conversational intake (DEC-039 "Talk it through").
//!
//! Two bounded, independent calls per turn, both degrade-fail-safe to `None`:
//!
//! 1. [`propose_question`] — given the captured state + the next unfilled slot,
//!    propose a warm, specific next question. On ANY transport/empty result the
//!    caller falls back to [`crate::intake_machine::IntakeSlot::wizard_question`].
//! 2. [`extract_delta`] — given the operator's latest answer + which slot is
//!    being filled, extract a typed [`SlotDelta`] (JSON). On ANY transport/parse
//!    failure the caller falls back to
//!    [`crate::intake_machine::IntakeState::apply_raw_answer`].
//!
//! Pure half — NO transport edge (mirrors [`crate::author`]). The gateway wraps
//! the shared `complete_once` dispatcher behind [`IntakeClient`]; tests inject a
//! deterministic mock. Both calls are intentionally small so they fit any
//! token cap the provider path enforces.

use async_trait::async_trait;

use argo_types::ArgoResult;

use crate::intake_machine::{IntakeSlot, IntakeState, SlotDelta};

/// Narrow provider trait — same shape as [`crate::author::AuthorClient`]. The
/// gateway implements it over `complete_once`; tests inject a stub.
/// Implementations MUST use a low temperature so the same turn renders stably.
#[async_trait]
pub trait IntakeClient: Send + Sync {
    /// Send one bounded prompt; return the raw model response text.
    async fn complete(&self, prompt: &str) -> ArgoResult<String>;
}

/// A compact, capped transcript rendering for prompt context — at most the last
/// `MAX_CONTEXT_TURNS` turns so the prompt stays small regardless of dialogue
/// length.
const MAX_CONTEXT_TURNS: usize = 8;

/// Render the tail of the transcript as `Assistant:` / `Operator:` lines for
/// prompt context.
fn render_context(state: &IntakeState) -> String {
    let start = state.transcript.len().saturating_sub(MAX_CONTEXT_TURNS);
    state.transcript[start..]
        .iter()
        .map(|t| {
            let who = match t.speaker {
                crate::intake_machine::Speaker::Assistant => "Assistant",
                crate::intake_machine::Speaker::Operator => "Operator",
            };
            format!("{who}: {}", t.text.trim())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build the next-question prompt for `slot`. Asks for ONE plain question and
/// nothing else, anchored on what the operator already said.
#[must_use]
pub fn build_question_prompt(state: &IntakeState, slot: IntakeSlot) -> String {
    let context = render_context(state);
    let role = state.charter.as_deref().unwrap_or("(not described yet)");
    format!(
        "You are SKYCOT's hiring interviewer. You are helping an operator describe \
         an AI teammate they want to hire, ONE topic at a time. Ask a single, warm, \
         specific question to learn about: {topic}. Keep it to one sentence. Do NOT \
         number it, do NOT add preamble, do NOT ask about anything other than this \
         topic. Reply with ONLY the question text.\n\n\
         Role so far: {role}\n\
         Conversation so far:\n{context}\n\n\
         Your next question (about {topic}):",
        topic = slot_topic(slot),
    )
}

/// A plain-English description of what a slot captures, for the prompt.
fn slot_topic(slot: IntakeSlot) -> &'static str {
    match slot {
        IntakeSlot::Charter => "the role and what it should take off the operator's plate",
        IntakeSlot::Persona => "what to name the teammate",
        IntakeSlot::Responsibilities => "the teammate's main day-to-day responsibilities",
        IntakeSlot::Cadence => "when the teammate should work (a schedule or a trigger)",
        IntakeSlot::Exceptions => "what the teammate must never do alone or must escalate",
        IntakeSlot::SuccessCriteria => "how success is measured for the teammate",
        IntakeSlot::Access => {
            "what systems or accounts the teammate will need access to (draft-only at birth)"
        }
        IntakeSlot::Escalation => {
            "when the teammate should stop and escalate — after how many failures, and to whom"
        }
    }
}

/// Propose the next question via the provider. Degrade-fail-safe: `None` on any
/// transport error OR an empty/whitespace response, so the caller uses the
/// deterministic wizard question.
pub async fn propose_question(
    state: &IntakeState,
    slot: IntakeSlot,
    client: &dyn IntakeClient,
) -> Option<String> {
    let prompt = build_question_prompt(state, slot);
    let raw = client.complete(&prompt).await.ok()?;
    let cleaned = clean_question(&raw);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

/// Strip surrounding quotes/whitespace and collapse a multi-line model answer to
/// its first non-empty line (the question), so a chatty model still yields one
/// clean question.
fn clean_question(raw: &str) -> String {
    raw.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .trim()
        .to_string()
}

/// Build the answer→slot-delta extraction prompt. Asks for ONE JSON object whose
/// keys are the [`SlotDelta`] field names; only the field(s) the answer actually
/// supplies need be present.
#[must_use]
pub fn build_extract_prompt(slot: IntakeSlot, answer: &str) -> String {
    format!(
        "Extract structured intake data from an operator's answer about {topic}. \
         Reply with EXACTLY ONE JSON object and nothing else. Use ONLY these keys, \
         and ONLY those the answer actually provides:\n\
         {{\n  \
         \"charter\": <string: the role description, if given>,\n  \
         \"persona\": <string: the teammate's name, if given>,\n  \
         \"responsibilities\": [<strings: duties, if given>],\n  \
         \"cadence\": <string: schedule/trigger, if given>,\n  \
         \"exceptions\": [<strings: carve-outs to escalate, if given>],\n  \
         \"success_criteria\": [{{\"name\": <metric label>, \"kind\": \
         <\"percentage\"|\"duration_ms\"|\"count\"|\"ratio\"|\"score\">, \
         \"target\": <number>, \"comparator\": <\"gte\"|\"lte\"|\"eq\">}}],\n  \
         \"access_integrations\": [<strings: systems/accounts needed, if given>],\n  \
         \"escalation_failures_threshold\": <integer 1-100: escalate after N failures, if given>,\n  \
         \"escalation_to\": <string: who to escalate to, if given>\n\
         }}\n\n\
         The answer is about {topic}, so put its content in the matching field. \
         Do not invent values the operator did not state.\n\n\
         Operator's answer: {answer}",
        topic = slot_topic(slot),
    )
}

/// Extract a [`SlotDelta`] from the operator's `answer` via the provider.
/// Degrade-fail-safe: `None` on any transport error, no parseable JSON, or an
/// empty delta — the caller then fills the current slot from the raw answer
/// text via the deterministic wizard path.
pub async fn extract_delta(
    slot: IntakeSlot,
    answer: &str,
    client: &dyn IntakeClient,
) -> Option<SlotDelta> {
    let prompt = build_extract_prompt(slot, answer);
    let raw = client.complete(&prompt).await.ok()?;
    parse_delta(&raw)
}

/// Tolerant parse — reuses the shared balanced-object scanner; returns `None`
/// on no JSON / parse error / an all-empty delta.
fn parse_delta(raw: &str) -> Option<SlotDelta> {
    let json_text = crate::broad_role::first_balanced_object(raw)?;
    let delta: SlotDelta = serde_json::from_str(json_text).ok()?;
    if delta.is_empty() {
        None
    } else {
        Some(delta)
    }
}

#[cfg(test)]
#[path = "intake_llm_tests.rs"]
mod tests;
