//! Pure JSON ↔ argo-types translation for the OpenAI Chat Completions wire.
//!
//! No I/O, no reqwest — kept side-effect free so we can unit-test it without
//! a runtime.
use crate::manifest::ProviderManifest;
use crate::transport::{ProviderRequest, ProviderResponse, Sampling, StopReason, ToolSchema};
use argo_types::{ArgoResult, ContentBlock, Message, MessageRole, ToolCallRequest};
use serde::{Deserialize, Serialize};

pub(super) fn build_request_body(
    manifest: &ProviderManifest,
    req: &ProviderRequest,
) -> ArgoResult<serde_json::Value> {
    let model = manifest.resolve_model_id(&req.model).to_string();
    let mut messages: Vec<serde_json::Value> = Vec::new();
    if let Some(sys) = &req.system {
        messages.push(serde_json::json!({ "role": "system", "content": sys }));
    }
    for m in &req.messages {
        messages.push(message_to_openai(m)?);
    }
    let tools_json = req
        .tools
        .iter()
        .map(tool_schema_to_openai)
        .collect::<Vec<_>>();
    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false,
    });
    if !tools_json.is_empty() {
        body["tools"] = serde_json::Value::Array(tools_json);
    }
    apply_sampling(&mut body, &req.sampling);
    Ok(body)
}

fn apply_sampling(body: &mut serde_json::Value, s: &Sampling) {
    if let Some(t) = s.temperature {
        body["temperature"] = serde_json::json!(t);
    }
    if let Some(p) = s.top_p {
        body["top_p"] = serde_json::json!(p);
    }
    if let Some(m) = s.max_tokens {
        body["max_tokens"] = serde_json::json!(m);
    }
    if !s.stop.is_empty() {
        body["stop"] = serde_json::json!(s.stop);
    }
}

fn tool_schema_to_openai(t: &ToolSchema) -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": t.name,
            "description": t.description,
            "parameters": t.parameters,
        }
    })
}

fn message_to_openai(m: &Message) -> ArgoResult<serde_json::Value> {
    let role = match m.role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    };
    let mut obj = serde_json::Map::new();
    obj.insert("role".into(), serde_json::Value::String(role.into()));
    obj.insert("content".into(), content_to_openai(&m.content));
    if !m.tool_calls.is_empty() {
        obj.insert("tool_calls".into(), tool_calls_to_openai(&m.tool_calls));
    }
    if let Some(tcid) = m.tool_call_id {
        obj.insert(
            "tool_call_id".into(),
            serde_json::Value::String(tcid.to_string()),
        );
    }
    Ok(serde_json::Value::Object(obj))
}

fn content_to_openai(blocks: &[ContentBlock]) -> serde_json::Value {
    if blocks
        .iter()
        .all(|b| matches!(b, ContentBlock::Text { .. }))
    {
        let joined: String = blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        return serde_json::Value::String(joined);
    }
    let parts: Vec<serde_json::Value> = blocks.iter().filter_map(content_block_to_openai).collect();
    serde_json::Value::Array(parts)
}

/// Explicit-arm conversion for one [`ContentBlock`] (issue #5 / F-10).
///
/// Audio and Document blocks are not part of the OpenAI Chat Completions
/// wire; we drop them here but emit a `tracing::warn!` so callers know to
/// convert upstream (e.g. via an MCP tool that uploads + returns a URL).
/// `ToolUse` and `ToolResult` are carried via separate fields on the
/// outgoing message, not as inline content parts.
fn content_block_to_openai(block: &ContentBlock) -> Option<serde_json::Value> {
    match block {
        ContentBlock::Text { text } => Some(serde_json::json!({
            "type": "text",
            "text": text,
        })),
        ContentBlock::Image { url, .. } => Some(serde_json::json!({
            "type": "image_url",
            "image_url": { "url": url },
        })),
        ContentBlock::Audio { url, mime } => {
            tracing::warn!(
                %url, %mime,
                "openai-compat: dropping unsupported Audio content block",
            );
            None
        }
        ContentBlock::Document { url, mime } => {
            tracing::warn!(
                %url, %mime,
                "openai-compat: dropping unsupported Document content block",
            );
            None
        }
        ContentBlock::ToolUse(_) | ContentBlock::ToolResult(_) => None,
    }
}

fn tool_calls_to_openai(calls: &[ToolCallRequest]) -> serde_json::Value {
    let arr: Vec<serde_json::Value> = calls
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id.to_string(),
                "type": "function",
                "function": {
                    "name": c.name,
                    "arguments": c.arguments.to_string(),
                }
            })
        })
        .collect();
    serde_json::Value::Array(arr)
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct OpenAiResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub choices: Vec<OpenAiChoice>,
    #[serde(default)]
    pub usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct OpenAiChoice {
    #[serde(default)]
    pub message: Option<OpenAiMessage>,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct OpenAiMessage {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub thinking: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct OpenAiUsage {
    #[serde(default)]
    pub prompt_tokens: Option<u32>,
    #[serde(default)]
    pub completion_tokens: Option<u32>,
}

pub(super) fn assemble_response(
    parsed: OpenAiResponse,
    tool_calls: Vec<ToolCallRequest>,
    raw: serde_json::Value,
) -> ProviderResponse {
    let first_choice = parsed.choices.first();
    let stop_reason = first_choice
        .and_then(|c| c.finish_reason.as_deref())
        .map_or(StopReason::EndTurn, map_finish_reason);
    let text = first_choice
        .and_then(|c| c.message.as_ref())
        .and_then(|m| m.content.clone());
    let reasoning = first_choice
        .and_then(|c| c.message.as_ref())
        .and_then(|m| m.thinking.clone().or_else(|| m.reasoning.clone()));
    ProviderResponse {
        text: text.filter(|s| !s.is_empty()),
        reasoning,
        tool_calls,
        stop_reason,
        tokens_in: parsed.usage.as_ref().and_then(|u| u.prompt_tokens),
        tokens_out: parsed.usage.as_ref().and_then(|u| u.completion_tokens),
        provider_meta: raw,
    }
}

pub(super) fn map_finish_reason(s: &str) -> StopReason {
    match s {
        "stop" => StopReason::EndTurn,
        "tool_calls" | "function_call" => StopReason::ToolUse,
        "length" => StopReason::MaxTokens,
        "content_filter" => StopReason::ContentFilter,
        other => StopReason::Other(other.to_string()),
    }
}
