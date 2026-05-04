use super::wire::{apply_quirks, assemble_response, build_request_body, AnthropicResponse};
use super::AnthropicTransport;
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
id = "anthropic-test"
family = "anthropic"
display_name = "Anthropic Test"
base_url = "https://api.anthropic.com"
[auth]
api_key_env = "ANTHROPIC_API_KEY"
header_name = "x-api-key"
header_format = "{key}"
[transport]
streaming = "sse"
chat_path = "/v1/messages"
[tool_calls]
format = "anthropic_tool_use"
    "#
}

fn parse_manifest() -> ProviderManifest {
    ProviderManifest::from_toml_str(manifest_str()).unwrap()
}

#[test]
fn rejects_wrong_family() {
    let toml = manifest_str().replace("family = \"anthropic\"", "family = \"bedrock\"");
    let m = ProviderManifest::from_toml_str(&toml).unwrap();
    let err = AnthropicTransport::from_manifest(m, "sk-ant-x").unwrap_err();
    assert!(matches!(err, ArgoError::Provider(_)));
}

#[test]
fn rejects_empty_key() {
    let m = parse_manifest();
    let err = AnthropicTransport::from_manifest(m, "").unwrap_err();
    assert!(matches!(err, ArgoError::Provider(_)));
}

#[test]
fn auth_header_format_substitutes_key() {
    let m = parse_manifest();
    let t = AnthropicTransport::from_manifest(m, "sk-ant-12345").unwrap();
    assert_eq!(t.auth_header_value(), "sk-ant-12345");
}

#[test]
fn endpoint_uses_chat_path() {
    let m = parse_manifest();
    let t = AnthropicTransport::from_manifest(m, "sk-ant-x").unwrap();
    assert_eq!(t.endpoint(), "https://api.anthropic.com/v1/messages");
}

#[test]
fn build_request_serializes_with_max_tokens() {
    let m = parse_manifest();
    let req = ProviderRequest {
        model: "claude-haiku-4-5".into(),
        messages: vec![user_message("hello")],
        tools: vec![ToolSchema {
            name: "ls".into(),
            description: "list".into(),
            parameters: serde_json::json!({"type":"object"}),
        }],
        system: Some("be terse".into()),
        sampling: Sampling::default(),
    };
    let body = build_request_body(&m, &req).unwrap();
    assert_eq!(body["model"], "claude-haiku-4-5");
    assert_eq!(body["system"], "be terse");
    assert_eq!(body["max_tokens"], 4096);
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"][0]["type"], "text");
    assert_eq!(body["tools"][0]["name"], "ls");
    assert_eq!(body["tools"][0]["input_schema"]["type"], "object");
}

#[test]
fn quirks_send_thinking_blocks_enables_thinking_param() {
    let toml = format!(
        "{}\n[quirks]\nsend-thinking-blocks = true",
        manifest_str().trim_end()
    );
    let m = ProviderManifest::from_toml_str(&toml).unwrap();
    let mut body = serde_json::json!({"model":"x"});
    apply_quirks(&mut body, &m);
    assert_eq!(body["thinking"]["type"], "enabled");
}

#[test]
fn quirks_prompt_caching_breakpoints_tags_system() {
    let toml = format!(
        "{}\n[quirks]\nprompt-caching-breakpoints = [\"system\"]",
        manifest_str().trim_end()
    );
    let m = ProviderManifest::from_toml_str(&toml).unwrap();
    let mut body = serde_json::json!({"system":"be terse"});
    apply_quirks(&mut body, &m);
    let arr = body["system"].as_array().unwrap();
    assert_eq!(arr[0]["cache_control"]["type"], "ephemeral");
}

#[test]
fn quirks_prompt_caching_breakpoints_tags_tools() {
    let toml = format!(
        "{}\n[quirks]\nprompt-caching-breakpoints = [\"tools\"]",
        manifest_str().trim_end()
    );
    let m = ProviderManifest::from_toml_str(&toml).unwrap();
    let mut body = serde_json::json!({
        "tools": [{"name":"ls","description":"d","input_schema":{}}]
    });
    apply_quirks(&mut body, &m);
    assert_eq!(body["tools"][0]["cache_control"]["type"], "ephemeral");
}

#[test]
fn assemble_response_picks_text_thinking_and_tool_use() {
    let parsed: AnthropicResponse = serde_json::from_value(serde_json::json!({
        "id": "msg_x",
        "model": "claude-haiku-4-5",
        "stop_reason": "tool_use",
        "content": [
            {"type": "thinking", "thinking": "let me think"},
            {"type": "text", "text": "Calling tool."},
            {"type": "tool_use", "id": "toolu_1", "name": "ls", "input": {"path": "/"}}
        ],
        "usage": {"input_tokens": 12, "output_tokens": 7}
    }))
    .unwrap();
    let r = assemble_response(parsed, serde_json::json!({}));
    assert_eq!(r.text.as_deref(), Some("Calling tool."));
    assert_eq!(r.reasoning.as_deref(), Some("let me think"));
    assert_eq!(r.stop_reason, StopReason::ToolUse);
    assert_eq!(r.tokens_in, Some(12));
    assert_eq!(r.tokens_out, Some(7));
    assert_eq!(r.tool_calls.len(), 1);
    assert_eq!(r.tool_calls[0].name, "ls");
    assert_eq!(r.tool_calls[0].arguments["path"], "/");
}

/// #719: when we replay a multi-turn conversation back to Anthropic,
/// the tool_use block in the assistant message and the tool_result
/// block in the following user message MUST carry matching ids —
/// otherwise Anthropic returns HTTP 400 "unexpected `tool_use_id`
/// found in `tool_result` blocks. Each `tool_result` block must
/// have a corresponding `tool_use` block in the previous message."
///
/// Pre-#719, `tool_call_to_anthropic` preferred `provider_call_id`
/// (the `toolu_…` Anthropic gave us) while `push_tool_result` only
/// had `Message.tool_call_id` (our internal UUID, never plumbed
/// through `ToolCallResult`). The two sides emitted different ids
/// and Anthropic rejected the second-round request — the exact
/// failure live `argo work serve --provider anthropic` hit.
///
/// This test pins the round-trip invariant.
#[test]
fn tool_use_and_tool_result_ids_match_in_multi_turn_request() {
    let m = parse_manifest();
    // Same internal ToolCallId on both sides; tool_call's
    // provider_call_id is set to a fake "toolu_…" so we can confirm
    // it's NOT what gets emitted (the bug pre-#719).
    let internal_id = "00000000-0000-7000-8000-000000000030";
    let provider_id = "toolu_abcDEF123";

    let user: Message = serde_json::from_value(serde_json::json!({
        "id": "00000000-0000-7000-8000-000000000010",
        "session_id": "00000000-0000-7000-8000-000000000001",
        "role": "user",
        "content": [{"type":"text","text":"list /"}],
        "tool_calls": [],
        "tool_call_id": null,
        "reasoning": null,
        "created_at": "2025-01-01T00:00:00Z",
        "provider_meta": null
    })).unwrap();

    let assistant: Message = serde_json::from_value(serde_json::json!({
        "id": "00000000-0000-7000-8000-000000000011",
        "session_id": "00000000-0000-7000-8000-000000000001",
        "role": "assistant",
        "content": [{"type":"text","text":"ok, calling ls."}],
        "tool_calls": [{
            "id": internal_id,
            "provider_call_id": provider_id,
            "name": "ls",
            "arguments": {"path":"/"}
        }],
        "tool_call_id": null,
        "reasoning": null,
        "created_at": "2025-01-01T00:00:00Z",
        "provider_meta": null
    })).unwrap();

    let tool: Message = serde_json::from_value(serde_json::json!({
        "id": "00000000-0000-7000-8000-000000000012",
        "session_id": "00000000-0000-7000-8000-000000000001",
        "role": "tool",
        "content": [{
            "type": "tool_result",
            "id": internal_id,
            "is_error": false,
            "content": [{"type":"text","text":"/etc /var"}],
            "duration_ms": 12,
            "tokens_in": null,
            "tokens_out": null
        }],
        "tool_calls": [],
        "tool_call_id": internal_id,
        "reasoning": null,
        "created_at": "2025-01-01T00:00:00Z",
        "provider_meta": null
    })).unwrap();

    let req = ProviderRequest {
        model: "claude-haiku-4-5".into(),
        messages: vec![user, assistant, tool],
        tools: vec![],
        system: None,
        sampling: Sampling::default(),
    };
    let body = build_request_body(&m, &req).unwrap();
    let messages = body["messages"].as_array().expect("messages array");
    assert_eq!(messages.len(), 3, "expected user / assistant / user-tool_result");

    // Locate the assistant tool_use block's id and the user
    // tool_result block's tool_use_id, then assert they're equal.
    let asst_content = messages[1]["content"].as_array().expect("assistant content");
    let tool_use_id = asst_content
        .iter()
        .find_map(|b| {
            (b["type"] == "tool_use")
                .then(|| b["id"].as_str().map(str::to_string))
                .flatten()
        })
        .expect("assistant message must carry a tool_use block");

    let tool_content = messages[2]["content"].as_array().expect("tool_result content");
    let tool_result_ref = tool_content
        .iter()
        .find_map(|b| {
            (b["type"] == "tool_result")
                .then(|| b["tool_use_id"].as_str().map(str::to_string))
                .flatten()
        })
        .expect("user message must carry a tool_result block");

    assert_eq!(
        tool_use_id, tool_result_ref,
        "tool_use.id ({tool_use_id}) must match tool_result.tool_use_id ({tool_result_ref}) — \
         Anthropic 400's the request otherwise"
    );
}

#[test]
fn assemble_response_handles_missing_stop_reason_as_end_turn() {
    let parsed: AnthropicResponse = serde_json::from_value(serde_json::json!({
        "content":[{"type":"text","text":"hi"}]
    }))
    .unwrap();
    let r = assemble_response(parsed, serde_json::json!({}));
    assert_eq!(r.stop_reason, StopReason::EndTurn);
}
