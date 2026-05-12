//! Adaptive context truncation
//! (`mission-adaptive-context-truncation` / MISSION_BACKLOG §8 #248).
//!
//! Sister fallback to [`crate::context_compress`]. Compression is the
//! preferred path — an LLM summary buys headroom without losing the
//! shape of the conversation. Truncation is the deterministic
//! fallback that fires when:
//!
//! - the summariser itself failed (rate-limit, refusal, network), or
//! - compression already ran but a single turn is still too big to
//!   fit (e.g. one giant tool result that compression can't help
//!   with), or
//! - the operator disabled compression entirely.
//!
//! Unlike compression, truncation never asks the LLM. It drops older
//! turns FIFO, and as a last resort replaces giant individual turns
//! with the operator-readable `[...truncated XXX tokens of tool
//! output...]` marker so the provider transport accepts the call.
//!
//! ## Priority order
//!
//! Highest-priority (always kept) to lowest (dropped first):
//! 1. System prompt — never dropped.
//! 2. Last user turn — never dropped (the model is responding to it).
//! 3. Last assistant turn — preserves immediate context.
//! 4. `Tool`-role messages within the last N user-turn boundaries
//!    (default `N=3`) — load-bearing for tool-call chains.
//! 5. Older user/assistant turns — dropped FIFO.
//!
//! ## Hard-fail contract
//!
//! When even the minimum-must-keep set (system prompt + last user
//! turn) cannot fit after dropping and intra-turn truncation,
//! [`truncate_to_fit`] returns
//! [`ArgoError::ContextWindowExhausted`]. The caller MUST surface
//! this — silently accepting a malformed response is worse than
//! failing loudly.
//!
//! ## Audit row
//!
//! [`build_audit_payload`] returns the JSON body for the
//! `runtime.context_truncated` audit row. The constant
//! [`AUDIT_KIND_LABEL`] is duplicated here so this crate does not
//! need an `argo-audit` dependency (which would close a workspace
//! cycle via `argo-store → argo-runtime`). The shape is distinct
//! from `runtime.context_compressed` so operators can drill in to
//! see how often truncation kicks in versus compression.

use crate::context_compress::TokenCounter;
use argo_types::{ArgoError, ContentBlock, Message, MessageRole};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Stable `kind` label persisted in the `audit_record.kind` column
/// for truncation-pass rows. Wire equivalent of the matching
/// `AuditKind` variant in `argo-audit`; the constant is duplicated
/// here so this crate does not need an `argo-audit` dependency
/// (which would close a workspace cycle).
pub const AUDIT_KIND_LABEL: &str = "runtime.context_truncated";

/// Default number of trailing user-turn boundaries within which
/// `Tool`-role messages are kept verbatim. Three covers the typical
/// "agent calls a tool → reads its output → decides the next step"
/// loop without preserving tool spam from earlier in the session.
pub const DEFAULT_RECENT_TOOL_RESULT_TURNS: usize = 3;

/// Default intra-turn truncation threshold (% of window). When a
/// single message exceeds this fraction of the window, its content
/// is replaced with the truncation marker.
pub const DEFAULT_INTRA_TURN_THRESHOLD_PCT: u32 = 50;

/// Operator-tunable knobs for adaptive truncation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruncationConfig {
    /// Number of trailing user-turn boundaries within which
    /// `Tool`-role messages are protected. A value of zero disables
    /// the tool-result protection entirely; in that case only the
    /// system prompt and the last user/assistant turns are protected.
    pub recent_tool_result_turns: usize,
    /// Intra-turn truncation fires when a single message's token count
    /// exceeds this percentage of the window. Default `50` (i.e. a
    /// turn that would consume half the window or more is replaced
    /// with the truncation marker).
    pub intra_turn_threshold_pct: u32,
}

impl Default for TruncationConfig {
    fn default() -> Self {
        Self {
            recent_tool_result_turns: DEFAULT_RECENT_TOOL_RESULT_TURNS,
            intra_turn_threshold_pct: DEFAULT_INTRA_TURN_THRESHOLD_PCT,
        }
    }
}

/// Outcome of a successful [`truncate_to_fit`] call.
#[derive(Debug, Clone)]
pub struct TruncationResult {
    /// New message sequence to feed the model for the next turn.
    pub view: Vec<Message>,
    /// Token count of the input slice before truncation.
    pub pre_token_count: usize,
    /// Token count of [`Self::view`].
    pub post_token_count: usize,
    /// Number of input messages dropped FIFO (does NOT include
    /// intra-turn truncations — those are counted separately).
    pub turns_dropped: usize,
    /// Number of messages that survived FIFO drop but had their
    /// content replaced with the truncation marker because the
    /// message alone exceeded the intra-turn threshold.
    pub intra_turn_truncations: usize,
}

impl TruncationResult {
    /// `true` when [`truncate_to_fit`] actually mutated the input
    /// (dropped a turn or intra-truncated one).
    #[must_use]
    pub fn truncated(&self) -> bool {
        self.turns_dropped > 0 || self.intra_turn_truncations > 0
    }
}

/// Truncate `messages` so the total fits in `max_tokens`, using
/// [`TruncationConfig::default()`].
///
/// See module docs for the priority order. Returns
/// [`ArgoError::ContextWindowExhausted`] when even the minimum-must-
/// keep set cannot fit after exhausting all truncation options.
///
/// # Errors
/// - [`ArgoError::ContextWindowExhausted`] — see module docs.
pub fn truncate_to_fit(
    messages: &[Message],
    max_tokens: usize,
    tokenizer: &dyn TokenCounter,
) -> Result<TruncationResult, ArgoError> {
    truncate_to_fit_with_config(
        messages,
        max_tokens,
        &TruncationConfig::default(),
        tokenizer,
    )
}

/// Truncate `messages` so the total fits in `max_tokens` using the
/// supplied [`TruncationConfig`]. Prefer [`truncate_to_fit`] unless
/// you need to tune protection or intra-turn-threshold knobs.
///
/// # Errors
/// - [`ArgoError::ContextWindowExhausted`] — see module docs.
pub fn truncate_to_fit_with_config(
    messages: &[Message],
    max_tokens: usize,
    config: &TruncationConfig,
    tokenizer: &dyn TokenCounter,
) -> Result<TruncationResult, ArgoError> {
    let pre_token_count: usize = messages.iter().map(|m| tokenizer.count_message(m)).sum();

    if pre_token_count <= max_tokens {
        return Ok(TruncationResult {
            view: messages.to_vec(),
            pre_token_count,
            post_token_count: pre_token_count,
            turns_dropped: 0,
            intra_turn_truncations: 0,
        });
    }

    let protected = protected_indices(messages, config.recent_tool_result_turns);
    let mut working: Vec<Option<Message>> = messages.iter().cloned().map(Some).collect();
    let mut current_total = pre_token_count;
    let mut turns_dropped = 0usize;
    let mut intra_turn_truncations = 0usize;

    // Phase 1: FIFO drop non-protected messages until the running
    // total fits.
    for (idx, slot) in working.iter_mut().enumerate() {
        if current_total <= max_tokens {
            break;
        }
        if protected.contains(&idx) {
            continue;
        }
        if let Some(msg) = slot.take() {
            current_total = current_total.saturating_sub(tokenizer.count_message(&msg));
            turns_dropped = turns_dropped.saturating_add(1);
        }
    }

    // Phase 2: intra-turn truncate any remaining message that on its
    // own exceeds the configured threshold. Applies to protected
    // messages too (we'd rather clamp the last assistant turn than
    // hard-fail).
    if current_total > max_tokens {
        let pct_usize = config.intra_turn_threshold_pct as usize;
        let threshold = max_tokens.saturating_mul(pct_usize) / 100;
        for slot in &mut working {
            if current_total <= max_tokens {
                break;
            }
            let Some(msg) = slot else { continue };
            let count = tokenizer.count_message(msg);
            if count > threshold {
                let replaced = intra_truncate_message(msg, count);
                let new_count = tokenizer.count_message(&replaced);
                *slot = Some(replaced);
                current_total = current_total.saturating_sub(count).saturating_add(new_count);
                intra_turn_truncations = intra_turn_truncations.saturating_add(1);
            }
        }
    }

    let view: Vec<Message> = working.into_iter().flatten().collect();
    let post_token_count: usize = view.iter().map(|m| tokenizer.count_message(m)).sum();

    if post_token_count > max_tokens {
        let minimum_tokens = minimum_token_count(messages, tokenizer);
        return Err(ArgoError::ContextWindowExhausted {
            minimum_tokens,
            window_tokens: max_tokens,
        });
    }

    Ok(TruncationResult {
        view,
        pre_token_count,
        post_token_count,
        turns_dropped,
        intra_turn_truncations,
    })
}

/// JSON body for the `runtime.context_truncated` audit row.
///
/// Shape: `{ pre_token_count, post_token_count, turns_dropped,
/// intra_turn_truncations }`. Callers pass the result to
/// `AuditSink::write` together with the constant
/// [`AUDIT_KIND_LABEL`].
#[must_use]
pub fn build_audit_payload(result: &TruncationResult) -> serde_json::Value {
    serde_json::json!({
        "pre_token_count": result.pre_token_count,
        "post_token_count": result.post_token_count,
        "turns_dropped": result.turns_dropped,
        "intra_turn_truncations": result.intra_turn_truncations,
    })
}

/// Compute the set of message indices that MUST be kept in the view.
///
/// Public for white-box test access; downstream callers should rely
/// on [`truncate_to_fit`] / [`truncate_to_fit_with_config`] for the
/// composed behaviour.
#[must_use]
pub fn protected_indices(messages: &[Message], n_tool_turns: usize) -> HashSet<usize> {
    let mut protected = HashSet::new();

    for (i, m) in messages.iter().enumerate() {
        if matches!(m.role, MessageRole::System) {
            protected.insert(i);
        }
    }

    if let Some((i, _)) = messages
        .iter()
        .enumerate()
        .rev()
        .find(|(_, m)| matches!(m.role, MessageRole::User))
    {
        protected.insert(i);
    }

    if let Some((i, _)) = messages
        .iter()
        .enumerate()
        .rev()
        .find(|(_, m)| matches!(m.role, MessageRole::Assistant))
    {
        protected.insert(i);
    }

    if n_tool_turns > 0 {
        let user_indices: Vec<usize> = messages
            .iter()
            .enumerate()
            .filter_map(|(i, m)| matches!(m.role, MessageRole::User).then_some(i))
            .collect();
        let start_user_idx = user_indices.len().saturating_sub(n_tool_turns);
        let boundary_idx = user_indices.get(start_user_idx).copied().unwrap_or(0);
        for (i, m) in messages.iter().enumerate().skip(boundary_idx) {
            if matches!(m.role, MessageRole::Tool) {
                protected.insert(i);
            }
        }
    }

    protected
}

/// Replace `msg`'s content with the truncation marker, preserving
/// every other field (id, role, session, tool_call_id, …).
fn intra_truncate_message(msg: &Message, original_tokens: usize) -> Message {
    let marker = format!("[...truncated {original_tokens} tokens of tool output...]");
    let mut new_msg = msg.clone();
    new_msg.content = vec![ContentBlock::Text { text: marker }];
    new_msg
}

/// Token count of the minimum-must-keep set: every `System` message
/// plus the last `User` message. Used by the
/// [`ArgoError::ContextWindowExhausted`] payload so the operator can
/// see how much headroom is missing.
fn minimum_token_count(messages: &[Message], tokenizer: &dyn TokenCounter) -> usize {
    let mut count = 0usize;
    for m in messages {
        if matches!(m.role, MessageRole::System) {
            count = count.saturating_add(tokenizer.count_message(m));
        }
    }
    if let Some(last_user) = messages
        .iter()
        .rev()
        .find(|m| matches!(m.role, MessageRole::User))
    {
        count = count.saturating_add(tokenizer.count_message(last_user));
    }
    count
}

#[cfg(test)]
#[path = "context_truncate_tests.rs"]
mod context_truncate_tests;
