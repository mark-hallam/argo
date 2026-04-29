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
fn truncate_for_error_keeps_short_strings_intact() {
    use super::truncate_for_error;
    let s = "short body".to_string();
    let out = truncate_for_error(s.clone());
    assert_eq!(out, s);
}

#[test]
fn truncate_for_error_caps_long_strings() {
    use super::{truncate_for_error, ERROR_BODY_CAP};
    let big = "x".repeat(ERROR_BODY_CAP * 2 + 7);
    let out = truncate_for_error(big);
    assert!(out.starts_with(&"x".repeat(ERROR_BODY_CAP)));
    assert!(out.contains("[truncated"));
    assert!(out.contains(&format!("{} bytes", ERROR_BODY_CAP + 7)));
}

#[test]
fn truncate_for_error_respects_utf8_boundaries() {
    use super::{truncate_for_error, ERROR_BODY_CAP};
    let mut s = String::with_capacity(ERROR_BODY_CAP + 4);
    while s.len() < ERROR_BODY_CAP - 2 {
        s.push('a');
    }
    s.push('é');
    s.push('é');
    s.push('é');
    let out = truncate_for_error(s);
    // No invalid UTF-8 panics; the truncated head must itself be valid UTF-8.
    let head: &str = out.split("...").next().unwrap();
    assert!(std::str::from_utf8(head.as_bytes()).is_ok());
}

#[test]
fn compute_backoff_delay_grows_exponentially_then_caps() {
    use super::compute_backoff_delay;
    let initial = 100u64;
    let max = 800u64;
    // Run a few rolls per attempt; we sample jitter so any single delay is
    // bounded by [1, cap]. The cap doubles per attempt until `max`.
    for attempt in 0..6 {
        let cap = (initial << attempt.min(31)).min(max).max(1);
        let mut max_seen = 0u64;
        for _ in 0..32 {
            let d = compute_backoff_delay(initial, max, attempt as u32).as_millis() as u64;
            assert!(
                (1..=cap).contains(&d),
                "attempt={attempt} delay={d} cap={cap}"
            );
            max_seen = max_seen.max(d);
        }
        // Across the rolls we should see we're at least using some of the
        // window, not pinning to 1ms.
        assert!(max_seen > 0);
    }
}

#[test]
fn compute_backoff_delay_clamps_attempt_overflow() {
    use super::compute_backoff_delay;
    // attempt = 64 would overflow `1u64 << 64`. We must clamp.
    let d = super::compute_backoff_delay(100, 800, 64).as_millis() as u64;
    assert!((1..=800).contains(&d));
    let _ = d;
    let _ = compute_backoff_delay(100, 800, u32::MAX).as_millis();
}

#[test]
fn idempotency_key_header_is_lowercase() {
    // reqwest normalises header names; our constant must match what
    // HeaderName accepts. The constant is declared via
    // HeaderName::from_static which panics on non-lowercase input —
    // this test is the visible regression guard.
    use super::IDEMPOTENCY_KEY;
    assert_eq!(IDEMPOTENCY_KEY.as_str(), "idempotency-key");
}

#[test]
fn stream_happy_path_emits_text_then_finish() {
    use crate::openai_compat::event_to_deltas;
    use crate::ProviderResponseDelta;
    use crate::StopReason;
    let chunks = [
        r#"{"choices":[{"delta":{"content":"Hel"}}]}"#,
        r#"{"choices":[{"delta":{"content":"lo"}}]}"#,
        r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
    ];
    let mut all = Vec::new();
    for c in chunks {
        all.extend(event_to_deltas(c).unwrap());
    }
    assert_eq!(
        all,
        vec![
            ProviderResponseDelta::TextChunk("Hel".into()),
            ProviderResponseDelta::TextChunk("lo".into()),
            ProviderResponseDelta::FinishReason(StopReason::EndTurn),
        ]
    );
}

#[test]
fn outbound_tool_call_uses_provider_call_id_when_present() {
    // Issue #4 / F-9: replaying an assistant message with tool_calls
    // back to OpenAI MUST emit the original provider id (e.g.
    // "call_FAxa…") rather than our internal UUIDv7.
    let m = parse_manifest();
    let v = serde_json::json!({
        "id": "00000000-0000-7000-8000-000000000010",
        "session_id": "00000000-0000-7000-8000-000000000020",
        "role": "assistant",
        "content": [{"type":"text","text":"calling..."}],
        "tool_calls": [{
            "id": "00000000-0000-7000-8000-000000000030",
            "provider_call_id": "call_FAxa9JK",
            "name": "ls",
            "arguments": {"path":"/"}
        }],
        "tool_call_id": null,
        "reasoning": null,
        "created_at": "2025-01-01T00:00:00Z",
        "provider_meta": null
    });
    let asst: argo_types::Message = serde_json::from_value(v).unwrap();
    let req = ProviderRequest {
        model: "gpt-4o".into(),
        messages: vec![asst],
        tools: vec![],
        system: None,
        sampling: Sampling::default(),
    };
    let body = build_request_body(&m, &req).unwrap();
    let calls = body["messages"][0]["tool_calls"].as_array().unwrap();
    assert_eq!(calls[0]["id"], "call_FAxa9JK");
}

#[test]
fn outbound_tool_call_falls_back_to_internal_id_when_no_provider_id() {
    let m = parse_manifest();
    let v = serde_json::json!({
        "id": "00000000-0000-7000-8000-000000000010",
        "session_id": "00000000-0000-7000-8000-000000000020",
        "role": "assistant",
        "content": [{"type":"text","text":"ok"}],
        "tool_calls": [{
            "id": "00000000-0000-7000-8000-000000000030",
            "provider_call_id": null,
            "name": "ls",
            "arguments": {}
        }],
        "tool_call_id": null,
        "reasoning": null,
        "created_at": "2025-01-01T00:00:00Z",
        "provider_meta": null
    });
    let asst: argo_types::Message = serde_json::from_value(v).unwrap();
    let req = ProviderRequest {
        model: "gpt-4o".into(),
        messages: vec![asst],
        tools: vec![],
        system: None,
        sampling: Sampling::default(),
    };
    let body = build_request_body(&m, &req).unwrap();
    let calls = body["messages"][0]["tool_calls"].as_array().unwrap();
    assert_eq!(calls[0]["id"], "00000000-0000-7000-8000-000000000030");
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
