//! Pure wire-shape conversion for the AWS Bedrock Converse API.
//!
//! Bedrock's Converse output is a typed `ConverseOutput` from
//! `aws-sdk-bedrockruntime`. We render it as a JSON envelope mirroring the
//! on-the-wire shape so [`crate::extractors::bedrock_tool_use`] can run
//! against it the same way the other extractors run against their wires.
use crate::manifest::ProviderManifest;
use crate::transport::{ProviderRequest, ProviderResponse, Sampling, StopReason, ToolSchema};
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageRole, ToolCallId, ToolCallRequest,
};

/// JSON envelope passed to [`crate::extractors::bedrock_tool_use`].
///
/// Mirrors `ConverseOutput`'s shape:
/// `{ "output": { "message": { "role": ..., "content": [...] } },
///    "stopReason": "...", "usage": { ... } }`
pub(super) fn build_request_body(
    manifest: &ProviderManifest,
    req: &ProviderRequest,
) -> ArgoResult<serde_json::Value> {
    let model = manifest.resolve_model_id(&req.model).to_string();
    let messages: Vec<serde_json::Value> = req
        .messages
        .iter()
        .map(message_to_bedrock)
        .collect::<ArgoResult<_>>()?;
    let mut body = serde_json::json!({
        "modelId": model,
        "messages": messages,
    });
    if let Some(sys) = &req.system {
        body["system"] = serde_json::json!([{ "text": sys }]);
    }
    if !req.tools.is_empty() {
        let tools_json: Vec<serde_json::Value> =
            req.tools.iter().map(tool_schema_to_bedrock).collect();
        body["toolConfig"] = serde_json::json!({ "tools": tools_json });
    }
    apply_inference_config(&mut body, &req.sampling);
    Ok(body)
}

fn apply_inference_config(body: &mut serde_json::Value, s: &Sampling) {
    let mut cfg = serde_json::Map::new();
    if let Some(t) = s.temperature {
        cfg.insert("temperature".into(), serde_json::json!(t));
    }
    if let Some(p) = s.top_p {
        cfg.insert("topP".into(), serde_json::json!(p));
    }
    if let Some(m) = s.max_tokens {
        cfg.insert("maxTokens".into(), serde_json::json!(m));
    }
    if !s.stop.is_empty() {
        cfg.insert("stopSequences".into(), serde_json::json!(s.stop));
    }
    if !cfg.is_empty() {
        body["inferenceConfig"] = serde_json::Value::Object(cfg);
    }
}

fn tool_schema_to_bedrock(t: &ToolSchema) -> serde_json::Value {
    serde_json::json!({
        "toolSpec": {
            "name": t.name,
            "description": t.description,
            "inputSchema": { "json": t.parameters },
        }
    })
}

fn message_to_bedrock(m: &Message) -> ArgoResult<serde_json::Value> {
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
        if let Some(tcid) = m.tool_call_id {
            let body = collect_text(&m.content);
            content.push(serde_json::json!({
                "toolResult": {
                    "toolUseId": tcid.to_string(),
                    "content": [{ "text": body }],
                }
            }));
        }
    }
    for block in &m.content {
        if let Some(v) = content_block_to_bedrock(block) {
            content.push(v);
        }
    }
    for tc in &m.tool_calls {
        content.push(serde_json::json!({
            "toolUse": {
                "toolUseId": tc.id.to_string(),
                "name": tc.name,
                "input": tc.arguments,
            }
        }));
    }
    if content.is_empty() {
        content.push(serde_json::json!({ "text": "" }));
    }
    Ok(serde_json::json!({ "role": role, "content": content }))
}

fn content_block_to_bedrock(block: &ContentBlock) -> Option<serde_json::Value> {
    match block {
        ContentBlock::Text { text } => Some(serde_json::json!({ "text": text })),
        ContentBlock::Image { url, .. } => Some(serde_json::json!({
            "image": { "format": "auto", "source": { "url": url } },
        })),
        ContentBlock::Audio { url, mime } => {
            tracing::warn!(%url, %mime, "bedrock: dropping unsupported Audio block");
            None
        }
        ContentBlock::Document { url, mime } => Some(serde_json::json!({
            "document": { "source": { "url": url }, "format": mime },
        })),
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

/// Convert a `ConverseOutput` into the wire JSON envelope + a parsed
/// [`ProviderResponse`]. Done in one pass so callers don't pay double for
/// serde serialization.
pub(super) fn render_response(
    output: aws_sdk_bedrockruntime::operation::converse::ConverseOutput,
) -> (serde_json::Value, ProviderResponse) {
    use aws_sdk_bedrockruntime::types as bt;
    let stop_reason = match output.stop_reason() {
        bt::StopReason::EndTurn => StopReason::EndTurn,
        bt::StopReason::ToolUse => StopReason::ToolUse,
        bt::StopReason::MaxTokens => StopReason::MaxTokens,
        bt::StopReason::ContentFiltered => StopReason::ContentFilter,
        other => StopReason::Other(other.as_str().to_string()),
    };
    let (text, reasoning, tool_calls, content_json) = render_message_content(output.output());
    let usage = output.usage();
    let tokens_in = usage.map(|u| u.input_tokens() as u32);
    let tokens_out = usage.map(|u| u.output_tokens() as u32);
    let envelope = serde_json::json!({
        "output": { "message": { "role": "assistant", "content": content_json } },
        "stopReason": format!("{:?}", output.stop_reason()),
        "usage": usage.map(|u| serde_json::json!({
            "inputTokens": u.input_tokens(),
            "outputTokens": u.output_tokens(),
        })),
    });
    let resp = ProviderResponse {
        text: (!text.is_empty()).then_some(text),
        reasoning: (!reasoning.is_empty()).then_some(reasoning),
        tool_calls,
        stop_reason,
        tokens_in,
        tokens_out,
        provider_meta: envelope.clone(),
    };
    (envelope, resp)
}

fn render_message_content(
    output: Option<&aws_sdk_bedrockruntime::types::ConverseOutput>,
) -> (String, String, Vec<ToolCallRequest>, serde_json::Value) {
    use aws_sdk_bedrockruntime::types as bt;
    let mut text = String::new();
    let mut reasoning = String::new();
    let mut tool_calls = Vec::new();
    let mut content_json: Vec<serde_json::Value> = Vec::new();
    let blocks: &[bt::ContentBlock] = match output {
        Some(bt::ConverseOutput::Message(m)) => m.content(),
        _ => &[],
    };
    for block in blocks {
        render_content_block(
            block,
            &mut text,
            &mut reasoning,
            &mut tool_calls,
            &mut content_json,
        );
    }
    (
        text,
        reasoning,
        tool_calls,
        serde_json::Value::Array(content_json),
    )
}

fn render_content_block(
    block: &aws_sdk_bedrockruntime::types::ContentBlock,
    text: &mut String,
    reasoning: &mut String,
    tool_calls: &mut Vec<ToolCallRequest>,
    content_json: &mut Vec<serde_json::Value>,
) {
    use aws_sdk_bedrockruntime::types as bt;
    match block {
        bt::ContentBlock::Text(t) => {
            text.push_str(t);
            content_json.push(serde_json::json!({ "text": t }));
        }
        bt::ContentBlock::ToolUse(tu) => {
            let input = tu.input().clone();
            let input_json = document_to_json(&input);
            content_json.push(serde_json::json!({
                "toolUse": {
                    "toolUseId": tu.tool_use_id(),
                    "name": tu.name(),
                    "input": input_json,
                }
            }));
            tool_calls.push(ToolCallRequest {
                id: ToolCallId::new(),
                name: tu.name().to_string(),
                arguments: input_json,
            });
        }
        bt::ContentBlock::ReasoningContent(bt::ReasoningContentBlock::ReasoningText(rt)) => {
            reasoning.push_str(rt.text());
        }
        _ => {}
    }
}

/// Convert a `aws_smithy_types::Document` (Bedrock's untyped JSON-shaped
/// value) into a `serde_json::Value`.
pub(super) fn document_to_json(doc: &aws_smithy_types::Document) -> serde_json::Value {
    use aws_smithy_types::Document;
    match doc {
        Document::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), document_to_json(v)))
                .collect(),
        ),
        Document::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(document_to_json).collect())
        }
        Document::Number(n) => serde_json::Number::from_f64(n.to_f64_lossy())
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Document::String(s) => serde_json::Value::String(s.clone()),
        Document::Bool(b) => serde_json::Value::Bool(*b),
        Document::Null => serde_json::Value::Null,
    }
}
