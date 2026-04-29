use super::wire::{assemble_response, build_request_body, ResponsesEnvelope};
use super::OpenAiResponsesTransport;
use crate::manifest::ProviderManifest;
use crate::transport::{ProviderRequest, Sampling, StopReason, ToolSchema};
use argo_types::{ArgoError, Message};

fn user_message(text: &str) -> Message {
    let v = serde_json::json!({
        "id": "00000000-0000-7000-8000-000000000001",
        "session_id": "00000000-0000-7000-8000-000000000002",
        "role": "user",
        "content": [{"type": "text", "text": text}],
        "tool_calls": [],
        "tool_call_id": null,
        "reasoning": null,
        "created_at": "2025-01-01T00:00:00Z",
        "provider_meta": null
    });
    serde_json::from_value(v).unwrap()
}

fn manifest_str() -> &'static str {
    r#"
[provider]
id = "openai-responses-test"
family = "openai-responses"
display_name = "OpenAI Responses Test"
base_url = "https://api.openai.com"
[auth]
api_key_env = "OPENAI_API_KEY"
[transport]
streaming = "sse"
chat_path = "/v1/responses"
[tool_calls]
format = "openai_response_tools"
    "#
}

fn parse_manifest() -> ProviderManifest {
    ProviderManifest::from_toml_str(manifest_str()).unwrap()
}

#[test]
fn rejects_wrong_family() {
    let toml = manifest_str().replace("openai-responses", "openai-compatible");
    let m = ProviderManifest::from_toml_str(&toml).unwrap();
    let err = OpenAiResponsesTransport::from_manifest(m, "sk-x").unwrap_err();
    assert!(matches!(err, ArgoError::Provider(_)));
}

#[test]
fn build_request_emits_input_array_with_message_items() {
    let m = parse_manifest();
    let req = ProviderRequest {
        model: "gpt-5.0".into(),
        messages: vec![user_message("hello")],
        tools: vec![ToolSchema {
            name: "ls".into(),
            description: "list".into(),
            parameters: serde_json::json!({"type":"object"}),
        }],
        system: Some("be terse".into()),
        sampling: Sampling {
            max_tokens: Some(64),
            ..Default::default()
        },
    };
    let body = build_request_body(&m, &req).unwrap();
    assert_eq!(body["model"], "gpt-5.0");
    assert_eq!(body["instructions"], "be terse");
    assert_eq!(body["max_output_tokens"], 64);
    assert_eq!(body["input"][0]["type"], "message");
    assert_eq!(body["input"][0]["role"], "user");
    assert_eq!(body["input"][0]["content"][0]["type"], "input_text");
    assert_eq!(body["tools"][0]["type"], "function");
    assert_eq!(body["tools"][0]["name"], "ls");
}

#[test]
fn assemble_response_picks_text_and_function_call() {
    let env: ResponsesEnvelope = serde_json::from_value(serde_json::json!({
        "id":"resp_1","status":"completed",
        "output":[
            {"type":"reasoning","summary":[{"type":"summary_text","text":"thinking"}]},
            {"type":"message","content":[{"type":"output_text","text":"Hi."}]},
            {"type":"function_call","call_id":"call_1","name":"ls",
             "arguments":"{\"path\":\"/\"}"}
        ],
        "usage":{"input_tokens":12,"output_tokens":3}
    }))
    .unwrap();
    let r = assemble_response(env, serde_json::json!({}));
    assert_eq!(r.text.as_deref(), Some("Hi."));
    assert_eq!(r.reasoning.as_deref(), Some("thinking"));
    assert_eq!(r.tool_calls.len(), 1);
    assert_eq!(r.tool_calls[0].name, "ls");
    assert_eq!(r.tool_calls[0].arguments["path"], "/");
    // Tool calls present → ToolUse stop reason regardless of `status`.
    assert_eq!(r.stop_reason, StopReason::ToolUse);
}

#[test]
fn assemble_response_completed_without_tool_calls_is_end_turn() {
    let env: ResponsesEnvelope = serde_json::from_value(serde_json::json!({
        "id":"resp_1","status":"completed",
        "output":[
            {"type":"message","content":[{"type":"output_text","text":"Hi."}]}
        ]
    }))
    .unwrap();
    let r = assemble_response(env, serde_json::json!({}));
    assert_eq!(r.stop_reason, StopReason::EndTurn);
}

#[test]
fn assemble_response_incomplete_status_maps_to_max_tokens() {
    let env: ResponsesEnvelope = serde_json::from_value(serde_json::json!({
        "status":"incomplete",
        "output":[]
    }))
    .unwrap();
    let r = assemble_response(env, serde_json::json!({}));
    assert_eq!(r.stop_reason, StopReason::MaxTokens);
}
