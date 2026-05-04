//! `argo session transcript <id>` — full session dump (#720).
//!
//! Unlike `argo session show` (which prints 80-char first/last
//! snippets), this command renders every message body in full,
//! plus every tool_use and tool_result block, plus the session
//! header (id, vertical, provider, model, started_at). Designed
//! as the canonical inspection path for `argo work serve` runs
//! after `completed_jobs.assistant_text` was found insufficient
//! (it's body-truncated to ~2000 chars; this command is not).
//!
//! `argo session show` is unchanged — same snippet-only output
//! it always had.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde_json::Value;

pub async fn run(raw: &str, json_out: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let session_id = super::session_resolve::resolve(&store, raw).await?;
    let session = store
        .get_session(session_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {raw}")))?;
    let messages = store.list_messages(session_id).await?;

    if json_out {
        emit_json(&session, &messages);
    } else {
        emit_text(&session, &messages);
    }
    Ok(())
}

fn emit_json(session: &argo_store::SessionHeader, messages: &[argo_store::MessageRow]) {
    let messages_json: Vec<Value> = messages
        .iter()
        .map(|row| {
            serde_json::json!({
                "seq": row.seq,
                "id": hex::encode(&row.id),
                "role": row.role,
                "created_at_micros": row.created_at,
                "blocks": parse_content_blocks(&row.content_json),
            })
        })
        .collect();

    let payload = serde_json::json!({
        "session_id": session.id.0.to_string(),
        "vertical_id": session.vertical_id,
        "provider": session.provider,
        "model": session.model,
        "title": session.title,
        "started_at": session.started_at.to_rfc3339(),
        "ended_at": session.ended_at.map(|d| d.to_rfc3339()),
        "message_count": messages.len(),
        "messages": messages_json,
    });
    println!(
        "{}",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
    );
}

fn emit_text(session: &argo_store::SessionHeader, messages: &[argo_store::MessageRow]) {
    println!("session {}", session.id.0);
    if let Some(v) = &session.vertical_id {
        println!("  vertical: {v}");
    }
    println!("  provider: {} / {}", session.provider, session.model);
    if let Some(t) = &session.title {
        if !t.is_empty() {
            println!("  title:    {t}");
        }
    }
    println!(
        "  started:  {}",
        session.started_at.format("%Y-%m-%d %H:%M:%S UTC")
    );
    if let Some(end) = session.ended_at {
        println!("  ended:    {}", end.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    println!("  messages: {}", messages.len());
    println!();

    for (i, row) in messages.iter().enumerate() {
        let when = chrono::DateTime::from_timestamp_micros(row.created_at)
            .map(|d| d.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "??:??:??".into());
        println!("  [{:02}] {} @ {when}", i + 1, row.role);
        let body = render_blocks_text(&row.content_json);
        for line in body.lines() {
            println!("    {line}");
        }
        if body.is_empty() {
            println!("    (empty)");
        }
        println!();
    }
}

/// Normalise stored `content_json` (which may be a raw string,
/// a typed-block array, or null) into a JSON array of typed
/// blocks. Returns the array as `serde_json::Value::Array`.
fn parse_content_blocks(content_json: &str) -> Value {
    match serde_json::from_str::<Value>(content_json) {
        Ok(Value::Array(arr)) => Value::Array(arr),
        Ok(Value::String(s)) => {
            serde_json::json!([{"type": "text", "text": s}])
        }
        Ok(Value::Null) => serde_json::json!([]),
        Ok(other) => serde_json::json!([other]),
        Err(_) => serde_json::json!([{"type": "text", "text": content_json}]),
    }
}

/// Render content blocks as readable text. text blocks pass through;
/// tool_use → `→ tool_use NAME(input_json)`; tool_result →
/// `✓ tool_result <id>: <text>` (or ✗ if is_error). Unknown block
/// types render as `[type]`.
fn render_blocks_text(content_json: &str) -> String {
    let blocks = parse_content_blocks(content_json);
    let arr = blocks.as_array().cloned().unwrap_or_default();
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
                out.push_str(&format!("→ tool_use {name}({input})\n"));
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
                out.push_str(&format!("{mark} tool_result {id}: {body}\n"));
            }
            other => {
                out.push_str(&format!("[{other}]\n"));
            }
        }
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_handles_array_pass_through() {
        let v = parse_content_blocks(r#"[{"type":"text","text":"hi"}]"#);
        assert!(v.is_array());
        assert_eq!(v[0]["text"], "hi");
    }

    #[test]
    fn parse_wraps_raw_string_as_text_block() {
        let v = parse_content_blocks(r#""raw user prompt""#);
        assert!(v.is_array());
        assert_eq!(v[0]["type"], "text");
        assert_eq!(v[0]["text"], "raw user prompt");
    }

    #[test]
    fn parse_null_is_empty_array() {
        let v = parse_content_blocks("null");
        assert_eq!(v.as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn render_text_block_passes_through_full_text() {
        // No 80-char snippet truncation, no mid-sentence cut.
        let long = "x".repeat(5000);
        let json = format!(r#"[{{"type":"text","text":"{long}"}}]"#);
        let out = render_blocks_text(&json);
        assert_eq!(out.len(), 5000);
    }

    #[test]
    fn render_tool_use_uses_arrow_marker() {
        let out = render_blocks_text(r#"[{"type":"tool_use","name":"ls","input":{"path":"/"}}]"#);
        assert!(out.contains("→ tool_use ls"));
        assert!(out.contains("path"));
    }

    #[test]
    fn render_tool_result_uses_check_marker_and_extracts_text() {
        let json = r#"[{"type":"tool_result","id":"abc","is_error":false,"content":[{"type":"text","text":"/etc /var"}]}]"#;
        let out = render_blocks_text(json);
        assert!(out.contains("✓ tool_result abc: /etc /var"), "got: {out}");
    }
}
