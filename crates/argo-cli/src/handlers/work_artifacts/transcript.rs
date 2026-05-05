//! Transcript-rendering helpers for `transcript.md`.
//!
//! Extracted from `mod.rs` (#727) so the orchestration file
//! stays under the 500-LOC budget. Two pure functions:
//!
//!   * `render_transcript_md(session, messages)` — full
//!     `# Worker run transcript — session …` markdown header
//!     plus every message rendered in document order.
//!   * `render_blocks_md(content_json)` — render a single
//!     stored `content_json` payload as readable markdown
//!     (typed text blocks pass through; tool_use becomes
//!     `→ tool_use NAME(input)`; tool_result becomes
//!     `✓ tool_result ID: body` or `✗ …` when is_error).
//!
//! Stays out of `extract_text_blocks` — that one's an
//! orchestration helper for multi-step section assembly and
//! lives in mod.rs next to its caller.

use serde_json::Value;

pub(super) fn render_transcript_md(
    session: &argo_store::SessionHeader,
    messages: &[argo_store::MessageRow],
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Worker run transcript — session {}\n\n",
        session.id.0
    ));
    if let Some(v) = &session.vertical_id {
        out.push_str(&format!("- vertical: `{v}`\n"));
    }
    out.push_str(&format!(
        "- provider: `{}/{}`\n",
        session.provider, session.model
    ));
    out.push_str(&format!("- started: {}\n", session.started_at.to_rfc3339()));
    if let Some(end) = session.ended_at {
        out.push_str(&format!("- ended: {}\n", end.to_rfc3339()));
    }
    out.push_str(&format!("- messages: {}\n\n---\n\n", messages.len()));

    for (i, row) in messages.iter().enumerate() {
        out.push_str(&format!("## [{:02}] {}\n\n", i + 1, row.role));
        let body = render_blocks_md(&row.content_json);
        if body.trim().is_empty() {
            out.push_str("_(empty)_\n\n");
        } else {
            out.push_str(&body);
            if !body.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
    }
    out
}

/// Stored content_json may be a typed-block array, a raw string
/// (legacy), or null. Render to readable markdown either way.
fn render_blocks_md(content_json: &str) -> String {
    let parsed: Value = serde_json::from_str(content_json).unwrap_or(Value::Null);
    let arr: Vec<Value> = match parsed {
        Value::Array(a) => a,
        Value::String(s) => return s,
        Value::Null => return String::new(),
        other => vec![other],
    };
    let mut out = String::new();
    for b in arr {
        let kind = b.get("type").and_then(|t| t.as_str()).unwrap_or("?");
        match kind {
            "text" => {
                if let Some(t) = b.get("text").and_then(|t| t.as_str()) {
                    out.push_str(t);
                    out.push('\n');
                }
            }
            "tool_use" => {
                let name = b.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                let input = b
                    .get("input")
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "{}".into());
                out.push_str(&format!("→ tool_use `{name}`({input})\n"));
            }
            "tool_result" => {
                let id = b.get("id").and_then(|i| i.as_str()).unwrap_or("?");
                let body = b
                    .get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| {
                        arr.iter()
                            .find_map(|inner| inner.get("text").and_then(|t| t.as_str()))
                    })
                    .unwrap_or("(no text)");
                let is_error = b.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);
                let mark = if is_error { "✗" } else { "✓" };
                out.push_str(&format!("{mark} tool_result `{id}`: {body}\n"));
            }
            other => {
                out.push_str(&format!("[{other}]\n"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_blocks_md_handles_typed_text_blocks() {
        let out = render_blocks_md(r#"[{"type":"text","text":"hi there"}]"#);
        assert!(out.contains("hi there"));
    }

    #[test]
    fn render_blocks_md_handles_raw_string_legacy() {
        let out = render_blocks_md(r#""legacy raw body""#);
        assert!(out.contains("legacy raw body"));
    }

    #[test]
    fn render_blocks_md_passes_full_text_no_truncation() {
        let long = "x".repeat(5000);
        let json = format!(r#"[{{"type":"text","text":"{long}"}}]"#);
        let out = render_blocks_md(&json);
        assert!(out.len() >= 5000);
    }

    #[test]
    fn render_blocks_md_handles_tool_use_and_tool_result() {
        let json = r#"[
            {"type":"tool_use","name":"web_search","input":{"q":"GTM"}},
            {"type":"tool_result","id":"abc","is_error":false,
             "content":[{"type":"text","text":"ok"}]}
        ]"#;
        let out = render_blocks_md(json);
        assert!(out.contains("→ tool_use `web_search`"));
        assert!(out.contains("✓ tool_result `abc`: ok"));
    }
}
