//! Unit tests for the Anthropic CU loop driver. Loaded via
//! `#[cfg(test)] #[path = ...] mod tests;` from `anthropic.rs`
//! to keep that file under the per-file 500-LOC budget.

use super::*;

#[test]
fn parse_response_extracts_click_action() {
    let raw = serde_json::json!({
        "content": [{
            "type": "tool_use",
            "name": "computer",
            "input": { "action": "left_click", "coordinate": [42, 84] }
        }],
        "usage": { "input_tokens": 100, "output_tokens": 50 }
    });
    let resp = parse_response(raw).unwrap();
    match resp {
        CuStepResponse::ToolCall { action, tokens_used } => {
            assert_eq!(action, CuAction::Click { x: 42, y: 84 });
            assert_eq!(tokens_used, 150);
        }
        CuStepResponse::Stop { .. } => panic!("expected ToolCall"),
    }
}

#[test]
fn parse_response_extracts_type_action() {
    let raw = serde_json::json!({
        "content": [{
            "type": "tool_use",
            "name": "computer",
            "input": { "action": "type", "text": "hello world" }
        }],
        "usage": { "input_tokens": 10, "output_tokens": 10 }
    });
    let resp = parse_response(raw).unwrap();
    match resp {
        CuStepResponse::ToolCall {
            action: CuAction::Type { text },
            tokens_used,
        } => {
            assert_eq!(text, "hello world");
            assert_eq!(tokens_used, 20);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
}

#[test]
fn parse_response_extracts_key_chord() {
    let raw = serde_json::json!({
        "content": [{
            "type": "tool_use",
            "name": "computer",
            "input": { "action": "key", "text": "ctrl+a" }
        }],
        "usage": {}
    });
    let resp = parse_response(raw).unwrap();
    match resp {
        CuStepResponse::ToolCall {
            action: CuAction::Key { keys },
            ..
        } => {
            assert_eq!(keys, vec!["ctrl".to_string(), "a".to_string()]);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
}

#[test]
fn parse_response_extracts_stop_success_tool() {
    let raw = serde_json::json!({
        "content": [{
            "type": "tool_use",
            "name": "computer",
            "input": { "action": "stop_success", "text": "listing posted at $X" }
        }]
    });
    let resp = parse_response(raw).unwrap();
    match resp {
        CuStepResponse::Stop {
            reason,
            text,
            ..
        } => {
            assert_eq!(reason, StopReason::Success);
            assert_eq!(text.as_deref(), Some("listing posted at $X"));
        }
        other => panic!("unexpected variant: {other:?}"),
    }
}

#[test]
fn parse_response_extracts_stop_failure_text_fallback() {
    let raw = serde_json::json!({
        "content": [{
            "type": "text",
            "text": "I cannot proceed. stop_failure"
        }]
    });
    let resp = parse_response(raw).unwrap();
    match resp {
        CuStepResponse::Stop {
            reason: StopReason::Failure,
            ..
        } => {}
        other => panic!("unexpected variant: {other:?}"),
    }
}

#[test]
fn parse_response_errors_on_unknown_tool_name() {
    let raw = serde_json::json!({
        "content": [{
            "type": "tool_use",
            "name": "computer",
            "input": { "action": "make_coffee" }
        }]
    });
    let err = parse_response(raw).expect_err("unknown action errors");
    assert!(err.to_string().contains("unknown action"));
}

#[test]
fn parse_response_errors_on_missing_content() {
    let raw = serde_json::json!({});
    let err = parse_response(raw).expect_err("no content errors");
    assert!(err.to_string().contains("missing"));
}

#[test]
fn parse_response_errors_when_neither_tool_nor_stop_text() {
    let raw = serde_json::json!({
        "content": [{ "type": "text", "text": "still thinking, no signal" }]
    });
    let err = parse_response(raw).expect_err("must surface error");
    assert!(err.to_string().contains("recognised"));
}

#[test]
fn encode_b64_round_trips_a_known_vector() {
    // RFC 4648: "Man" → "TWFu"; "Ma" → "TWE="; "M" → "TQ=="
    assert_eq!(encode_b64(b"Man"), "TWFu");
    assert_eq!(encode_b64(b"Ma"), "TWE=");
    assert_eq!(encode_b64(b"M"), "TQ==");
    assert_eq!(encode_b64(b""), "");
}
