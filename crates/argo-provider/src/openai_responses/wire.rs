//! Pure JSON ↔ argo-types translation for the OpenAI Responses API.
//!
//! The Responses API uses `input` / `output` arrays of *items* (not the
//! Chat-Completions-style `messages` array). Each item is one of
//! `message`, `function_call`, `function_call_output`, `reasoning`, or a
//! handful of others. Spec: <https://platform.openai.com/docs/api-reference/responses>.
use crate::manifest::ProviderManifest;
use crate::transport::{ProviderRequest, ProviderResponse, Sampling, StopReason, ToolSchema};
use argo_types::{ArgoResult, ContentBlock, Message, MessageRole, ToolCallId, ToolCallRequest};

pub(super) fn build_request_body(
    manifest: &ProviderManifest,
    req: &ProviderRequest,
) -> ArgoResult<serde_json::Value> {
    let model = manifest.resolve_model_id(&req.model).to_string();
    let mut input: Vec<serde_json::Value> = Vec::new();
    for m in &req.messages {
        push_message_items(m, &mut input);
    }
    let tools_json: Vec<serde_json::Value> =
        req.tools.iter().map(tool_schema_to_responses).collect();
    let mut body = serde_json::json!({
        "model": model,
        "input": input,
    });
    if let Some(sys) = &req.system {
        body["instructions"] = serde_json::Value::String(sys.clone());
    }
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
        body["max_output_tokens"] = serde_json::json!(m);
    }
}

fn tool_schema_to_responses(t: &ToolSchema) -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "name": t.name,
        "description": t.description,
        "parameters": t.parameters,
    })
}

fn push_message_items(m: &Message, out: &mut Vec<serde_json::Value>) {
    if let MessageRole::Tool = m.role {
        // Tool result becomes its own item.
        if let Some(tcid) = m.tool_call_id {
            let body = collect_text(&m.content);
            out.push(serde_json::json!({
                "type": "function_call_output",
                "call_id": tcid.to_string(),
                "output": body,
            }));
        }
        return;
    }
    let role = match m.role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => unreachable!("handled above"),
    };
    let content = content_to_responses(&m.content);
    out.push(serde_json::json!({
        "type": "message",
        "role": role,
        "content": content,
    }));
    for tc in &m.tool_calls {
        out.push(serde_json::json!({
            "type": "function_call",
            "call_id": tc.id.to_string(),
            "name": tc.name,
            "arguments": tc.arguments.to_string(),
        }));
    }
}

fn content_to_responses(blocks: &[ContentBlock]) -> serde_json::Value {
    let parts: Vec<serde_json::Value> = blocks
        .iter()
        .filter_map(content_block_to_responses)
        .collect();
    serde_json::Value::Array(parts)
}

fn content_block_to_responses(block: &ContentBlock) -> Option<serde_json::Value> {
    match block {
        ContentBlock::Text { text } => Some(serde_json::json!({
            "type": "input_text",
            "text": text,
        })),
        ContentBlock::Image { url, .. } => Some(serde_json::json!({
            "type": "input_image",
            "image_url": url,
        })),
        ContentBlock::Audio { url, mime } => {
            tracing::warn!(%url, %mime, "openai-responses: dropping unsupported Audio block");
            None
        }
        ContentBlock::Document { url, mime } => {
            tracing::warn!(%url, %mime, "openai-responses: dropping unsupported Document block");
            None
        }
        ContentBlock::ToolUse(_) | ContentBlock::ToolResult(_) => None,
    }
}

fn collect_text(blocks: &[ContentBlock]) -> String {
    blocks
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
        .join("")
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct ResponsesEnvelope {
    #[serde(default)]
    #[allow(dead_code)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub output: Vec<ResponsesItem>,
    #[serde(default)]
    pub usage: Option<ResponsesUsage>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type")]
pub(super) enum ResponsesItem {
    #[serde(rename = "message")]
    Message {
        #[serde(default)]
        content: Vec<ResponsesContent>,
    },
    #[serde(rename = "function_call")]
    FunctionCall {
        #[serde(default)]
        call_id: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        arguments: String,
    },
    #[serde(rename = "reasoning")]
    Reasoning {
        #[serde(default)]
        summary: Vec<ResponsesContent>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type")]
pub(super) enum ResponsesContent {
    #[serde(rename = "output_text")]
    OutputText {
        #[serde(default)]
        text: String,
    },
    #[serde(rename = "summary_text")]
    SummaryText {
        #[serde(default)]
        text: String,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct ResponsesUsage {
    #[serde(default)]
    pub input_tokens: Option<u32>,
    #[serde(default)]
    pub output_tokens: Option<u32>,
}

pub(super) fn assemble_response(
    parsed: ResponsesEnvelope,
    raw: serde_json::Value,
) -> ProviderResponse {
    let mut text = String::new();
    let mut reasoning = String::new();
    let mut tool_calls: Vec<ToolCallRequest> = Vec::new();
    let mut sticky_tool_use = false;
    for item in parsed.output {
        match item {
            ResponsesItem::Message { content } => {
                for c in content {
                    if let ResponsesContent::OutputText { text: t } = c {
                        text.push_str(&t);
                    }
                }
            }
            ResponsesItem::Reasoning { summary } => {
                for c in summary {
                    if let ResponsesContent::SummaryText { text: t } = c {
                        reasoning.push_str(&t);
                    }
                }
            }
            ResponsesItem::FunctionCall {
                call_id,
                name,
                arguments,
            } => {
                let parsed_args = if arguments.is_empty() {
                    serde_json::Value::Object(serde_json::Map::new())
                } else {
                    serde_json::from_str(&arguments).unwrap_or(serde_json::Value::Null)
                };
                tool_calls.push(ToolCallRequest {
                    id: ToolCallId::new(),
                    name,
                    arguments: parsed_args,
                });
                let _ = call_id; // provider id round-trip is issue #4 (Agent G).
                sticky_tool_use = true;
            }
            ResponsesItem::Unknown => {}
        }
    }
    let stop_reason = if sticky_tool_use {
        StopReason::ToolUse
    } else {
        map_status(parsed.status.as_deref().unwrap_or("completed"))
    };
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

pub(super) fn map_status(s: &str) -> StopReason {
    match s {
        "completed" => StopReason::EndTurn,
        "incomplete" => StopReason::MaxTokens,
        "failed" => StopReason::Other("failed".into()),
        other => StopReason::Other(other.to_string()),
    }
}
