//! Streaming SSE parser for OpenAI Chat Completions.
//!
//! Two layers:
//!
//! 1. [`SseBuffer::take_event`] — pure byte-level SSE event extraction.
//! 2. [`event_to_deltas`] — pure JSON → [`ProviderResponseDelta`] conversion.
//!
//! The actual stream construction (chunked HTTP, error mapping) lives in
//! `mod.rs::stream_response_body`. Keeping the parsers pure makes them
//! straightforward to unit-test without a network.
use crate::transport::{ProviderResponseDelta, StopReason};
use argo_types::{ArgoError, ArgoResult};

/// Sentinel that OpenAI uses to signal the end of an SSE stream.
pub(super) const DONE_SENTINEL: &str = "[DONE]";

/// Buffer of partial SSE bytes; pulls one full event per `take_event` call.
///
/// SSE events are terminated by a blank line (`\n\n` or `\r\n\r\n`). Each
/// event is a sequence of `field: value` lines; we only care about `data:`
/// here (OpenAI doesn't use `event:` / `id:` / `retry:` for completions).
#[derive(Debug, Default)]
pub(super) struct SseBuffer {
    buf: String,
}

impl SseBuffer {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn extend(&mut self, chunk: &str) {
        self.buf.push_str(chunk);
    }

    /// Pop the next complete event from the buffer, if any. Returns the
    /// concatenated `data:` payload (multi-line `data:` joins with `\n`).
    pub(super) fn take_event(&mut self) -> Option<String> {
        let term = self
            .buf
            .find("\r\n\r\n")
            .map(|i| (i, 4))
            .or_else(|| self.buf.find("\n\n").map(|i| (i, 2)))?;
        let (end, sep_len) = term;
        let event_text: String = self.buf[..end].to_string();
        self.buf.drain(..end + sep_len);
        Some(extract_data_lines(&event_text))
    }
}

fn extract_data_lines(event_text: &str) -> String {
    let mut data = String::new();
    for line in event_text.split('\n') {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
        }
    }
    data
}

/// Parse one OpenAI Chat Completions streaming event into zero or more
/// deltas. The event payload is the `data:` value; the caller has already
/// stripped the `data: ` prefix.
///
/// ```
/// use argo_provider::{event_to_deltas, ProviderResponseDelta};
/// let deltas = event_to_deltas(r#"{"choices":[{"delta":{"content":"hi"}}]}"#)
///     .unwrap();
/// assert_eq!(deltas, vec![ProviderResponseDelta::TextChunk("hi".into())]);
/// ```
pub fn event_to_deltas(payload: &str) -> ArgoResult<Vec<ProviderResponseDelta>> {
    if payload == DONE_SENTINEL {
        return Ok(Vec::new());
    }
    let value: serde_json::Value = serde_json::from_str(payload)
        .map_err(|e| ArgoError::Provider(format!("openai SSE event JSON: {e}")))?;
    let mut out = Vec::new();
    push_choice_deltas(&value, &mut out);
    push_usage_delta(&value, &mut out);
    Ok(out)
}

fn push_choice_deltas(value: &serde_json::Value, out: &mut Vec<ProviderResponseDelta>) {
    let Some(choices) = value.get("choices").and_then(|v| v.as_array()) else {
        return;
    };
    for choice in choices {
        push_text_delta(choice, out);
        push_reasoning_delta(choice, out);
        push_tool_call_deltas(choice, out);
        push_finish_reason(choice, out);
    }
}

fn push_text_delta(choice: &serde_json::Value, out: &mut Vec<ProviderResponseDelta>) {
    if let Some(text) = choice
        .get("delta")
        .and_then(|d| d.get("content"))
        .and_then(|v| v.as_str())
    {
        if !text.is_empty() {
            out.push(ProviderResponseDelta::TextChunk(text.to_string()));
        }
    }
}

fn push_reasoning_delta(choice: &serde_json::Value, out: &mut Vec<ProviderResponseDelta>) {
    let delta = match choice.get("delta") {
        Some(d) => d,
        None => return,
    };
    let value = delta
        .get("reasoning")
        .or_else(|| delta.get("thinking"))
        .and_then(|v| v.as_str());
    if let Some(text) = value {
        if !text.is_empty() {
            out.push(ProviderResponseDelta::ReasoningChunk(text.to_string()));
        }
    }
}

fn push_tool_call_deltas(choice: &serde_json::Value, out: &mut Vec<ProviderResponseDelta>) {
    let Some(arr) = choice
        .get("delta")
        .and_then(|d| d.get("tool_calls"))
        .and_then(|v| v.as_array())
    else {
        return;
    };
    for tc in arr {
        let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let name = tc
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let args = tc
            .get("function")
            .and_then(|f| f.get("arguments"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if id.is_empty() && name.is_empty() && args.is_empty() {
            continue;
        }
        out.push(ProviderResponseDelta::ToolCallDelta {
            id: id.to_string(),
            name: name.to_string(),
            args_partial: args.to_string(),
        });
    }
}

fn push_finish_reason(choice: &serde_json::Value, out: &mut Vec<ProviderResponseDelta>) {
    if let Some(reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
        out.push(ProviderResponseDelta::FinishReason(map_finish_reason(
            reason,
        )));
    }
}

fn push_usage_delta(value: &serde_json::Value, out: &mut Vec<ProviderResponseDelta>) {
    let Some(usage) = value.get("usage") else {
        return;
    };
    let tokens_in = usage
        .get("prompt_tokens")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);
    let tokens_out = usage
        .get("completion_tokens")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32);
    if let (Some(tokens_in), Some(tokens_out)) = (tokens_in, tokens_out) {
        out.push(ProviderResponseDelta::TokenUsage {
            tokens_in,
            tokens_out,
        });
    }
}

fn map_finish_reason(s: &str) -> StopReason {
    match s {
        "stop" => StopReason::EndTurn,
        "tool_calls" | "function_call" => StopReason::ToolUse,
        "length" => StopReason::MaxTokens,
        "content_filter" => StopReason::ContentFilter,
        other => StopReason::Other(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_takes_one_event_at_a_time() {
        let mut b = SseBuffer::new();
        b.extend("data: {\"a\":1}\n\ndata: {\"b\":2}\n\n");
        assert_eq!(b.take_event().as_deref(), Some("{\"a\":1}"));
        assert_eq!(b.take_event().as_deref(), Some("{\"b\":2}"));
        assert!(b.take_event().is_none());
    }

    #[test]
    fn buffer_handles_split_chunks() {
        let mut b = SseBuffer::new();
        b.extend("data: {\"a\":");
        assert!(b.take_event().is_none());
        b.extend("1}\n\n");
        assert_eq!(b.take_event().as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn buffer_handles_crlf_line_endings() {
        let mut b = SseBuffer::new();
        b.extend("data: {\"a\":1}\r\n\r\n");
        assert_eq!(b.take_event().as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn buffer_joins_multi_line_data() {
        let mut b = SseBuffer::new();
        b.extend("data: line1\ndata: line2\n\n");
        assert_eq!(b.take_event().as_deref(), Some("line1\nline2"));
    }

    #[test]
    fn done_sentinel_yields_no_deltas() {
        let deltas = event_to_deltas(DONE_SENTINEL).unwrap();
        assert!(deltas.is_empty());
    }

    #[test]
    fn text_chunk_extracted() {
        let deltas = event_to_deltas(r#"{"choices":[{"delta":{"content":"hi"}}]}"#).unwrap();
        assert_eq!(deltas, vec![ProviderResponseDelta::TextChunk("hi".into())]);
    }

    #[test]
    fn finish_reason_mapped() {
        let deltas =
            event_to_deltas(r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#).unwrap();
        assert_eq!(
            deltas,
            vec![ProviderResponseDelta::FinishReason(StopReason::EndTurn)]
        );
    }

    #[test]
    fn tool_call_delta_extracted() {
        let payload = r#"{"choices":[{"delta":{"tool_calls":[
            {"index":0,"id":"call_x","function":{"name":"ls","arguments":"{\"p\":"}}
        ]}}]}"#;
        let deltas = event_to_deltas(payload).unwrap();
        assert_eq!(
            deltas,
            vec![ProviderResponseDelta::ToolCallDelta {
                id: "call_x".into(),
                name: "ls".into(),
                args_partial: r#"{"p":"#.into(),
            }]
        );
    }

    #[test]
    fn usage_delta_extracted_when_both_counts_present() {
        let payload = r#"{"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":3}}"#;
        let deltas = event_to_deltas(payload).unwrap();
        assert_eq!(
            deltas,
            vec![ProviderResponseDelta::TokenUsage {
                tokens_in: 10,
                tokens_out: 3,
            }]
        );
    }

    #[test]
    fn malformed_event_is_provider_error() {
        let err = event_to_deltas("not-json").unwrap_err();
        assert!(matches!(err, ArgoError::Provider(_)));
    }

    #[test]
    fn empty_text_chunk_is_skipped() {
        let deltas = event_to_deltas(r#"{"choices":[{"delta":{"content":""}}]}"#).unwrap();
        assert!(deltas.is_empty());
    }
}
