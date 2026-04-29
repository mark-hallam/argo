use super::wire::{assemble_response, build_request_body, OpenAiResponse};
use super::OpenAiCompatTransport;
use crate::manifest::ProviderManifest;
use crate::transport::{ProviderRequest, Sampling, StopReason, ToolSchema, Transport};
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
id = "openai-test"
family = "openai-compatible"
display_name = "OpenAI Test"
base_url = "https://api.test"
[auth]
api_key_env = "OPENAI_API_KEY"
[transport]
streaming = "sse"
[tool_calls]
format = "openai_tool_calls"
    "#
}

fn parse_manifest() -> ProviderManifest {
    ProviderManifest::from_toml_str(manifest_str()).unwrap()
}

#[test]
fn rejects_wrong_family() {
    let toml = manifest_str().replace("openai-compatible", "anthropic");
    let m = ProviderManifest::from_toml_str(&toml).unwrap();
    let err = OpenAiCompatTransport::from_manifest(m, "sk-x").unwrap_err();
    assert!(matches!(err, ArgoError::Provider(_)));
}

#[test]
fn rejects_empty_key() {
    let m = parse_manifest();
    let err = OpenAiCompatTransport::from_manifest(m, "").unwrap_err();
    assert!(matches!(err, ArgoError::Provider(_)));
}

#[test]
fn auth_header_substitutes_key() {
    let m = parse_manifest();
    let t = OpenAiCompatTransport::from_manifest(m, "sk-12345").unwrap();
    assert_eq!(t.auth_header_value(), "Bearer sk-12345");
}

#[test]
fn endpoint_concatenates_base_and_path() {
    let m = parse_manifest();
    let t = OpenAiCompatTransport::from_manifest(m, "sk-x").unwrap();
    assert_eq!(t.endpoint(), "https://api.test/v1/chat/completions");
}

#[test]
fn capabilities_cover_api_key_env_read() {
    use argo_types::Capability;
    let m = parse_manifest();
    let t = OpenAiCompatTransport::from_manifest(m, "sk-x").unwrap();
    assert!(t.capabilities().covers(&Capability::EnvRead {
        var_glob: "OPENAI_API_KEY".into()
    }));
}

#[test]
fn build_request_body_serializes_messages_and_tools() {
    let m = parse_manifest();
    let req = ProviderRequest {
        model: "gpt-4o-mini".into(),
        messages: vec![user_message("hello")],
        tools: vec![ToolSchema {
            name: "ls".into(),
            description: "list files".into(),
            parameters: serde_json::json!({"type":"object"}),
        }],
        system: Some("be terse".into()),
        sampling: Sampling {
            temperature: Some(0.2),
            max_tokens: Some(64),
            ..Default::default()
        },
    };
    let body = build_request_body(&m, &req).unwrap();
    assert_eq!(body["model"], "gpt-4o-mini");
    assert_eq!(body["stream"], false);
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["messages"][1]["role"], "user");
    let temp = body["temperature"].as_f64().unwrap();
    assert!((temp - 0.2).abs() < 1e-3);
    assert_eq!(body["max_tokens"], 64);
    assert_eq!(body["tools"][0]["function"]["name"], "ls");
}

#[test]
fn assemble_response_picks_text_and_finish_reason() {
    let parsed: OpenAiResponse = serde_json::from_value(serde_json::json!({
        "id": "resp1",
        "model": "gpt-4o-mini",
        "choices": [{
            "message": { "content": "hi" },
            "finish_reason": "stop"
        }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 3 }
    }))
    .unwrap();
    let resp = assemble_response(parsed, vec![], serde_json::json!({}));
    assert_eq!(resp.stop_reason, StopReason::EndTurn);
    assert_eq!(resp.tokens_in, Some(10));
    assert_eq!(resp.tokens_out, Some(3));
    assert_eq!(resp.text.as_deref(), Some("hi"));
}

#[test]
fn assemble_response_omits_empty_text() {
    let parsed: OpenAiResponse = serde_json::from_value(serde_json::json!({
        "choices": [{ "message": { "content": "" }, "finish_reason": "tool_calls" }]
    }))
    .unwrap();
    let resp = assemble_response(parsed, vec![], serde_json::json!({}));
    assert!(resp.text.is_none());
    assert_eq!(resp.stop_reason, StopReason::ToolUse);
}

#[test]
fn map_finish_reason_covers_known_values() {
    use super::wire::map_finish_reason;
    assert_eq!(map_finish_reason("stop"), StopReason::EndTurn);
    assert_eq!(map_finish_reason("tool_calls"), StopReason::ToolUse);
    assert_eq!(map_finish_reason("length"), StopReason::MaxTokens);
    assert_eq!(
        map_finish_reason("content_filter"),
        StopReason::ContentFilter
    );
    match map_finish_reason("weird") {
        StopReason::Other(s) => assert_eq!(s, "weird"),
        _ => panic!("expected Other"),
    }
}
