//! `GET /sessions/<id>/export` and its render helpers. Lifted out
//! of `routes_session_get.rs` so the dispatcher there stays under
//! the 500-LOC budget — and so the JSONL / Markdown rendering for
//! HTTP exports lives in one focused file.
//!
//! The dispatcher in `routes_session_get.rs` re-imports
//! `export_response` and calls it directly; nothing else here is
//! exposed beyond the gateway module.

use super::routes_query::parse_query_kv;
use argo_store::{MessageRow, SessionStore};
use argo_types::SessionId;
use std::sync::Arc;

/// `GET /sessions/<id>/export[?format=jsonl|markdown]`. JSONL is
/// the default — one self-describing JSON object per line, the
/// same shape as `argo session export <id>` writes. Markdown
/// returns a transcript with the session header + one block
/// per message (no audit rows in markdown). Mirrors the CLI
/// flag wording exactly.
pub(super) async fn export_response(
    sid: &str,
    tail: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return (
            "503 Service Unavailable",
            "{\"error\":\"store unavailable\"}".to_string(),
        );
    };
    let format = parse_query_kv(tail, "format")
        .unwrap_or_else(|| "jsonl".to_string());
    let include_audit = parse_query_kv(tail, "include-audit")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    let messages = match store.list_messages(id).await {
        Ok(m) => m,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"list_messages: {e}\"}}"),
            );
        }
    };
    match format.as_str() {
        "jsonl" => {
            let mut body = render_jsonl(&messages);
            if include_audit {
                match argo_audit::rows_for_session(store, id).await {
                    Ok(rows) => {
                        for r in &rows {
                            if let Ok(line) = serde_json::to_string(r) {
                                body.push_str(&line);
                                body.push('\n');
                            }
                        }
                    }
                    Err(e) => {
                        return (
                            "500 Internal Server Error",
                            format!("{{\"error\":\"rows_for_session: {e}\"}}"),
                        );
                    }
                }
            }
            ("200 OK", body)
        }
        "markdown" => ("200 OK", render_markdown_body(sid, &messages)),
        other => (
            "400 Bad Request",
            format!(
                "{{\"error\":\"unknown format '{other}' (expected 'jsonl' or 'markdown')\"}}"
            ),
        ),
    }
}

/// One self-describing JSON object per line — same wire shape as
/// `argo session export` (no audit rows in this HTTP variant).
fn render_jsonl(messages: &[MessageRow]) -> String {
    let mut out = String::new();
    for row in messages {
        let line = serde_json::json!({
            "seq": row.seq,
            "id": hex::encode(&row.id),
            "session_id": hex::encode(&row.session_id),
            "role": row.role,
            "content": serde_json::from_str::<serde_json::Value>(&row.content_json)
                .unwrap_or(serde_json::Value::Null),
            "tool_calls": serde_json::from_str::<serde_json::Value>(&row.tool_calls_json)
                .unwrap_or(serde_json::Value::Array(Vec::new())),
            "tool_call_id": row.tool_call_id.as_ref().map(hex::encode),
            "reasoning": row.reasoning,
            "provider_meta": serde_json::from_str::<serde_json::Value>(&row.provider_meta)
                .unwrap_or(serde_json::Value::Null),
            "created_at_micros": row.created_at,
        });
        if let Ok(s) = serde_json::to_string(&line) {
            out.push_str(&s);
            out.push('\n');
        }
    }
    out
}

/// Markdown transcript with a `# Session <uuid>` header and one
/// `## [seq] role` block per message. Body extraction matches the
/// CLI's `extract_markdown_body` (string / typed-block array /
/// null). No audit rows in this HTTP variant.
fn render_markdown_body(sid: &str, messages: &[MessageRow]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Session {sid}\n\n"));
    for row in messages {
        out.push_str(&format!("## [{}] {}\n\n", row.seq, row.role));
        let body = extract_body(&row.content_json);
        if !body.trim().is_empty() {
            out.push_str(&body);
            out.push_str("\n\n");
        }
    }
    out
}

/// Mirrors `crate::handlers::session_export::extract_markdown_body`
/// — kept inline here so the gateway helper stays self-contained
/// (the CLI handler module isn't `pub`-visible from this crate).
fn extract_body(content_json: &str) -> String {
    let v: serde_json::Value = match serde_json::from_str(content_json) {
        Ok(v) => v,
        Err(_) => return content_json.to_string(),
    };
    match &v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Array(items) => {
            let mut out = String::new();
            for it in items {
                if let Some(s) = it.as_str() {
                    out.push_str(s);
                    out.push('\n');
                    continue;
                }
                let kind = it.get("type").and_then(|t| t.as_str()).unwrap_or("block");
                match kind {
                    "text" => {
                        if let Some(t) = it.get("text").and_then(|t| t.as_str()) {
                            out.push_str(t);
                            out.push('\n');
                        }
                    }
                    "tool_use" => {
                        let name = it.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                        out.push_str(&format!("_(tool_use → {name})_\n"));
                    }
                    "tool_result" => out.push_str("_(tool_result)_\n"),
                    other => out.push_str(&format!("_({other})_\n")),
                }
            }
            out.trim_end().to_string()
        }
        other => other.to_string(),
    }
}
