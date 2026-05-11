//! Lightweight safety gate that runs at the tail of every Genesis
//! pipeline pass — archetype path and broad-role path alike.
//!
//! Mission `mission-genesis-non-archetype-roles`. The judge asks an
//! external model 3 short questions about the synthesised draft:
//!
//! 1. Does the role contain illegal or harmful instructions?
//! 2. Does the action vocabulary cross into restricted MCP surfaces
//!    (financial trades, medical advice without disclaimers, etc.)?
//! 3. Is the persona impersonating a real person?
//!
//! The model returns one of `Allow`, `AllowWithDisclaimer(reason)`,
//! or `Block(reason)`. On any judge transport / parse failure we
//! conservatively fall back to `Block` — never silently allow a
//! draft when the judge crashes.
//!
//! ## Sensible deviation from the brief (2026-05-11)
//!
//! The brief calls for "calling existing `argo-judge`". The real
//! `argo-judge` surface (`Judge::adjudicate` with a `Rubric`,
//! `JudgeSubject`, `JudgeEvidence`, full `JudgeRecord` return) is
//! heavyweight: it requires a `Transport`, a session id, and a
//! ledger persist hook — designed for skill-create/refinement
//! adjudication, not 3-question prompt verdicts.
//!
//! Rather than BLOCK the brief on a precursor "add generic-prompt
//! API to argo-judge", we mirror the [`crate::archetype::ProviderClient`]
//! pattern and expose a narrow [`GenesisJudgeClient`] trait here.
//! Callers can wrap a real `argo-judge` `Transport` in a few lines
//! when they wire production code; tests inject a mock. The
//! `argo-judge` surface is untouched.

use crate::broad_role::BroadRoleDraft;
use argo_types::ArgoResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Three-state safety verdict per the brief. Distinct from the
/// crate-level [`argo_types::JudgeVerdict`] (which models
/// `Success / Partial / Failure`) because Genesis cares about a
/// different operational distinction: allow / allow-with-disclaimer
/// / block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum JudgeVerdict {
    /// Draft is safe — pipeline continues unchanged.
    Allow,
    /// Draft is safe but a disclaimer must be prepended to the
    /// persona description before downstream code touches it.
    AllowWithDisclaimer { reason: String },
    /// Draft is unsafe — pipeline must error.
    Block { reason: String },
}

/// Narrow provider abstraction used by [`judge_genesis_draft`]. Same
/// pattern as [`crate::archetype::ProviderClient`] — keeps the judge
/// surface independent of any specific transport so production code
/// can adapt whatever HTTP client it already holds and tests can
/// inject a deterministic mock. Implementations MUST set temperature
/// low so the same draft renders the same verdict on every call.
#[async_trait]
pub trait GenesisJudgeClient: Send + Sync {
    /// Send a one-shot judge prompt and return the raw model
    /// response text. Implementations should aim for JSON-only
    /// output per the prompt contract; [`judge_genesis_draft`] is
    /// tolerant of preceding prose.
    async fn judge(&self, prompt: &str) -> ArgoResult<String>;
}

/// Minimal view onto a draft that the safety judge needs. Both the
/// archetype path (`GenesisPlan`) and the broad-role path
/// (`BroadRoleDraft`) build a `JudgeInput` from their own fields;
/// the judge prompt only sees this projection so the same fixed
/// prompt template works for either source.
#[derive(Debug, Clone)]
pub struct JudgeInput<'a> {
    pub charter: &'a str,
    pub persona_description: &'a str,
    pub action_vocabulary: Vec<String>,
}

impl<'a> JudgeInput<'a> {
    /// Project a [`BroadRoleDraft`] into a judge input.
    #[must_use]
    pub fn from_broad(draft: &'a BroadRoleDraft) -> Self {
        Self {
            charter: &draft.charter,
            persona_description: &draft.persona_description,
            action_vocabulary: draft.action_vocabulary.clone(),
        }
    }
}

/// Build the fixed 3-question safety prompt. Short and stable on
/// purpose — long judges drift across model upgrades, the brief
/// explicitly calls out ≤300 chars for the question block. The
/// response contract is a single JSON object so parsing is
/// deterministic.
#[must_use]
pub fn build_judge_prompt(input: &JudgeInput<'_>) -> String {
    let actions = input.action_vocabulary.join(", ");
    format!(
        "You are a safety judge for a synthesised AI worker role. \
         Answer THREE questions about the draft below and reply with ONE \
         JSON object: {{\"verdict\": \"allow|allow_with_disclaimer|block\", \
         \"reason\": \"<short reason if not allow>\"}}.\n\n\
         Q1: Does the role contain illegal or harmful instructions?\n\
         Q2: Does the action vocabulary cross into restricted MCP surfaces \
         (financial trades, medical advice without disclaimers, legal advice, etc.)?\n\
         Q3: Is the persona impersonating a real person?\n\n\
         If any answer is yes and the risk is severe, reply \"block\".\n\
         If any answer is yes but a clear disclaimer addresses the risk, \
         reply \"allow_with_disclaimer\" with the disclaimer as reason.\n\
         Otherwise reply \"allow\".\n\n\
         Charter: {charter}\n\
         Persona description: {persona}\n\
         Action vocabulary: {actions}",
        charter = input.charter,
        persona = input.persona_description,
    )
}

/// Run the safety judge against the supplied draft view. On ANY
/// judge transport / parse failure we conservatively return
/// `Block` — the brief's "judge error → Block" rule is load-bearing
/// and tested explicitly in `genesis_judge_tests.rs`.
pub async fn judge_genesis_draft(
    input: &JudgeInput<'_>,
    client: &dyn GenesisJudgeClient,
) -> ArgoResult<JudgeVerdict> {
    let prompt = build_judge_prompt(input);
    match client.judge(&prompt).await {
        Ok(raw) => Ok(parse_verdict(&raw).unwrap_or_else(|reason| JudgeVerdict::Block { reason })),
        Err(e) => Ok(JudgeVerdict::Block {
            reason: format!("safety judge transport error: {e}"),
        }),
    }
}

fn parse_verdict(raw: &str) -> Result<JudgeVerdict, String> {
    let json_text = crate::broad_role::first_balanced_object(raw)
        .ok_or_else(|| "judge response: no JSON object".to_string())?;
    let value: serde_json::Value = serde_json::from_str(json_text)
        .map_err(|e| format!("judge response JSON parse: {e}"))?;
    let verdict_str = value
        .get("verdict")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let reason = value
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    match verdict_str.as_str() {
        "allow" => Ok(JudgeVerdict::Allow),
        "allow_with_disclaimer" => Ok(JudgeVerdict::AllowWithDisclaimer {
            reason: if reason.is_empty() {
                "disclaimer required".to_string()
            } else {
                reason
            },
        }),
        "block" => Ok(JudgeVerdict::Block {
            reason: if reason.is_empty() {
                "blocked by safety judge".to_string()
            } else {
                reason
            },
        }),
        other => Err(format!("unknown verdict '{other}'")),
    }
}

#[cfg(test)]
#[path = "genesis_judge_tests.rs"]
mod tests;
