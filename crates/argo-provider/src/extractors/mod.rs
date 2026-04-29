//! Tool-call extractors — one per wire format.
//!
//! Spec: `docs/plan/argo-build-plan.md` §8 / Phase 2 plan.
//!
//! Phase 2 ships **only** `openai_tool_calls` end-to-end. The other five
//! return `ArgoError::Provider("not yet implemented in Phase 2")` — Agent F
//! will land them.
use crate::manifest::ToolCallFormat;
use argo_types::{ArgoError, ArgoResult, ToolCallId, ToolCallRequest};

/// Dispatch to the right extractor based on the manifest's `[tool_calls]`
/// `format` field.
///
/// `body_json` is the *parsed* provider response (after JSON deserialization
/// of either a non-stream body or a fully-collected SSE stream).
///
/// ```
/// use argo_provider::{extract_tool_calls, ToolCallFormat};
/// let body = serde_json::json!({
///     "choices": [{
///         "message": {
///             "tool_calls": [{
///                 "id": "call_123",
///                 "type": "function",
///                 "function": {
///                     "name": "file_read",
///                     "arguments": "{\"path\":\"/tmp/x\"}"
///                 }
///             }]
///         }
///     }]
/// });
/// let calls = extract_tool_calls(ToolCallFormat::OpenaiToolCalls, &body).unwrap();
/// assert_eq!(calls.len(), 1);
/// assert_eq!(calls[0].name, "file_read");
/// ```
pub fn extract_tool_calls(
    format: ToolCallFormat,
    body_json: &serde_json::Value,
) -> ArgoResult<Vec<ToolCallRequest>> {
    match format {
        ToolCallFormat::OpenaiToolCalls => openai_tool_calls(body_json),
        ToolCallFormat::OpenaiResponseTools => openai_response_tools(body_json),
        ToolCallFormat::AnthropicToolUse => anthropic_tool_use(body_json),
        ToolCallFormat::BedrockToolUse => bedrock_tool_use(body_json),
        ToolCallFormat::XmlToolCall => xml_tool_call(body_json),
        ToolCallFormat::LlmExtracted => llm_extracted(body_json),
    }
}

const NOT_YET: &str = "not yet implemented in Phase 2";

/// OpenAI Chat Completions `choices[].message.tool_calls[]`.
///
/// ```
/// use argo_provider::openai_tool_calls;
/// let body = serde_json::json!({
///     "choices": [{
///         "message": {
///             "tool_calls": [{
///                 "id": "call_1",
///                 "type": "function",
///                 "function": { "name": "ls", "arguments": "{}" }
///             }]
///         }
///     }]
/// });
/// let calls = openai_tool_calls(&body).unwrap();
/// assert_eq!(calls.len(), 1);
/// ```
pub fn openai_tool_calls(body: &serde_json::Value) -> ArgoResult<Vec<ToolCallRequest>> {
    let Some(choices) = body.get("choices").and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for choice in choices {
        let Some(tool_calls) = choice
            .get("message")
            .and_then(|m| m.get("tool_calls"))
            .and_then(|v| v.as_array())
        else {
            continue;
        };
        for tc in tool_calls {
            out.push(parse_openai_tool_call(tc)?);
        }
    }
    Ok(out)
}

fn parse_openai_tool_call(tc: &serde_json::Value) -> ArgoResult<ToolCallRequest> {
    let function = tc
        .get("function")
        .ok_or_else(|| ArgoError::Provider("openai tool_call missing `function`".into()))?;
    let name = function
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ArgoError::Provider("openai tool_call missing function.name".into()))?
        .to_string();
    let arguments = parse_openai_arguments(function)?;
    Ok(ToolCallRequest {
        id: ToolCallId::new(),
        name,
        arguments,
    })
}

fn parse_openai_arguments(function: &serde_json::Value) -> ArgoResult<serde_json::Value> {
    match function.get("arguments") {
        None => Ok(serde_json::Value::Object(serde_json::Map::new())),
        Some(serde_json::Value::String(s)) if s.is_empty() => {
            Ok(serde_json::Value::Object(serde_json::Map::new()))
        }
        Some(serde_json::Value::String(s)) => serde_json::from_str(s)
            .map_err(|e| ArgoError::Provider(format!("openai arguments JSON parse: {e}"))),
        Some(v) => Ok(v.clone()),
    }
}

/// OpenAI Responses API tool blocks. The body is the parsed `/v1/responses`
/// envelope; `output[]` is an array of items one of which can be
/// `function_call`.
///
/// ```
/// use argo_provider::openai_response_tools;
/// let body = serde_json::json!({
///     "output": [
///         {"type": "message", "content": [{"type":"output_text","text":"hi"}]},
///         {"type": "function_call", "call_id": "call_x",
///          "name": "ls", "arguments": "{\"path\":\"/\"}"}
///     ]
/// });
/// let calls = openai_response_tools(&body).unwrap();
/// assert_eq!(calls.len(), 1);
/// ```
pub fn openai_response_tools(body: &serde_json::Value) -> ArgoResult<Vec<ToolCallRequest>> {
    let Some(output) = body.get("output").and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for item in output {
        if item.get("type").and_then(|v| v.as_str()) != Some("function_call") {
            continue;
        }
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ArgoError::Provider("openai_response_tools missing `name`".into()))?
            .to_string();
        let arguments = parse_string_or_object_arguments(item.get("arguments"))?;
        out.push(ToolCallRequest {
            id: ToolCallId::new(),
            name,
            arguments,
        });
    }
    Ok(out)
}

fn parse_string_or_object_arguments(
    v: Option<&serde_json::Value>,
) -> ArgoResult<serde_json::Value> {
    match v {
        None => Ok(serde_json::Value::Object(serde_json::Map::new())),
        Some(serde_json::Value::String(s)) if s.is_empty() => {
            Ok(serde_json::Value::Object(serde_json::Map::new()))
        }
        Some(serde_json::Value::String(s)) => serde_json::from_str(s)
            .map_err(|e| ArgoError::Provider(format!("openai_response_tools arguments: {e}"))),
        Some(other) => Ok(other.clone()),
    }
}

/// Anthropic content-block `type = "tool_use"`. The body is the parsed
/// `/v1/messages` response; `content[]` carries text, thinking, and
/// `tool_use` blocks side-by-side.
///
/// ```
/// use argo_provider::anthropic_tool_use;
/// let body = serde_json::json!({
///     "content": [
///         {"type": "text", "text": "Calling tool."},
///         {"type": "tool_use", "id": "toolu_x",
///          "name": "ls", "input": {"path": "/"}}
///     ]
/// });
/// let calls = anthropic_tool_use(&body).unwrap();
/// assert_eq!(calls.len(), 1);
/// assert_eq!(calls[0].name, "ls");
/// ```
pub fn anthropic_tool_use(body: &serde_json::Value) -> ArgoResult<Vec<ToolCallRequest>> {
    let Some(blocks) = body.get("content").and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for block in blocks {
        if block.get("type").and_then(|v| v.as_str()) != Some("tool_use") {
            continue;
        }
        let name = block
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ArgoError::Provider("anthropic tool_use missing `name`".into()))?
            .to_string();
        let arguments = block
            .get("input")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        out.push(ToolCallRequest {
            id: ToolCallId::new(),
            name,
            arguments,
        });
    }
    Ok(out)
}

/// AWS Bedrock Converse-API `toolUse` blocks. The body is the JSON
/// envelope rendered from a `ConverseOutput`:
/// `{ "output": { "message": { "content": [{"toolUse": {...}}] } } }`.
///
/// ```
/// use argo_provider::bedrock_tool_use;
/// let body = serde_json::json!({
///     "output": { "message": { "role": "assistant", "content": [
///         {"text": "calling tool"},
///         {"toolUse": {"toolUseId":"tool_1","name":"ls","input":{"path":"/"}}}
///     ]}}
/// });
/// let calls = bedrock_tool_use(&body).unwrap();
/// assert_eq!(calls.len(), 1);
/// assert_eq!(calls[0].name, "ls");
/// ```
pub fn bedrock_tool_use(body: &serde_json::Value) -> ArgoResult<Vec<ToolCallRequest>> {
    let Some(content) = body
        .get("output")
        .and_then(|o| o.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for block in content {
        let Some(tu) = block.get("toolUse") else {
            continue;
        };
        let name = tu
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ArgoError::Provider("bedrock toolUse missing `name`".into()))?
            .to_string();
        let arguments = tu
            .get("input")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        out.push(ToolCallRequest {
            id: ToolCallId::new(),
            name,
            arguments,
        });
    }
    Ok(out)
}

/// Hermes-style `<tool_call>{...}</tool_call>` XML embedded in a Chat
/// Completions text body.
///
/// Hermes Function Calling Format (NousResearch) emits tool calls inline as
/// XML wrappers around a JSON payload:
///
/// ```text
/// <tool_call>
/// {"name": "ls", "arguments": {"path": "/"}}
/// </tool_call>
/// ```
///
/// The extractor scans every `choices[].message.content` string for
/// `<tool_call>...</tool_call>` blocks, parses the inner JSON, and emits
/// one [`ToolCallRequest`] per block.
///
/// ```
/// use argo_provider::xml_tool_call;
/// let body = serde_json::json!({
///     "choices": [{"message": {"content":
///         "Sure.\n<tool_call>\n{\"name\": \"ls\", \"arguments\": {\"path\": \"/\"}}\n</tool_call>"
///     }}]
/// });
/// let calls = xml_tool_call(&body).unwrap();
/// assert_eq!(calls.len(), 1);
/// assert_eq!(calls[0].name, "ls");
/// assert_eq!(calls[0].arguments["path"], "/");
/// ```
pub fn xml_tool_call(body: &serde_json::Value) -> ArgoResult<Vec<ToolCallRequest>> {
    let mut out = Vec::new();
    let Some(choices) = body.get("choices").and_then(|v| v.as_array()) else {
        return Ok(out);
    };
    for choice in choices {
        let text = choice
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");
        for raw in scan_tool_call_blocks(text) {
            out.push(parse_hermes_block(raw)?);
        }
    }
    Ok(out)
}

const HERMES_OPEN: &str = "<tool_call>";
const HERMES_CLOSE: &str = "</tool_call>";

fn scan_tool_call_blocks(text: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(open_rel) = text[cursor..].find(HERMES_OPEN) {
        let open_abs = cursor + open_rel;
        let after_open = open_abs + HERMES_OPEN.len();
        let Some(close_rel) = text[after_open..].find(HERMES_CLOSE) else {
            break;
        };
        let close_abs = after_open + close_rel;
        blocks.push(text[after_open..close_abs].trim());
        cursor = close_abs + HERMES_CLOSE.len();
    }
    blocks
}

fn parse_hermes_block(raw: &str) -> ArgoResult<ToolCallRequest> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| ArgoError::Provider(format!("xml_tool_call body JSON: {e}")))?;
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ArgoError::Provider("xml_tool_call missing `name`".into()))?
        .to_string();
    let arguments = value
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    Ok(ToolCallRequest {
        id: ToolCallId::new(),
        name,
        arguments,
    })
}

/// Plain-text format requiring a second-pass model extract. **Stub — Agent F.**
pub fn llm_extracted(_body: &serde_json::Value) -> ArgoResult<Vec<ToolCallRequest>> {
    Err(ArgoError::Provider(NOT_YET.into()))
}

#[cfg(test)]
mod tests;
