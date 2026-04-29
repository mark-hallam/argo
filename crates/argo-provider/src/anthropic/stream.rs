//! Streaming SSE event → [`ProviderResponseDelta`] conversion for the
//! Anthropic Messages wire.
//!
//! Anthropic emits a typed event sequence (`message_start`,
//! `content_block_start`, `content_block_delta`, `content_block_stop`,
//! `message_delta`, `message_stop`). Each event's `data:` payload carries
//! the same `type` discriminator as the SSE `event:` line, so we dispatch
//! on the JSON `type` field alone — no `event:` parsing needed.
//!
//! Spec: <https://docs.anthropic.com/en/api/messages-streaming>.
use crate::anthropic::wire::map_stop_reason;
use crate::transport::ProviderResponseDelta;
use argo_types::{ArgoError, ArgoResult};
use std::collections::HashMap;

/// Per-block state carried across `content_block_delta` events. Anthropic
/// indexes blocks by `index` and we need the block's id + name when
/// emitting tool-call deltas (those only arrive on the `_start` event).
#[derive(Debug, Default)]
pub(super) struct StreamState {
    blocks: HashMap<u64, BlockState>,
    pub(super) done: bool,
}

#[derive(Debug)]
struct BlockState {
    kind: BlockKind,
    id: String,
    name: String,
}

#[derive(Debug, Clone, Copy)]
enum BlockKind {
    Text,
    Thinking,
    ToolUse,
    Other,
}

impl StreamState {
    pub(super) fn new() -> Self {
        Self::default()
    }
}

/// Parse one Anthropic streaming event payload into zero or more deltas.
pub(super) fn event_to_deltas(
    state: &mut StreamState,
    payload: &str,
) -> ArgoResult<Vec<ProviderResponseDelta>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }
    let v: serde_json::Value = serde_json::from_str(payload)
        .map_err(|e| ArgoError::Provider(format!("anthropic SSE event JSON: {e}")))?;
    let kind = v
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or_else(|| ArgoError::Provider("anthropic SSE event missing `type`".into()))?;
    let mut out = Vec::new();
    match kind {
        "message_start" => push_message_start(&v, &mut out),
        "content_block_start" => handle_block_start(state, &v, &mut out),
        "content_block_delta" => handle_block_delta(state, &v, &mut out),
        "content_block_stop" => {} // no-op; we don't pin a delta on close
        "message_delta" => push_message_delta(&v, &mut out),
        "message_stop" => state.done = true,
        "ping" | "error" => {}
        _ => {} // forward-compatible: unknown event kinds skipped silently
    }
    Ok(out)
}

fn push_message_start(v: &serde_json::Value, out: &mut Vec<ProviderResponseDelta>) {
    let usage = v.get("message").and_then(|m| m.get("usage"));
    if let Some(u) = usage {
        if let (Some(i), Some(o)) = (
            u.get("input_tokens").and_then(|x| x.as_u64()),
            u.get("output_tokens").and_then(|x| x.as_u64()),
        ) {
            out.push(ProviderResponseDelta::TokenUsage {
                tokens_in: i as u32,
                tokens_out: o as u32,
            });
        }
    }
}

fn handle_block_start(
    state: &mut StreamState,
    v: &serde_json::Value,
    out: &mut Vec<ProviderResponseDelta>,
) {
    let index = v.get("index").and_then(|x| x.as_u64()).unwrap_or(0);
    let cb = match v.get("content_block") {
        Some(c) => c,
        None => return,
    };
    let kind_str = cb.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let (kind, id, name) = match kind_str {
        "text" => (BlockKind::Text, String::new(), String::new()),
        "thinking" => (BlockKind::Thinking, String::new(), String::new()),
        "tool_use" => {
            let id = cb
                .get("id")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let name = cb
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            // Emit a starter delta so consumers learn the id+name early.
            out.push(ProviderResponseDelta::ToolCallDelta {
                id: id.clone(),
                name: name.clone(),
                args_partial: String::new(),
            });
            (BlockKind::ToolUse, id, name)
        }
        _ => (BlockKind::Other, String::new(), String::new()),
    };
    state.blocks.insert(index, BlockState { kind, id, name });
}

fn handle_block_delta(
    state: &mut StreamState,
    v: &serde_json::Value,
    out: &mut Vec<ProviderResponseDelta>,
) {
    let index = v.get("index").and_then(|x| x.as_u64()).unwrap_or(0);
    let delta = match v.get("delta") {
        Some(d) => d,
        None => return,
    };
    let block = match state.blocks.get(&index) {
        Some(b) => b,
        None => return,
    };
    match (block.kind, delta.get("type").and_then(|t| t.as_str())) {
        (BlockKind::Text, Some("text_delta")) => {
            if let Some(t) = delta.get("text").and_then(|x| x.as_str()) {
                if !t.is_empty() {
                    out.push(ProviderResponseDelta::TextChunk(t.to_string()));
                }
            }
        }
        (BlockKind::Thinking, Some("thinking_delta")) => {
            if let Some(t) = delta.get("thinking").and_then(|x| x.as_str()) {
                if !t.is_empty() {
                    out.push(ProviderResponseDelta::ReasoningChunk(t.to_string()));
                }
            }
        }
        (BlockKind::ToolUse, Some("input_json_delta")) => {
            if let Some(p) = delta.get("partial_json").and_then(|x| x.as_str()) {
                out.push(ProviderResponseDelta::ToolCallDelta {
                    id: block.id.clone(),
                    name: block.name.clone(),
                    args_partial: p.to_string(),
                });
            }
        }
        _ => {}
    }
}

fn push_message_delta(v: &serde_json::Value, out: &mut Vec<ProviderResponseDelta>) {
    if let Some(reason) = v
        .get("delta")
        .and_then(|d| d.get("stop_reason"))
        .and_then(|x| x.as_str())
    {
        out.push(ProviderResponseDelta::FinishReason(map_stop_reason(reason)));
    }
    if let Some(u) = v.get("usage") {
        if let Some(o) = u.get("output_tokens").and_then(|x| x.as_u64()) {
            // Anthropic sends only `output_tokens` on message_delta. Pair it
            // with 0 for input — consumers already saw input_tokens on
            // message_start.
            out.push(ProviderResponseDelta::TokenUsage {
                tokens_in: 0,
                tokens_out: o as u32,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::StopReason;

    #[test]
    fn message_start_emits_token_usage() {
        let mut s = StreamState::new();
        let payload = r#"{"type":"message_start","message":{
            "id":"msg_1","model":"claude-haiku-4-5",
            "usage":{"input_tokens":12,"output_tokens":1}
        }}"#;
        let d = event_to_deltas(&mut s, payload).unwrap();
        assert_eq!(
            d,
            vec![ProviderResponseDelta::TokenUsage {
                tokens_in: 12,
                tokens_out: 1
            }]
        );
    }

    #[test]
    fn text_delta_emitted() {
        let mut s = StreamState::new();
        event_to_deltas(
            &mut s,
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        )
        .unwrap();
        let d = event_to_deltas(
            &mut s,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#,
        )
        .unwrap();
        assert_eq!(d, vec![ProviderResponseDelta::TextChunk("Hello".into())]);
    }

    #[test]
    fn thinking_delta_emitted() {
        let mut s = StreamState::new();
        event_to_deltas(
            &mut s,
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#,
        )
        .unwrap();
        let d = event_to_deltas(
            &mut s,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"reasoning..."}}"#,
        )
        .unwrap();
        assert_eq!(
            d,
            vec![ProviderResponseDelta::ReasoningChunk("reasoning...".into())]
        );
    }

    #[test]
    fn tool_use_block_emits_starter_then_partial() {
        let mut s = StreamState::new();
        let starter = event_to_deltas(
            &mut s,
            r#"{"type":"content_block_start","index":1,"content_block":{
                "type":"tool_use","id":"toolu_1","name":"ls","input":{}
            }}"#,
        )
        .unwrap();
        assert_eq!(
            starter,
            vec![ProviderResponseDelta::ToolCallDelta {
                id: "toolu_1".into(),
                name: "ls".into(),
                args_partial: String::new(),
            }]
        );
        let chunk = event_to_deltas(
            &mut s,
            r#"{"type":"content_block_delta","index":1,"delta":{
                "type":"input_json_delta","partial_json":"{\"path\":\"/\"}"
            }}"#,
        )
        .unwrap();
        assert_eq!(
            chunk,
            vec![ProviderResponseDelta::ToolCallDelta {
                id: "toolu_1".into(),
                name: "ls".into(),
                args_partial: r#"{"path":"/"}"#.into(),
            }]
        );
    }

    #[test]
    fn message_delta_emits_finish_reason() {
        let mut s = StreamState::new();
        let d = event_to_deltas(
            &mut s,
            r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":3}}"#,
        )
        .unwrap();
        assert!(d.contains(&ProviderResponseDelta::FinishReason(StopReason::EndTurn)));
        assert!(d
            .iter()
            .any(|x| matches!(x, ProviderResponseDelta::TokenUsage { tokens_out: 3, .. })));
    }

    #[test]
    fn message_stop_marks_state_done() {
        let mut s = StreamState::new();
        event_to_deltas(&mut s, r#"{"type":"message_stop"}"#).unwrap();
        assert!(s.done);
    }

    #[test]
    fn missing_type_is_provider_error() {
        let mut s = StreamState::new();
        let err = event_to_deltas(&mut s, r#"{"foo":1}"#).unwrap_err();
        assert!(matches!(err, ArgoError::Provider(_)));
    }
}
