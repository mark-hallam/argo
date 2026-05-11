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
    let resps = parse_response(raw).unwrap();
    assert_eq!(resps.len(), 1);
    match &resps[0] {
        CuStepResponse::ToolCall { action, tokens_used } => {
            assert_eq!(*action, CuAction::Click { x: 42, y: 84 });
            assert_eq!(*tokens_used, 150);
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
    let resps = parse_response(raw).unwrap();
    assert_eq!(resps.len(), 1);
    match &resps[0] {
        CuStepResponse::ToolCall {
            action: CuAction::Type { text },
            tokens_used,
        } => {
            assert_eq!(text, "hello world");
            assert_eq!(*tokens_used, 20);
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
    let resps = parse_response(raw).unwrap();
    assert_eq!(resps.len(), 1);
    match &resps[0] {
        CuStepResponse::ToolCall {
            action: CuAction::Key { keys },
            ..
        } => {
            assert_eq!(*keys, vec!["ctrl".to_string(), "a".to_string()]);
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
    let resps = parse_response(raw).unwrap();
    assert_eq!(resps.len(), 1);
    match &resps[0] {
        CuStepResponse::Stop {
            reason,
            text,
            ..
        } => {
            assert_eq!(*reason, StopReason::Success);
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
    let resps = parse_response(raw).unwrap();
    assert_eq!(resps.len(), 1);
    match &resps[0] {
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

/// Mission `mission-cu-parser-multi-action`: a turn carrying two
/// `tool_use` blocks (mouse_move then left_click) yields a Vec of
/// length 2 in document order. Today's behaviour silently dropped
/// the second block and cost an extra round-trip.
#[test]
fn parse_response_collects_two_tool_uses_in_order() {
    let raw = serde_json::json!({
        "content": [
            {
                "type": "tool_use",
                "name": "computer",
                "input": { "action": "mouse_move", "coordinate": [10, 20] }
            },
            {
                "type": "tool_use",
                "name": "computer",
                "input": { "action": "left_click", "coordinate": [10, 20] }
            }
        ],
        "usage": { "input_tokens": 100, "output_tokens": 50 }
    });
    let resps = parse_response(raw).unwrap();
    assert_eq!(resps.len(), 2);
    // First entry carries the turn's full token charge so the
    // budget tracker doesn't double-count multi-action turns.
    match &resps[0] {
        CuStepResponse::ToolCall {
            action: CuAction::MouseMove { x, y },
            tokens_used,
        } => {
            assert_eq!(*x, 10);
            assert_eq!(*y, 20);
            assert_eq!(*tokens_used, 150);
        }
        other => panic!("expected MouseMove first; got {other:?}"),
    }
    match &resps[1] {
        CuStepResponse::ToolCall {
            action: CuAction::Click { x, y },
            tokens_used,
        } => {
            assert_eq!(*x, 10);
            assert_eq!(*y, 20);
            assert_eq!(*tokens_used, 0);
        }
        other => panic!("expected Click second; got {other:?}"),
    }
}

/// Three `tool_use` blocks in one turn yield a Vec of length 3,
/// preserving document order. Confirms the parser doesn't cap at
/// "first or two" via accidental short-circuit.
#[test]
fn parse_response_collects_three_tool_uses_in_order() {
    let raw = serde_json::json!({
        "content": [
            {
                "type": "tool_use",
                "name": "computer",
                "input": { "action": "mouse_move", "coordinate": [1, 1] }
            },
            {
                "type": "tool_use",
                "name": "computer",
                "input": { "action": "left_click", "coordinate": [1, 1] }
            },
            {
                "type": "tool_use",
                "name": "computer",
                "input": { "action": "type", "text": "x" }
            }
        ],
        "usage": { "input_tokens": 5, "output_tokens": 5 }
    });
    let resps = parse_response(raw).unwrap();
    assert_eq!(resps.len(), 3);
    assert!(matches!(
        resps[0],
        CuStepResponse::ToolCall {
            action: CuAction::MouseMove { .. },
            ..
        }
    ));
    assert!(matches!(
        resps[1],
        CuStepResponse::ToolCall {
            action: CuAction::Click { .. },
            ..
        }
    ));
    assert!(matches!(
        resps[2],
        CuStepResponse::ToolCall {
            action: CuAction::Type { .. },
            ..
        }
    ));
}

/// "Act then stop" semantics: a `tool_use` block followed by a
/// `stop_success` tool_use yields a 2-entry Vec — the action
/// first, the Stop second. The executor dispatches the action,
/// then surfaces the stop.
#[test]
fn parse_response_act_then_stop_returns_two_entries_with_stop_last() {
    let raw = serde_json::json!({
        "content": [
            {
                "type": "tool_use",
                "name": "computer",
                "input": { "action": "left_click", "coordinate": [50, 60] }
            },
            {
                "type": "tool_use",
                "name": "computer",
                "input": { "action": "stop_success", "text": "all done" }
            }
        ],
        "usage": { "input_tokens": 30, "output_tokens": 20 }
    });
    let resps = parse_response(raw).unwrap();
    assert_eq!(resps.len(), 2);
    match &resps[0] {
        CuStepResponse::ToolCall {
            action: CuAction::Click { x, y },
            tokens_used,
        } => {
            assert_eq!(*x, 50);
            assert_eq!(*y, 60);
            assert_eq!(*tokens_used, 50);
        }
        other => panic!("expected Click first; got {other:?}"),
    }
    match &resps[1] {
        CuStepResponse::Stop {
            reason: StopReason::Success,
            text,
            tokens_used,
        } => {
            assert_eq!(text.as_deref(), Some("all done"));
            assert_eq!(*tokens_used, 0);
        }
        other => panic!("expected Stop second; got {other:?}"),
    }
}

/// Stop-only turn (no tool_use, just a stop-signal text block):
/// returns a single-element Vec carrying the stop response.
/// Preserves today's behaviour for the canonical terminal turn.
#[test]
fn parse_response_text_stop_only_yields_one_element_vec() {
    let raw = serde_json::json!({
        "content": [{
            "type": "text",
            "text": "Wrapped up. stop_success"
        }],
        "usage": { "input_tokens": 5, "output_tokens": 5 }
    });
    let resps = parse_response(raw).unwrap();
    assert_eq!(resps.len(), 1);
    match &resps[0] {
        CuStepResponse::Stop {
            reason: StopReason::Success,
            text,
            tokens_used,
        } => {
            assert!(text.as_deref().unwrap_or("").contains("stop_success"));
            assert_eq!(*tokens_used, 10);
        }
        other => panic!("expected Stop; got {other:?}"),
    }
}

#[test]
fn build_messages_body_uses_configured_tool_variant_and_beta_header() {
    let req = CuStepRequest {
        system_prompt: "do the thing",
        screenshot_png: b"PNGBYTES",
        prior_actions: &[],
        screen_width: 1024,
        screen_height: 768,
    };
    let body = build_messages_body(req, "claude-opus-4-7", "computer_20251124");
    assert_eq!(body["tools"][0]["type"], "computer_20251124");
    assert_eq!(body["tools"][0]["name"], "computer");
    assert_eq!(body["model"], "claude-opus-4-7");
    assert_eq!(beta_header_for("computer_20251124"), "computer-use-2025-11-24");

    // Override path: a caller pinning the older 2025-01-24
    // variant must round-trip through the request body and the
    // matching beta header.
    let req2 = CuStepRequest {
        system_prompt: "x",
        screenshot_png: b"",
        prior_actions: &[],
        screen_width: 800,
        screen_height: 600,
    };
    let body2 = build_messages_body(req2, "claude-sonnet-4-5", "computer_20250124");
    assert_eq!(body2["tools"][0]["type"], "computer_20250124");
    assert_eq!(beta_header_for("computer_20250124"), "computer-use-2025-01-24");
    // Unknown variants fall back to the current default header
    // so a future config-only bump still produces a valid call.
    assert_eq!(beta_header_for("computer_99999999"), "computer-use-2025-11-24");
}

#[test]
fn encode_b64_round_trips_a_known_vector() {
    // RFC 4648: "Man" → "TWFu"; "Ma" → "TWE="; "M" → "TQ=="
    assert_eq!(encode_b64(b"Man"), "TWFu");
    assert_eq!(encode_b64(b"Ma"), "TWE=");
    assert_eq!(encode_b64(b"M"), "TQ==");
    assert_eq!(encode_b64(b""), "");
}
