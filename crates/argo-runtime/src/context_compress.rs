//! Context-window-aware compression
//! (`mission-context-window-aware-compression` / MISSION_BACKLOG §8
//! #247 Amendment §5.8).
//!
//! Long-running sessions blow past the model's context window.
//! Today the runtime either truncates naively (drops oldest turns)
//! or fails. This module is the seam that fixes that:
//!
//! - [`should_compress`] decides whether the running token count has
//!   crossed the operator-tuned trigger threshold (default 80% of
//!   the model's context window).
//! - [`compress`] partitions the message history into a `system`
//!   prefix, an `older` slice that gets summarised into one
//!   synthetic `assistant` turn (`<summary>…</summary>`), and a
//!   `recent` slice that is kept verbatim (default last 10 user
//!   turns). The resulting view is a valid alternating message
//!   sequence the provider transport accepts without modification.
//! - [`build_audit_payload`] returns the JSON body for the
//!   `runtime.context_compressed` audit row. The constant
//!   [`AUDIT_KIND_LABEL`] is duplicated here so this crate does not
//!   need to take an `argo-audit` dependency (which would close a
//!   workspace cycle via `argo-store → argo-runtime`); the matching
//!   `AuditKind::RuntimeContextCompressed` enum variant lands in a
//!   follow-up wiring brief along with the supervisor call site.
//!
//! ## What this is NOT
//!
//! Compression is a *view*: the DB representation of
//! `session.messages` is left untouched. Replay / inspection can
//! always recover the full chain. Cross-turn semantic deduplication,
//! image compression in vision-enabled models, and per-provider
//! tokenizer integration are explicit non-goals — the brief assumes
//! a [`TokenCounter`] is plumbed by callers (precursor briefs file
//! the real tokenizer crates).
//!
//! ## Provenance
//!
//! `runtime.context_compressed` payload shape:
//! `{ pre_token_count, post_token_count, summary_length_tokens,
//! turns_compressed }`. Distinct from `turn_*` events (per-turn
//! lifecycle) — one row per compression pass.

use argo_types::{ContentBlock, Message, MessageId, MessageRole, SessionId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable `kind` label persisted in the `audit_record.kind` column
/// for compression-pass rows. The matching `AuditKind` enum variant
/// is added in the wiring brief; the constant is duplicated here so
/// this crate does not need an `argo-audit` dependency.
pub const AUDIT_KIND_LABEL: &str = "runtime.context_compressed";

/// Default trigger threshold (% of model context window).
pub const DEFAULT_TRIGGER_PCT: u32 = 80;

/// Default number of trailing user turns kept verbatim.
pub const DEFAULT_KEEP_LAST_TURNS: usize = 10;

/// Default cap on summary length in tokens.
pub const DEFAULT_SUMMARY_MAX_TOKENS: usize = 500;

/// Operator-tunable knobs read off `[runtime.context_compress]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Master enable. When `false`, [`should_compress`] always
    /// returns `false` and [`compress`] should not be invoked.
    pub enabled: bool,
    /// Compression fires when `token_count >= trigger_pct% *
    /// window_tokens`. Operators who want earlier compression set
    /// this lower (e.g. 70%).
    pub trigger_pct: u32,
    /// Number of trailing user turns to keep verbatim. Must be > 0
    /// — a value of zero would compress the entire history into one
    /// summary turn, which loses the most-recent context the model
    /// is mid-decision on.
    pub keep_last_turns: usize,
    /// Cap on the generated summary's token count. The summarizer
    /// implementation is responsible for honoring this; this module
    /// passes the cap through to [`Summarizer::summarize`] but does
    /// not enforce it post-hoc (the summarizer owns the call).
    pub summary_max_tokens: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trigger_pct: DEFAULT_TRIGGER_PCT,
            keep_last_turns: DEFAULT_KEEP_LAST_TURNS,
            summary_max_tokens: DEFAULT_SUMMARY_MAX_TOKENS,
        }
    }
}

/// Output of a [`Summarizer::summarize`] call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    /// Plain-text summary body. Wrapped in `<summary>…</summary>`
    /// when injected into the compressed envelope.
    pub text: String,
    /// Token count of [`Self::text`] as reported by the same
    /// tokenizer used for the surrounding messages.
    pub token_count: usize,
}

/// Outcome of a successful [`compress`] call.
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// New message sequence to feed the model for the next turn.
    /// Shape: `[system_prefix…, synthetic_summary?, recent…]`.
    pub view: Vec<Message>,
    /// Token count of the input message slice before compression.
    pub pre_token_count: usize,
    /// Token count of [`Self::view`].
    pub post_token_count: usize,
    /// Number of input messages collapsed into the summary turn.
    /// Zero when the input had fewer than `keep_last_turns` user
    /// turns and no compression occurred.
    pub turns_compressed: usize,
    /// Token count of the synthetic summary message (zero on a
    /// no-op compression).
    pub summary_length_tokens: usize,
}

impl CompressionResult {
    /// `true` when [`compress`] actually emitted a summary turn (vs
    /// returning the input verbatim because it was short enough).
    #[must_use]
    pub fn compressed(&self) -> bool {
        self.turns_compressed > 0
    }
}

/// Compression-pipeline errors.
#[derive(Debug, Error)]
pub enum CompressionError {
    /// The injected [`Summarizer`] returned an error. The carried
    /// string is the summariser's own error text.
    #[error("summarizer failed: {0}")]
    Summarizer(String),
    /// The supplied [`CompressionConfig`] is internally inconsistent
    /// (e.g. `keep_last_turns == 0`).
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

/// LLM-backed summary generator. Implementations call the same
/// model the session is running on with a focused prompt that biases
/// toward preserving load-bearing facts (decisions, tool outputs,
/// open questions, error messages) rather than producing a loose
/// narrative summary.
#[async_trait]
pub trait Summarizer: Send + Sync {
    /// Summarise `older_turns` into ≤ `max_tokens` tokens. The
    /// returned [`Summary::token_count`] should reflect what the
    /// caller's tokenizer would report for [`Summary::text`].
    async fn summarize(
        &self,
        older_turns: &[Message],
        max_tokens: usize,
    ) -> Result<Summary, CompressionError>;
}

/// Tokenizer abstraction. Implementations wrap a provider-specific
/// tokenizer (`claude-tokenizer` for Anthropic, `tiktoken-rs` for
/// OpenAI, etc.). Kept narrow so this module does not pick the
/// provider for the rest of the workspace.
pub trait TokenCounter: Send + Sync {
    /// Token count for one [`Message`]. Implementations sum content
    /// blocks + role overhead per the provider's accounting.
    fn count_message(&self, msg: &Message) -> usize;
    /// Token count for a raw string (used by the summariser path to
    /// re-count the wrapped summary turn after assembly).
    fn count_text(&self, text: &str) -> usize;
}

/// Decide whether [`compress`] should run on the next turn.
///
/// `window_tokens` is the model's context-window cap (e.g. 200_000
/// for Claude Sonnet 4.x); typically read off
/// `ModelSpec::context_window` at call time. A `window_tokens` of
/// `0` short-circuits to `false` — callers that don't know the
/// window are opting out, not opting in.
#[must_use]
pub fn should_compress(
    token_count: usize,
    window_tokens: usize,
    config: &CompressionConfig,
) -> bool {
    if !config.enabled || window_tokens == 0 {
        return false;
    }
    let pct = u64::from(config.trigger_pct);
    let window = window_tokens as u64;
    let threshold = pct.checked_mul(window).map_or(u64::MAX, |p| p / 100);
    (token_count as u64) >= threshold
}

/// Three-way partition of the input message slice.
///
/// - `system` — leading run of `MessageRole::System` messages,
///   kept verbatim.
/// - `older` — messages between `system` and `recent` (candidates
///   for summarisation).
/// - `recent` — last `keep_last_turns` user turns plus everything
///   that follows them, kept verbatim.
///
/// When the input contains fewer than `keep_last_turns` user
/// messages after the system prefix, `older` is empty and `recent`
/// holds the whole body — `compress` short-circuits in that case.
#[must_use]
pub fn partition(messages: &[Message], keep_last_turns: usize) -> Partitioned<'_> {
    let mut system_end = 0;
    while system_end < messages.len()
        && matches!(messages[system_end].role, MessageRole::System)
    {
        system_end += 1;
    }
    let body = &messages[system_end..];
    let mut boundary = 0;
    let mut user_count = 0;
    if keep_last_turns > 0 {
        for (idx, msg) in body.iter().enumerate().rev() {
            if matches!(msg.role, MessageRole::User) {
                user_count += 1;
                if user_count == keep_last_turns {
                    boundary = idx;
                    break;
                }
            }
        }
    }
    let (older, recent) = body.split_at(boundary);
    Partitioned {
        system: &messages[..system_end],
        older,
        recent,
    }
}

/// Borrowed three-way slice produced by [`partition`].
#[derive(Debug, Clone, Copy)]
pub struct Partitioned<'a> {
    /// Leading system-message run.
    pub system: &'a [Message],
    /// Messages eligible for summarisation.
    pub older: &'a [Message],
    /// Trailing messages kept verbatim.
    pub recent: &'a [Message],
}

/// Compress `messages` into a `[system…, synthetic_summary,
/// recent…]` envelope. Returns the input verbatim (with
/// `turns_compressed = 0`) when the partition leaves nothing to
/// compress.
///
/// # Errors
/// - [`CompressionError::InvalidConfig`] when `config.keep_last_turns
///   == 0` or `config.summary_max_tokens == 0`.
/// - [`CompressionError::Summarizer`] when the injected
///   [`Summarizer`] fails. Callers should surface the underlying
///   error; this module does not retry.
pub async fn compress(
    messages: &[Message],
    config: &CompressionConfig,
    counter: &dyn TokenCounter,
    summarizer: &dyn Summarizer,
) -> Result<CompressionResult, CompressionError> {
    if config.keep_last_turns == 0 {
        return Err(CompressionError::InvalidConfig(
            "keep_last_turns must be > 0".into(),
        ));
    }
    if config.summary_max_tokens == 0 {
        return Err(CompressionError::InvalidConfig(
            "summary_max_tokens must be > 0".into(),
        ));
    }
    let pre_token_count: usize = messages.iter().map(|m| counter.count_message(m)).sum();
    let part = partition(messages, config.keep_last_turns);

    if part.older.is_empty() {
        return Ok(CompressionResult {
            view: messages.to_vec(),
            pre_token_count,
            post_token_count: pre_token_count,
            turns_compressed: 0,
            summary_length_tokens: 0,
        });
    }

    let summary = summarizer
        .summarize(part.older, config.summary_max_tokens)
        .await?;
    let summary_msg = build_summary_message(&summary, part.older);

    let mut view = Vec::with_capacity(part.system.len() + 1 + part.recent.len());
    view.extend_from_slice(part.system);
    view.push(summary_msg);
    view.extend_from_slice(part.recent);

    let post_token_count: usize = view.iter().map(|m| counter.count_message(m)).sum();
    Ok(CompressionResult {
        view,
        pre_token_count,
        post_token_count,
        turns_compressed: part.older.len(),
        summary_length_tokens: summary.token_count,
    })
}

/// JSON body for the `runtime.context_compressed` audit row.
///
/// Shape: `{ pre_token_count, post_token_count,
/// summary_length_tokens, turns_compressed }`. Callers pass the
/// result to `AuditSink::write` together with the constant
/// [`AUDIT_KIND_LABEL`].
#[must_use]
pub fn build_audit_payload(result: &CompressionResult) -> serde_json::Value {
    serde_json::json!({
        "pre_token_count": result.pre_token_count,
        "post_token_count": result.post_token_count,
        "summary_length_tokens": result.summary_length_tokens,
        "turns_compressed": result.turns_compressed,
    })
}

fn build_summary_message(summary: &Summary, older: &[Message]) -> Message {
    let session_id = older
        .first()
        .map_or_else(SessionId::new, |m| m.session_id);
    Message {
        id: MessageId::new(),
        session_id,
        role: MessageRole::Assistant,
        content: vec![ContentBlock::Text {
            text: format!("<summary>{}</summary>", summary.text),
        }],
        tool_calls: vec![],
        tool_call_id: None,
        reasoning: None,
        created_at: chrono::Utc::now(),
        provider_meta: serde_json::json!({
            "synthetic": true,
            "synthetic_kind": "context_compress.summary",
            "turns_compressed": older.len(),
        }),
    }
}

#[cfg(test)]
#[path = "context_compress_tests.rs"]
mod context_compress_tests;
