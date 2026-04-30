//! Pure formatter for `RunEvent` envelopes emitted by
//! [`argo_agent_runner::run_turn_with_events`]. Lives in its own
//! file so the (relatively chunky) match arms + their unit tests
//! don't push `app.rs` over the 500-LOC budget.
//!
//! Each envelope has the shape `{"event": kind, "payload": {…}}`
//! and maps 1:1 to an `AuditKind` row, so a future "tail this
//! session" view can reuse the same formatter against
//! `audit_record` rows.

/// Render one `{"event": kind, "payload": {…}}` envelope from the
/// runner's mpsc channel as a single-line trail entry. Pure so it
/// can be unit-tested without spinning up the runner.
#[must_use]
pub fn format_live_event(env: &serde_json::Value) -> String {
    let kind = env.get("event").and_then(|v| v.as_str()).unwrap_or("?");
    let payload = env.get("payload");
    match kind {
        "TurnStart" => {
            let provider = payload
                .and_then(|p| p.get("provider"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let model = payload
                .and_then(|p| p.get("model"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("● TurnStart  provider={provider} model={model}")
        }
        "AssistantMessage" => {
            let round = payload
                .and_then(|p| p.get("round"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let tool_calls = payload
                .and_then(|p| p.get("tool_calls"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let tin = payload
                .and_then(|p| p.get("tokens_in"))
                .and_then(serde_json::Value::as_u64);
            let tout = payload
                .and_then(|p| p.get("tokens_out"))
                .and_then(serde_json::Value::as_u64);
            let tokens = match (tin, tout) {
                (Some(i), Some(o)) => format!(" {i}+{o}t"),
                _ => String::new(),
            };
            format!("✎ Assistant  round={round} tool_calls={tool_calls}{tokens}")
        }
        "ToolCall" => {
            let tool = payload
                .and_then(|p| p.get("tool"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("→ ToolCall   {tool}")
        }
        "ToolResult" => {
            let tool = payload
                .and_then(|p| p.get("tool"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let dur = payload
                .and_then(|p| p.get("duration_ms"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let err = payload
                .and_then(|p| p.get("is_error"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let tag = if err { " [error]" } else { "" };
            format!("← ToolResult {tool} {dur}ms{tag}")
        }
        "TurnEnd" => "✓ TurnEnd".to_string(),
        "Halt" => "✗ Halt".to_string(),
        other => format!("· {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_known_kinds() {
        let turn_start = serde_json::json!({
            "event": "TurnStart",
            "payload": {"provider": "anthropic", "model": "claude-x"}
        });
        let line = format_live_event(&turn_start);
        assert!(line.contains("TurnStart"));
        assert!(line.contains("anthropic"));
        assert!(line.contains("claude-x"));

        let tool_call = serde_json::json!({
            "event": "ToolCall",
            "payload": {"tool": "file_read"}
        });
        assert!(format_live_event(&tool_call).contains("file_read"));

        let tool_result = serde_json::json!({
            "event": "ToolResult",
            "payload": {"tool": "file_read", "duration_ms": 42, "is_error": true}
        });
        let line = format_live_event(&tool_result);
        assert!(line.contains("file_read"));
        assert!(line.contains("42ms"));
        assert!(line.contains("[error]"));

        let assistant = serde_json::json!({
            "event": "AssistantMessage",
            "payload": {"round": 0, "tool_calls": 1, "tokens_in": 12, "tokens_out": 3}
        });
        let line = format_live_event(&assistant);
        assert!(line.contains("Assistant"));
        assert!(line.contains("12+3t"));
    }

    #[test]
    fn unknown_kind_falls_back_to_raw_label() {
        let other = serde_json::json!({"event": "Mystery", "payload": {}});
        assert!(format_live_event(&other).contains("Mystery"));
    }

    #[test]
    fn missing_payload_does_not_panic() {
        let bare = serde_json::json!({"event": "TurnEnd"});
        assert_eq!(format_live_event(&bare), "✓ TurnEnd");
    }
}
