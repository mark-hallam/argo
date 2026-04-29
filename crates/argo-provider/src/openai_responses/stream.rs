//! Streaming SSE event → [`ProviderResponseDelta`] conversion for the
//! OpenAI Responses API.
//!
//! Responses streaming uses typed events (`response.created`,
//! `response.output_item.added`, `response.output_text.delta`,
//! `response.function_call_arguments.delta`, `response.completed`, …). We
//! dispatch on the `type` field of the JSON payload.
use crate::transport::{ProviderResponseDelta, StopReason};
use argo_types::{ArgoError, ArgoResult};
use std::collections::HashMap;

/// Per-item state — Responses indexes streamed function-call items by their
/// `output_index`; arguments arrive as deltas referencing the same index.
#[derive(Debug, Default)]
pub(super) struct StreamState {
    items: HashMap<u64, ItemState>,
    pub(super) done: bool,
}

#[derive(Debug)]
struct ItemState {
    id: String,
    name: String,
}

impl StreamState {
    pub(super) fn new() -> Self {
        Self::default()
    }
}

pub(super) fn event_to_deltas(
    state: &mut StreamState,
    payload: &str,
) -> ArgoResult<Vec<ProviderResponseDelta>> {
    if payload.is_empty() {
        return Ok(Vec::new());
    }
    let v: serde_json::Value = serde_json::from_str(payload)
        .map_err(|e| ArgoError::Provider(format!("openai-responses SSE event JSON: {e}")))?;
    let kind = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let mut out = Vec::new();
    match kind {
        "response.output_text.delta" => push_text(&v, &mut out),
        "response.reasoning_summary_text.delta" => push_reasoning(&v, &mut out),
        "response.output_item.added" => add_item(state, &v, &mut out),
        "response.function_call_arguments.delta" => push_args_delta(state, &v, &mut out),
        "response.completed" => push_completed(&v, &mut out, state),
        "response.failed" => out.push(ProviderResponseDelta::FinishReason(StopReason::Other(
            "failed".into(),
        ))),
        _ => {} // forward-compatible: unknown event kinds skipped silently
    }
    Ok(out)
}

fn push_text(v: &serde_json::Value, out: &mut Vec<ProviderResponseDelta>) {
    if let Some(t) = v.get("delta").and_then(|x| x.as_str()) {
        if !t.is_empty() {
            out.push(ProviderResponseDelta::TextChunk(t.to_string()));
        }
    }
}

fn push_reasoning(v: &serde_json::Value, out: &mut Vec<ProviderResponseDelta>) {
    if let Some(t) = v.get("delta").and_then(|x| x.as_str()) {
        if !t.is_empty() {
            out.push(ProviderResponseDelta::ReasoningChunk(t.to_string()));
        }
    }
}

fn add_item(state: &mut StreamState, v: &serde_json::Value, out: &mut Vec<ProviderResponseDelta>) {
    let item = match v.get("item") {
        Some(i) => i,
        None => return,
    };
    if item.get("type").and_then(|t| t.as_str()) != Some("function_call") {
        return;
    }
    let index = v.get("output_index").and_then(|x| x.as_u64()).unwrap_or(0);
    let id = item
        .get("call_id")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let name = item
        .get("name")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    out.push(ProviderResponseDelta::ToolCallDelta {
        id: id.clone(),
        name: name.clone(),
        args_partial: String::new(),
    });
    state.items.insert(index, ItemState { id, name });
}

fn push_args_delta(
    state: &mut StreamState,
    v: &serde_json::Value,
    out: &mut Vec<ProviderResponseDelta>,
) {
    let index = v.get("output_index").and_then(|x| x.as_u64()).unwrap_or(0);
    let item = match state.items.get(&index) {
        Some(i) => i,
        None => return,
    };
    let chunk = v.get("delta").and_then(|x| x.as_str()).unwrap_or("");
    if chunk.is_empty() {
        return;
    }
    out.push(ProviderResponseDelta::ToolCallDelta {
        id: item.id.clone(),
        name: item.name.clone(),
        args_partial: chunk.to_string(),
    });
}

fn push_completed(
    v: &serde_json::Value,
    out: &mut Vec<ProviderResponseDelta>,
    state: &mut StreamState,
) {
    let resp = v.get("response");
    let status = resp
        .and_then(|r| r.get("status"))
        .and_then(|x| x.as_str())
        .unwrap_or("completed");
    out.push(ProviderResponseDelta::FinishReason(
        super::wire::map_status(status),
    ));
    if let Some(u) = resp.and_then(|r| r.get("usage")) {
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
    state.done = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::StopReason;

    #[test]
    fn text_delta_extracted() {
        let mut s = StreamState::new();
        let d = event_to_deltas(
            &mut s,
            r#"{"type":"response.output_text.delta","delta":"Hello"}"#,
        )
        .unwrap();
        assert_eq!(d, vec![ProviderResponseDelta::TextChunk("Hello".into())]);
    }

    #[test]
    fn reasoning_delta_extracted() {
        let mut s = StreamState::new();
        let d = event_to_deltas(
            &mut s,
            r#"{"type":"response.reasoning_summary_text.delta","delta":"thinking"}"#,
        )
        .unwrap();
        assert_eq!(
            d,
            vec![ProviderResponseDelta::ReasoningChunk("thinking".into())]
        );
    }

    #[test]
    fn function_call_starter_then_args() {
        let mut s = StreamState::new();
        let starter = event_to_deltas(
            &mut s,
            r#"{"type":"response.output_item.added","output_index":0,
                "item":{"type":"function_call","call_id":"call_x","name":"ls"}}"#,
        )
        .unwrap();
        assert_eq!(
            starter,
            vec![ProviderResponseDelta::ToolCallDelta {
                id: "call_x".into(),
                name: "ls".into(),
                args_partial: String::new(),
            }]
        );
        let chunk = event_to_deltas(
            &mut s,
            r#"{"type":"response.function_call_arguments.delta","output_index":0,
                "delta":"{\"path\":\"/\"}"}"#,
        )
        .unwrap();
        assert_eq!(
            chunk,
            vec![ProviderResponseDelta::ToolCallDelta {
                id: "call_x".into(),
                name: "ls".into(),
                args_partial: r#"{"path":"/"}"#.into(),
            }]
        );
    }

    #[test]
    fn completed_emits_finish_and_usage() {
        let mut s = StreamState::new();
        let d = event_to_deltas(
            &mut s,
            r#"{"type":"response.completed","response":{"status":"completed",
                "usage":{"input_tokens":12,"output_tokens":3}}}"#,
        )
        .unwrap();
        assert!(d.contains(&ProviderResponseDelta::FinishReason(StopReason::EndTurn)));
        assert!(d.contains(&ProviderResponseDelta::TokenUsage {
            tokens_in: 12,
            tokens_out: 3
        }));
        assert!(s.done);
    }
}
