//! Pure JSON ↔ argo-types translation for the Anthropic Messages wire.
//!
//! No I/O, no reqwest — kept side-effect free so we can unit-test it without
//! a runtime. Spec: <https://docs.anthropic.com/en/api/messages>.
use crate::manifest::ProviderManifest;
use crate::transport::{ProviderRequest, ProviderResponse, Sampling, StopReason, ToolSchema};
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageRole, ToolCallId, ToolCallRequest,
};

/// Anthropic Messages requires `max_tokens` — we pick a high default when
/// the caller omits it so the request doesn't 400.
pub(super) const DEFAULT_MAX_TOKENS: u32 = 4096;

pub(super) fn build_request_body(
    manifest: &ProviderManifest,
    req: &ProviderRequest,
) -> ArgoResult<serde_json::Value> {
    let model = manifest.resolve_model_id(&req.model).to_string();
    let messages: Vec<serde_json::Value> = req
        .messages
        .iter()
        .map(message_to_anthropic)
        .collect::<ArgoResult<_>>()?;
    let tools_json: Vec<serde_json::Value> =
        req.tools.iter().map(tool_schema_to_anthropic).collect();
    let max_tokens = req.sampling.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS);
    let mut body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": messages,
    });
    if let Some(sys) = &req.system {
        body["system"] = serde_json::Value::String(sys.clone());
    }
    if !tools_json.is_empty() {
        body["tools"] = serde_json::Value::Array(tools_json);
    }
    apply_sampling(&mut body, &req.sampling);
    apply_quirks(&mut body, manifest);
    Ok(body)
}

fn apply_sampling(body: &mut serde_json::Value, s: &Sampling) {
    if let Some(t) = s.temperature {
        body["temperature"] = serde_json::json!(t);
    }
    if let Some(p) = s.top_p {
        body["top_p"] = serde_json::json!(p);
    }
    if !s.stop.is_empty() {
        body["stop_sequences"] = serde_json::json!(s.stop);
    }
}

/// Apply quirks declared in `[quirks]` (issue #2 / F-3).
///
/// - `send-thinking-blocks = true` → enable extended thinking via the
///   `thinking` request param.
/// - `prompt-caching-breakpoints = ["system", "tools"]` → tag the relevant
///   blocks with `cache_control = { type = "ephemeral" }`.
pub(super) fn apply_quirks(body: &mut serde_json::Value, manifest: &ProviderManifest) {
    if matches!(
        manifest.quirks.get("send-thinking-blocks"),
        Some(toml::Value::Boolean(true))
    ) {
        body["thinking"] = serde_json::json!({ "type": "enabled", "budget_tokens": 1024 });
    }
    if let Some(toml::Value::Array(bps)) = manifest.quirks.get("prompt-caching-breakpoints") {
        let labels: Vec<&str> = bps.iter().filter_map(|v| v.as_str()).collect();
        if labels.contains(&"system") {
            tag_system_block_for_cache(body);
        }
        if labels.contains(&"tools") {
            tag_tools_for_cache(body);
        }
    }
}

fn tag_system_block_for_cache(body: &mut serde_json::Value) {
    let Some(serde_json::Value::String(text)) = body.get("system").cloned() else {
        return;
    };
    body["system"] = serde_json::json!([{
        "type": "text",
        "text": text,
        "cache_control": { "type": "ephemeral" },
    }]);
}

fn tag_tools_for_cache(body: &mut serde_json::Value) {
    let Some(serde_json::Value::Array(tools)) = body.get_mut("tools") else {
        return;
    };
    if let Some(serde_json::Value::Object(map)) = tools.last_mut() {
        map.insert(
            "cache_control".into(),
            serde_json::json!({ "type": "ephemeral" }),
        );
    }
}

fn tool_schema_to_anthropic(t: &ToolSchema) -> serde_json::Value {
    serde_json::json!({
        "name": t.name,
        "description": t.description,
        "input_schema": t.parameters,
    })
}

fn message_to_anthropic(m: &Message) -> ArgoResult<serde_json::Value> {
    // Anthropic only accepts user / assistant roles; tool results ride as
    // user-role messages with `tool_result` content blocks.
    let role =
        match m.role {
            MessageRole::User | MessageRole::Tool => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => return Err(ArgoError::Provider(
                "system messages must be passed via ProviderRequest::system, not the message list"
                    .into(),
            )),
        };
    let mut content = Vec::new();
    if matches!(m.role, MessageRole::Tool) {
        push_tool_result(m, &mut content);
    }
    for block in &m.content {
        if let Some(v) = content_block_to_anthropic(block) {
            content.push(v);
        }
    }
    for tc in &m.tool_calls {
        content.push(tool_call_to_anthropic(tc));
    }
    if content.is_empty() {
        content.push(serde_json::json!({ "type": "text", "text": "" }));
    }
    Ok(serde_json::json!({ "role": role, "content": content }))
}

fn push_tool_result(m: &Message, content: &mut Vec<serde_json::Value>) {
    let tcid = match m.tool_call_id {
        Some(id) => id.to_string(),
        None => return,
    };
    let body = m
        .content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text { text } => Some(text.as_str()),
            ContentBlock::ToolResult(r) => r.content.iter().find_map(|c| match c {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            }),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    content.push(serde_json::json!({
        "type": "tool_result",
        "tool_use_id": tcid,
        "content": body,
    }));
}

fn content_block_to_anthropic(block: &ContentBlock) -> Option<serde_json::Value> {
    match block {
        ContentBlock::Text { text } => Some(serde_json::json!({
            "type": "text",
            "text": text,
        })),
        ContentBlock::Image { url, .. } => Some(serde_json::json!({
            "type": "image",
            "source": { "type": "url", "url": url },
        })),
        ContentBlock::Audio { url, mime } => {
            tracing::warn!(%url, %mime, "anthropic: dropping unsupported Audio block");
            None
        }
        ContentBlock::Document { url, mime } => Some(serde_json::json!({
            "type": "document",
            "source": { "type": "url", "url": url, "media_type": mime },
        })),
        ContentBlock::ToolUse(_) | ContentBlock::ToolResult(_) => None,
    }
}

fn tool_call_to_anthropic(tc: &ToolCallRequest) -> serde_json::Value {
    // #719: emit the internal `tc.id` (UUIDv7), NOT
    // `provider_call_id`. The matching `tool_result` block produced
    // by `push_tool_result` derives its `tool_use_id` from
    // `Message.tool_call_id` — i.e. the same internal `ToolCallId`
    // — so both sides round-trip with the same id and Anthropic
    // accepts the multi-turn request.
    //
    // Pre-#719 this preferred `provider_call_id` (the `toolu_…` id
    // Anthropic generated). That was an aspirational comment ("so
    // multi-turn round-trips cleanly") that broke the round-trip
    // because `ToolCallResult` never carried `provider_call_id`,
    // so the two sides emitted different ids. Anthropic's API
    // doesn't require the round-tripped id to be the original
    // `toolu_…` value — only that the assistant's `tool_use.id`
    // matches the next user message's `tool_result.tool_use_id`.
    serde_json::json!({
        "type": "tool_use",
        "id": tc.id.to_string(),
        "name": tc.name,
        "input": tc.arguments,
    })
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct AnthropicResponse {
    #[serde(default)]
    #[allow(dead_code)]
    pub id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub model: Option<String>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub content: Vec<AnthropicBlock>,
    #[serde(default)]
    pub usage: Option<AnthropicUsage>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type")]
pub(super) enum AnthropicBlock {
    #[serde(rename = "text")]
    Text {
        #[serde(default)]
        text: String,
    },
    #[serde(rename = "thinking")]
    Thinking {
        #[serde(default)]
        thinking: String,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        #[serde(default)]
        id: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct AnthropicUsage {
    #[serde(default)]
    pub input_tokens: Option<u32>,
    #[serde(default)]
    pub output_tokens: Option<u32>,
    /// Tokens used to create a prompt-cache entry (mission-prompt-cache-hit-rate-observability).
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u32>,
    /// Tokens served from a prompt-cache entry.
    #[serde(default)]
    pub cache_read_input_tokens: Option<u32>,
}

pub(super) fn assemble_response(
    parsed: AnthropicResponse,
    raw: serde_json::Value,
    provider_id: &str,
    model_id: &str,
) -> ProviderResponse {
    if let Some(usage) = parsed.usage.as_ref() {
        argo_telemetry::metrics_prompt_cache::record(
            provider_id,
            model_id,
            usage.input_tokens.unwrap_or(0),
            usage.cache_creation_input_tokens.unwrap_or(0),
            usage.cache_read_input_tokens.unwrap_or(0),
        );
    }
    let stop_reason = parsed
        .stop_reason
        .as_deref()
        .map_or(StopReason::EndTurn, map_stop_reason);
    let mut text = String::new();
    let mut reasoning = String::new();
    let mut tool_calls = Vec::new();
    for block in parsed.content {
        match block {
            AnthropicBlock::Text { text: t } => text.push_str(&t),
            AnthropicBlock::Thinking { thinking } => reasoning.push_str(&thinking),
            AnthropicBlock::ToolUse { id, name, input } => {
                tool_calls.push(ToolCallRequest {
                    id: ToolCallId::new(),
                    provider_call_id: (!id.is_empty()).then_some(id),
                    name,
                    arguments: input,
                });
            }
            AnthropicBlock::Unknown => {}
        }
    }
    ProviderResponse {
        text: (!text.is_empty()).then_some(text),
        reasoning: (!reasoning.is_empty()).then_some(reasoning),
        tool_calls,
        stop_reason,
        tokens_in: parsed.usage.as_ref().and_then(|u| u.input_tokens),
        tokens_out: parsed.usage.as_ref().and_then(|u| u.output_tokens),
        provider_meta: raw,
    }
}

pub(super) fn map_stop_reason(s: &str) -> StopReason {
    match s {
        "end_turn" => StopReason::EndTurn,
        "tool_use" => StopReason::ToolUse,
        "max_tokens" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::EndTurn,
        other => StopReason::Other(other.to_string()),
    }
}
