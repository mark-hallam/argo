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
fn empty_body_yields_empty_extracts_for_implemented_formats() {
    let body = serde_json::json!({});
    for fmt in [
        ToolCallFormat::OpenaiToolCalls,
        ToolCallFormat::OpenaiResponseTools,
        ToolCallFormat::AnthropicToolUse,
        ToolCallFormat::BedrockToolUse,
    ] {
        let calls = extract_tool_calls(fmt, &body).unwrap();
        assert!(
            calls.is_empty(),
            "{fmt:?} should produce no calls for an empty body"
        );
    }
}

#[test]
fn openai_extractor_preserves_provider_call_id() {
    // Issue #4 / F-9 regression — the OpenAI `id` field must round-trip
    // through ToolCallRequest::provider_call_id so multi-turn tool
    // sequences send back the call_id OpenAI originally issued.
    let body = serde_json::json!({
        "choices": [{
            "message": {
                "tool_calls": [{
                    "id": "call_FAxa9JK",
                    "type": "function",
                    "function": { "name": "ls", "arguments": "{}" }
                }]
            }
        }]
    });
    let calls = openai_tool_calls(&body).unwrap();
    assert_eq!(calls[0].provider_call_id.as_deref(), Some("call_FAxa9JK"));
}

#[test]
fn anthropic_extractor_preserves_provider_call_id() {
    let body = serde_json::json!({
        "content": [
            {"type": "tool_use", "id": "toolu_01abc", "name": "ls", "input": {}}
        ]
    });
    let calls = anthropic_tool_use(&body).unwrap();
    assert_eq!(calls[0].provider_call_id.as_deref(), Some("toolu_01abc"));
}

#[test]
fn responses_extractor_preserves_provider_call_id() {
    let body = serde_json::json!({
        "output": [
            {"type": "function_call", "call_id": "call_resp_xyz",
             "name": "ls", "arguments": "{}"}
        ]
    });
    let calls = openai_response_tools(&body).unwrap();
    assert_eq!(calls[0].provider_call_id.as_deref(), Some("call_resp_xyz"));
}

#[test]
fn bedrock_extractor_preserves_provider_call_id() {
    let body = serde_json::json!({
        "output": { "message": { "content": [
            {"toolUse": {"toolUseId":"tooluse_aws_1","name":"ls","input":{}}}
        ]}}
    });
    let calls = bedrock_tool_use(&body).unwrap();
    assert_eq!(calls[0].provider_call_id.as_deref(), Some("tooluse_aws_1"));
}

#[test]
fn extractors_emit_none_provider_call_id_when_id_missing() {
    let body = serde_json::json!({
        "choices": [{
            "message": {
                "tool_calls": [{
                    "type": "function",
                    "function": { "name": "ls", "arguments": "{}" }
                }]
            }
        }]
    });
    let calls = openai_tool_calls(&body).unwrap();
    assert!(calls[0].provider_call_id.is_none());
}

#[test]
fn llm_extracted_remains_a_phase2_stub() {
    let body = serde_json::json!({});
    let err = extract_tool_calls(ToolCallFormat::LlmExtracted, &body).unwrap_err();
    match err {
        ArgoError::Provider(msg) => assert_eq!(msg, NOT_YET),
        other => panic!("wrong error variant: {other:?}"),
    }
}
