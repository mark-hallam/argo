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
fn llm_extracted_remains_a_phase2_stub() {
    let body = serde_json::json!({});
    let err = extract_tool_calls(ToolCallFormat::LlmExtracted, &body).unwrap_err();
    match err {
        ArgoError::Provider(msg) => assert_eq!(msg, NOT_YET),
        other => panic!("wrong error variant: {other:?}"),
    }
}
