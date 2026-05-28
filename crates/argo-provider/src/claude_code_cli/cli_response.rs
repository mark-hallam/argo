//! Typed structs for the `claude --print --output-format json` stdout.
//!
//! Real CLI output (Claude Code 2.x) looks like:
//!
//! ```json
//! {
//!   "type": "result",
//!   "subtype": "success",
//!   "is_error": false,
//!   "result": "hello",
//!   "stop_reason": "end_turn",
//!   "session_id": "...",
//!   "duration_ms": 3309,
//!   "usage": {
//!     "input_tokens": 6,
//!     "output_tokens": 6,
//!     "cache_read_input_tokens": 23529,
//!     "cache_creation_input_tokens": 30299
//!   },
//!   "modelUsage": {
//!     "claude-opus-4-7[1m]": { "inputTokens": 6, "outputTokens": 6, ... }
//!   }
//! }
//! ```
//!
//! The brief's reference shape (`assistant_text` + `model`) is accepted as
//! an alias so a mock binary can emit the simpler form for tests. The
//! `result` field is the canonical assistant-text source.
use serde::Deserialize;
use std::collections::BTreeMap;

/// Parsed top-level JSON from one CLI invocation.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct ClaudeCliJsonResponse {
    /// Canonical assistant text — `result` in production output.
    /// `assistant_text` is accepted as an alias for the brief's reference
    /// shape so the mock-binary tests can emit the simpler form.
    #[serde(default, alias = "assistant_text")]
    pub result: Option<String>,
    /// Whether the CLI reports an error (`is_error: true` overrides
    /// `result` content).
    #[serde(default)]
    pub is_error: bool,
    /// Optional model echo (mock binaries emit this directly; production
    /// CLI carries it under `modelUsage` instead — see `model_usage`).
    #[serde(default)]
    pub model: Option<String>,
    /// Stop reason (`end_turn`, `tool_use`, `max_tokens`, ...). When
    /// absent the driver defaults to `EndTurn`.
    #[serde(default)]
    pub stop_reason: Option<String>,
    /// Token usage. Absent when the CLI errors out before counting.
    #[serde(default)]
    pub usage: Option<ClaudeCliUsage>,
    /// Per-model usage map — production CLI populates this; mocks may
    /// omit it. When `model` is absent the driver picks the entry with
    /// the highest output-token count as the dominant model echo.
    #[serde(default, rename = "modelUsage")]
    pub model_usage: Option<BTreeMap<String, ClaudeCliModelUsage>>,
}

/// Token usage block (the brief-spec subset; production CLI adds cache
/// fields that we deliberately ignore for v1).
#[derive(Debug, Clone, Deserialize)]
pub(super) struct ClaudeCliUsage {
    /// Input tokens (one turn).
    #[serde(default)]
    pub input_tokens: Option<u32>,
    /// Output tokens (one turn).
    #[serde(default)]
    pub output_tokens: Option<u32>,
}

/// One entry in the production CLI's `modelUsage` map — `inputTokens` /
/// `outputTokens` are camelCase per the CLI's own JSON shape.
#[derive(Debug, Clone, Deserialize)]
pub(super) struct ClaudeCliModelUsage {
    /// Output tokens for this model in this invocation. Used to pick the
    /// dominant model when the top-level `model` echo is absent.
    #[serde(default, rename = "outputTokens")]
    pub output_tokens: Option<u32>,
}

impl ClaudeCliJsonResponse {
    /// Pick a model id — top-level `model` first, otherwise the entry in
    /// `model_usage` with the highest output-token count.
    pub(super) fn dominant_model(&self) -> Option<String> {
        if let Some(m) = self.model.clone() {
            return Some(m);
        }
        let map = self.model_usage.as_ref()?;
        map.iter()
            .max_by_key(|(_id, usage)| usage.output_tokens.unwrap_or(0))
            .map(|(id, _)| id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_brief_reference_shape() {
        let j = r#"{"assistant_text":"hello","model":"claude-sonnet-4-6","usage":{"input_tokens":10,"output_tokens":3}}"#;
        let parsed: ClaudeCliJsonResponse = serde_json::from_str(j).expect("parse");
        assert_eq!(parsed.result.as_deref(), Some("hello"));
        assert_eq!(parsed.model.as_deref(), Some("claude-sonnet-4-6"));
        let usage = parsed.usage.expect("usage present");
        assert_eq!(usage.input_tokens, Some(10));
        assert_eq!(usage.output_tokens, Some(3));
        assert!(!parsed.is_error);
    }

    #[test]
    fn parses_production_cli_shape() {
        let j = r#"{"type":"result","subtype":"success","is_error":false,"result":"hello","stop_reason":"end_turn","usage":{"input_tokens":6,"output_tokens":6,"cache_read_input_tokens":23529},"modelUsage":{"claude-opus-4-7[1m]":{"inputTokens":6,"outputTokens":6}}}"#;
        let parsed: ClaudeCliJsonResponse = serde_json::from_str(j).expect("parse");
        assert_eq!(parsed.result.as_deref(), Some("hello"));
        assert_eq!(parsed.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(
            parsed.dominant_model().as_deref(),
            Some("claude-opus-4-7[1m]")
        );
    }

    #[test]
    fn dominant_model_picks_highest_output_tokens() {
        let j = r#"{"result":"x","modelUsage":{"claude-haiku-4-5":{"outputTokens":1},"claude-opus-4-7":{"outputTokens":100}}}"#;
        let parsed: ClaudeCliJsonResponse = serde_json::from_str(j).expect("parse");
        assert_eq!(parsed.dominant_model().as_deref(), Some("claude-opus-4-7"));
    }

    #[test]
    fn is_error_flag_round_trips() {
        let j = r#"{"is_error":true,"result":"oops"}"#;
        let parsed: ClaudeCliJsonResponse = serde_json::from_str(j).expect("parse");
        assert!(parsed.is_error);
    }
}
