use super::streaming::{handle_stream_event, StreamCore};
use super::wire::{build_request_body, document_to_json};
use crate::manifest::ProviderManifest;
use crate::transport::{ProviderRequest, ProviderResponseDelta, Sampling, StopReason, ToolSchema};
use argo_types::Message;
use aws_sdk_bedrockruntime::types as bt;
use aws_smithy_types::Document;

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
id = "bedrock-test"
family = "bedrock"
display_name = "Bedrock Test"
base_url = "https://bedrock-runtime.us-east-1.amazonaws.com"
[auth]
api_key_env = "AWS_ACCESS_KEY_ID"
header_name = "Authorization"
header_format = "AWS4-HMAC-SHA256 {key}"
[transport]
streaming = "sse"
chat_path = "/converse"
[tool_calls]
format = "bedrock_tool_use"
[quirks]
aws-region = "us-east-1"
    "#
}

fn parse_manifest() -> ProviderManifest {
    ProviderManifest::from_toml_str(manifest_str()).unwrap()
}

#[test]
fn build_request_uses_bedrock_envelope_shape() {
    let m = parse_manifest();
    let req = ProviderRequest {
        model: "anthropic.claude-3-5-haiku-20241022-v1:0".into(),
        messages: vec![user_message("hello")],
        tools: vec![ToolSchema {
            name: "ls".into(),
            description: "list files".into(),
            parameters: serde_json::json!({"type":"object"}),
        }],
        system: Some("be terse".into()),
        sampling: Sampling {
            temperature: Some(0.2),
            max_tokens: Some(512),
            ..Default::default()
        },
    };
    let body = build_request_body(&m, &req).unwrap();
    assert_eq!(body["modelId"], "anthropic.claude-3-5-haiku-20241022-v1:0");
    assert_eq!(body["system"][0]["text"], "be terse");
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"][0]["text"], "hello");
    assert_eq!(body["toolConfig"]["tools"][0]["toolSpec"]["name"], "ls");
    assert_eq!(body["inferenceConfig"]["maxTokens"], 512);
}

#[test]
fn extractor_pulls_tool_use_from_envelope() {
    let body = serde_json::json!({
        "output": { "message": { "role": "assistant", "content": [
            {"text": "calling..."},
            {"toolUse": {"toolUseId":"tool_1","name":"ls","input":{"path":"/"}}}
        ]}}
    });
    let calls = crate::bedrock_tool_use(&body).unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "ls");
    assert_eq!(calls[0].arguments["path"], "/");
}

#[test]
fn extractor_returns_empty_when_no_tool_use_blocks() {
    let body = serde_json::json!({
        "output": { "message": { "content": [{"text":"hi"}] } }
    });
    let calls = crate::bedrock_tool_use(&body).unwrap();
    assert!(calls.is_empty());
}

#[test]
fn streaming_event_sequence_emits_text_then_finish() {
    let mut core = StreamCore::new();
    // ContentBlockStart for index 0 (text block).
    let start = bt::ContentBlockStartEvent::builder()
        .content_block_index(0)
        .build()
        .unwrap();
    handle_stream_event(
        bt::ConverseStreamOutput::ContentBlockStart(start),
        &mut core,
    );
    // Text delta on index 0.
    let delta = bt::ContentBlockDeltaEvent::builder()
        .content_block_index(0)
        .delta(bt::ContentBlockDelta::Text("Hello".into()))
        .build()
        .unwrap();
    handle_stream_event(
        bt::ConverseStreamOutput::ContentBlockDelta(delta),
        &mut core,
    );
    // MessageStop with EndTurn.
    let stop = bt::MessageStopEvent::builder()
        .stop_reason(bt::StopReason::EndTurn)
        .build()
        .unwrap();
    handle_stream_event(bt::ConverseStreamOutput::MessageStop(stop), &mut core);

    let drained: Vec<_> = core.pending.drain(..).collect();
    assert_eq!(drained.len(), 2);
    match &drained[0] {
        Ok(ProviderResponseDelta::TextChunk(t)) => assert_eq!(t, "Hello"),
        other => panic!("first delta: {other:?}"),
    }
    match &drained[1] {
        Ok(ProviderResponseDelta::FinishReason(r)) => assert_eq!(*r, StopReason::EndTurn),
        other => panic!("second delta: {other:?}"),
    }
    assert!(core.done);
}

#[test]
fn streaming_tool_use_sequence_emits_starter_then_args() {
    let mut core = StreamCore::new();
    let tu_block = bt::ToolUseBlockStart::builder()
        .tool_use_id("tool_1")
        .name("ls")
        .build()
        .unwrap();
    let start = bt::ContentBlockStartEvent::builder()
        .content_block_index(0)
        .start(bt::ContentBlockStart::ToolUse(tu_block))
        .build()
        .unwrap();
    handle_stream_event(
        bt::ConverseStreamOutput::ContentBlockStart(start),
        &mut core,
    );

    let tu_delta = bt::ToolUseBlockDelta::builder()
        .input(r#"{"path":"/"}"#)
        .build()
        .unwrap();
    let delta = bt::ContentBlockDeltaEvent::builder()
        .content_block_index(0)
        .delta(bt::ContentBlockDelta::ToolUse(tu_delta))
        .build()
        .unwrap();
    handle_stream_event(
        bt::ConverseStreamOutput::ContentBlockDelta(delta),
        &mut core,
    );

    assert_eq!(core.pending.len(), 2);
    match &core.pending[0] {
        Ok(ProviderResponseDelta::ToolCallDelta {
            id,
            name,
            args_partial,
        }) => {
            assert_eq!(id, "tool_1");
            assert_eq!(name, "ls");
            assert!(args_partial.is_empty());
        }
        other => panic!("starter delta: {other:?}"),
    }
    match &core.pending[1] {
        Ok(ProviderResponseDelta::ToolCallDelta {
            id, args_partial, ..
        }) => {
            assert_eq!(id, "tool_1");
            assert_eq!(args_partial, r#"{"path":"/"}"#);
        }
        other => panic!("args delta: {other:?}"),
    }
}

#[test]
fn document_to_json_round_trips_basic_shapes() {
    let doc = Document::Object(
        [
            ("a".into(), Document::String("v".into())),
            ("b".into(), Document::Bool(true)),
            (
                "c".into(),
                Document::Array(vec![
                    Document::Number(aws_smithy_types::Number::Float(1.0)),
                    Document::Null,
                ]),
            ),
        ]
        .into_iter()
        .collect(),
    );
    let v = document_to_json(&doc);
    assert_eq!(v["a"], "v");
    assert_eq!(v["b"], true);
    assert_eq!(v["c"][0], 1.0);
    assert!(v["c"][1].is_null());
}
