//! Stream `claude-genesis-charter-conversation` — Q&A walkthrough
//! that replaces "type a markdown charter from scratch" with a
//! conversational prompt sequence.
//!
//! This module is pure data: the [`CharterDialogue`] struct (the
//! captured answers), [`QUESTIONS`] (the Q&A template), and
//! [`dialogue_to_charter`] (the renderer that turns a dialogue
//! into the markdown charter the existing `plan` pipeline already
//! consumes). The cli-side prompt loop lives in
//! `argo-cli::handlers::genesis_interactive*`.
//!
//! The Q&A is intentionally short (6 questions) — the brief
//! example shows operators expect a 5-prompt walkthrough, not a
//! 30-question survey. Anything richer should land in a follow-up
//! brief.

use serde::{Deserialize, Serialize};

/// One captured Q&A round. The cli's interactive loop assembles
/// these into a [`CharterDialogue`]; the YAML loader for
/// `--non-interactive` deserialises the same shape directly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharterDialogue {
    /// e.g. `"marketing intern"`.
    pub role: String,
    /// e.g. `"Maya"` — also drives the persona slug.
    pub name: String,
    /// Optional. e.g. `"B2B SaaS"`. Empty string = unset.
    #[serde(default)]
    pub industry: String,
    /// 3-5 plain-English duty descriptions. The renderer numbers
    /// them in the rendered charter so the LLM refinement pass
    /// can latch onto the structure.
    pub duties: Vec<String>,
    /// e.g. `["slack-dm", "email"]` — labels written verbatim
    /// into the charter so the operator sees the exact strings
    /// the gateway will receive.
    pub channels: Vec<String>,
    /// Per-day USD cap. Numeric — the renderer formats with two
    /// decimals.
    pub spend_cap_per_day_usd: f64,
}

impl CharterDialogue {
    /// Validate that the captured answers form a charter the
    /// existing `argo-genesis::plan` pipeline can consume. Catches
    /// problems BEFORE we render so the operator sees a clear
    /// "duties is empty" error rather than `pipeline: charter is
    /// empty`.
    pub fn validate(&self) -> Result<(), DialogueError> {
        if self.role.trim().is_empty() {
            return Err(DialogueError::MissingField("role"));
        }
        if self.name.trim().is_empty() {
            return Err(DialogueError::MissingField("name"));
        }
        if self.duties.iter().all(|d| d.trim().is_empty()) {
            return Err(DialogueError::MissingField("duties"));
        }
        if self.channels.iter().all(|c| c.trim().is_empty()) {
            return Err(DialogueError::MissingField("channels"));
        }
        if !self.spend_cap_per_day_usd.is_finite() || self.spend_cap_per_day_usd <= 0.0 {
            return Err(DialogueError::InvalidSpendCap(
                self.spend_cap_per_day_usd,
            ));
        }
        Ok(())
    }
}

/// Why a [`CharterDialogue`] is not yet ready to render.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DialogueError {
    /// A required field was empty after trim.
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    /// The spend cap is non-positive or non-finite.
    #[error("invalid spend cap: {0} (must be > 0)")]
    InvalidSpendCap(f64),
}

/// One Q&A prompt. The [`QUESTIONS`] table drives the cli's
/// interactive loop; tests assert the renderer maps each key to
/// the right field on [`CharterDialogue`].
#[derive(Debug, Clone, Copy)]
pub struct Question {
    /// Stable identifier — also the YAML key under
    /// `--non-interactive`.
    pub key: &'static str,
    /// What the operator sees on stdout.
    pub prompt: &'static str,
    /// `true` for required fields (the loop refuses to advance
    /// past an empty answer).
    pub required: bool,
    /// `true` for fields that accept many answers (the loop reads
    /// until a blank line).
    pub multi: bool,
}

/// Canonical Q&A sequence. The order matches the brief's example.
pub const QUESTIONS: &[Question] = &[
    Question {
        key: "role",
        prompt: "What role should this worker play? (e.g. marketing intern, customer support, sales SDR)",
        required: true,
        multi: false,
    },
    Question {
        key: "name",
        prompt: "What's their name? (one word; lower-case becomes the persona slug)",
        required: true,
        multi: false,
    },
    Question {
        key: "industry",
        prompt: "Industry? (e.g. B2B SaaS, e-commerce, healthcare — optional, hit Enter to skip)",
        required: false,
        multi: false,
    },
    Question {
        key: "duties",
        prompt: "What 3-5 main duties? (one per line; empty line ends the list)",
        required: true,
        multi: true,
    },
    Question {
        key: "channels",
        prompt: "What channels for approvals? (comma-separated, e.g. slack-dm,email)",
        required: true,
        multi: false,
    },
    Question {
        key: "spend_cap",
        prompt: "What spend cap per day? (USD, numeric — e.g. 50)",
        required: true,
        multi: false,
    },
];

/// Render a [`CharterDialogue`] into the plain-English charter
/// the existing `argo-genesis::plan` pipeline already consumes.
/// The format intentionally reads like an operator-typed charter
/// so the deterministic archetype classifier latches onto the
/// same keywords it does for hand-written input.
#[must_use]
pub fn dialogue_to_charter(d: &CharterDialogue) -> String {
    let mut s = String::new();
    s.push_str("Be my ");
    s.push_str(d.role.trim());
    s.push_str(", named ");
    s.push_str(d.name.trim());
    if !d.industry.trim().is_empty() {
        s.push_str(", working in ");
        s.push_str(d.industry.trim());
    }
    s.push_str(".\n\nMain duties:\n");
    for (i, duty) in d
        .duties
        .iter()
        .filter(|x| !x.trim().is_empty())
        .enumerate()
    {
        s.push_str(&format!("{n}. {duty}\n", n = i + 1, duty = duty.trim()));
    }
    let active_channels: Vec<&str> = d
        .channels
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .collect();
    if !active_channels.is_empty() {
        s.push_str("\nFor approvals, use these channels: ");
        s.push_str(&active_channels.join(", "));
        s.push('.');
    }
    s.push_str(&format!(
        "\nDaily spend cap: ${cap:.2} USD.",
        cap = d.spend_cap_per_day_usd
    ));
    s
}

/// Sanitise a person's name into a persona slug Genesis accepts
/// (alphanumeric + `-` / `_`). The `name` answer in the dialogue
/// is conversational ("Maya"); the slug is what ends up in
/// filenames + the worker_id.
#[must_use]
pub fn name_to_persona_slug(name: &str) -> String {
    let mut out = String::new();
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == ' ' || ch == '-' || ch == '_' {
            out.push('-');
        }
        // every other char is dropped so the slug stays clean
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        // Fallback: never produce an empty persona slug — the
        // upstream pipeline rejects those.
        out.push_str("worker");
    }
    out
}

#[cfg(test)]
#[path = "charter_dialogue_tests.rs"]
mod tests;
