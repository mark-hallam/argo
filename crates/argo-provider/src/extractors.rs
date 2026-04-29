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

/// OpenAI Responses API tool blocks. **Stub — Agent F.**
pub fn openai_response_tools(_body: &serde_json::Value) -> ArgoResult<Vec<ToolCallRequest>> {
    Err(ArgoError::Provider(NOT_YET.into()))
}

/// Anthropic content-block `type = "tool_use"`. **Stub — Agent F.**
pub fn anthropic_tool_use(_body: &serde_json::Value) -> ArgoResult<Vec<ToolCallRequest>> {
    Err(ArgoError::Provider(NOT_YET.into()))
}

/// AWS Bedrock InvokeModel tool-use envelope. **Stub — Agent F.**
pub fn bedrock_tool_use(_body: &serde_json::Value) -> ArgoResult<Vec<ToolCallRequest>> {
    Err(ArgoError::Provider(NOT_YET.into()))
}

/// Hermes-style `<tool_call>{...}</tool_call>` XML in a text body.
/// **Stub — Agent F. Hermes fixture corpus lands then.**
pub fn xml_tool_call(_body: &serde_json::Value) -> ArgoResult<Vec<ToolCallRequest>> {
    Err(ArgoError::Provider(NOT_YET.into()))
}

/// Plain-text format requiring a second-pass model extract. **Stub — Agent F.**
pub fn llm_extracted(_body: &serde_json::Value) -> ArgoResult<Vec<ToolCallRequest>> {
    Err(ArgoError::Provider(NOT_YET.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_no_tool_calls_is_empty() {
        let body = serde_json::json!({
            "choices": [{ "message": { "content": "hi" } }]
        });
        let calls = openai_tool_calls(&body).unwrap();
        assert!(calls.is_empty());
    }

    #[test]
    fn openai_extracts_multiple_tool_calls() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "tool_calls": [
                        { "id": "1", "type": "function",
                          "function": { "name": "a", "arguments": "{\"x\":1}" } },
                        { "id": "2", "type": "function",
                          "function": { "name": "b", "arguments": "{\"y\":2}" } }
                    ]
                }
            }]
        });
        let calls = openai_tool_calls(&body).unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "a");
        assert_eq!(calls[0].arguments["x"], 1);
        assert_eq!(calls[1].name, "b");
        assert_eq!(calls[1].arguments["y"], 2);
    }

    #[test]
    fn openai_handles_object_arguments() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "1",
                        "type": "function",
                        "function": { "name": "a", "arguments": { "x": 1 } }
                    }]
                }
            }]
        });
        let calls = openai_tool_calls(&body).unwrap();
        assert_eq!(calls[0].arguments["x"], 1);
    }

    #[test]
    fn openai_handles_empty_arguments_string() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "1",
                        "type": "function",
                        "function": { "name": "noop", "arguments": "" }
                    }]
                }
            }]
        });
        let calls = openai_tool_calls(&body).unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].arguments.is_object());
    }

    #[test]
    fn openai_missing_function_name_is_provider_error() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "1",
                        "type": "function",
                        "function": { "arguments": "{}" }
                    }]
                }
            }]
        });
        let err = openai_tool_calls(&body).unwrap_err();
        assert!(matches!(err, ArgoError::Provider(_)));
    }

    #[test]
    fn openai_invalid_arguments_json_is_provider_error() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "1",
                        "type": "function",
                        "function": { "name": "a", "arguments": "not-json" }
                    }]
                }
            }]
        });
        let err = openai_tool_calls(&body).unwrap_err();
        assert!(matches!(err, ArgoError::Provider(_)));
    }

    #[test]
    fn dispatcher_routes_format() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "1", "type": "function",
                        "function": { "name": "a", "arguments": "{}" }
                    }]
                }
            }]
        });
        let calls = extract_tool_calls(ToolCallFormat::OpenaiToolCalls, &body).unwrap();
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn stubs_return_provider_error() {
        let body = serde_json::json!({});
        for fmt in [
            ToolCallFormat::OpenaiResponseTools,
            ToolCallFormat::AnthropicToolUse,
            ToolCallFormat::BedrockToolUse,
            ToolCallFormat::XmlToolCall,
            ToolCallFormat::LlmExtracted,
        ] {
            let err = extract_tool_calls(fmt, &body).unwrap_err();
            match err {
                ArgoError::Provider(msg) => assert_eq!(msg, NOT_YET),
                other => panic!("wrong error variant: {other:?}"),
            }
        }
    }
}
