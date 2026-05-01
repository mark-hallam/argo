//! Helper for the message-detail overlay: turn a session
//! message's `content_json` blob into a readable, multi-line
//! string. tool_use blocks become `→ <name>(<args>)`,
//! tool_result blocks become `← <id> <text>` (with `[error]`
//! when `is_error`), plain text passes through. Multiple blocks
//! are joined by a `---` rule. Lifted out of `overlays.rs` so
//! that file stays under the 500-LOC budget.

/// Render the typed-blocks form of a content_json blob as
/// readable text. Returns `None` when the input isn't a JSON
/// array of blocks (e.g. plain string content).
pub(crate) fn extract_block_text(content_json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(content_json).ok()?;
    let arr = parsed.as_array()?;
    let mut chunks = Vec::with_capacity(arr.len());
    for b in arr {
        let kind = b.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match kind {
            "text" => {
                if let Some(t) = b.get("text").and_then(|v| v.as_str()) {
                    chunks.push(t.to_string());
                }
            }
            "tool_use" => {
                let name = b.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let args = b
                    .get("input")
                    .or_else(|| b.get("arguments"))
                    .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                    .unwrap_or_default();
                chunks.push(format!("→ {name}\n{args}"));
            }
            "tool_result" => {
                let id = b.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let is_error = b.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
                let tag = if is_error { " [error]" } else { "" };
                let text = b
                    .get("content")
                    .and_then(|c| c.as_array())
                    .map(|inner| {
                        inner
                            .iter()
                            .filter_map(|cb| cb.get("text").and_then(|t| t.as_str()))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                chunks.push(format!("← {id}{tag}\n{text}"));
            }
            _ => {}
        }
    }
    if chunks.is_empty() {
        return None;
    }
    Some(chunks.join("\n\n---\n\n"))
}

#[cfg(test)]
mod tests {
    use super::extract_block_text;

    #[test]
    fn extract_unwraps_tool_result_text_blocks() {
        let json = r#"[{"type":"tool_result","id":"abc","is_error":false,
                       "content":[{"type":"text","text":"hello"},
                                  {"type":"text","text":"world"}]}]"#;
        let drill = extract_block_text(json).unwrap();
        assert!(drill.contains("← abc"));
        assert!(drill.contains("hello"));
        assert!(drill.contains("world"));
        assert!(!drill.contains("[error]"));
    }

    #[test]
    fn extract_marks_errors_and_handles_multiple_blocks() {
        let json = r#"[{"type":"text","text":"prelude"},
                      {"type":"tool_use","id":"01","name":"file_read",
                       "input":{"path":"/x"}},
                      {"type":"tool_result","id":"01","is_error":true,
                       "content":[{"type":"text","text":"boom"}]}]"#;
        let drill = extract_block_text(json).unwrap();
        assert!(drill.contains("prelude"));
        assert!(drill.contains("→ file_read"));
        assert!(drill.contains("/x"));
        assert!(drill.contains("← 01 [error]"));
        assert!(drill.contains("boom"));
        assert!(drill.contains("---")); // block separator
    }

    #[test]
    fn extract_returns_none_for_plain_string_content() {
        assert!(extract_block_text("\"plain string\"").is_none());
    }
}
