//! `argo session export` — extracted from `session.rs` to keep
//! that file under the LOC budget. Supports two output formats:
//! `jsonl` (default, the line-delimited JSON shape that scripts
//! and `argo session import` consume) and `markdown` (a
//! human-readable transcript: one heading per message with the
//! body extracted from `content_json`).
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::io::{BufWriter, Write};

/// Top-level entry called from `SessionCmd::Export`. `format` is
/// one of `jsonl` or `markdown`; anything else returns an
/// `ArgoError::Provider`.
pub async fn export_session(
    raw: &str,
    out: Option<&str>,
    include_audit: bool,
    format: &str,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    let messages = store.list_messages(SessionId(id)).await?;
    let mut writer: Box<dyn Write> = match out {
        Some(path) => {
            let f = std::fs::File::create(path)
                .map_err(|e| ArgoError::Provider(format!("create {path}: {e}")))?;
            Box::new(BufWriter::new(f))
        }
        None => Box::new(BufWriter::new(std::io::stdout())),
    };
    let audit_count = match format {
        "jsonl" => {
            export_jsonl(&mut writer, &messages, &store, SessionId(id), include_audit).await?
        }
        "markdown" => {
            let header = MarkdownHeader::load(&store, SessionId(id), &messages).await;
            export_markdown(&mut writer, raw, &header, &messages)?
        }
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'jsonl' or 'markdown')"
            )));
        }
    };
    writer
        .flush()
        .map_err(|e| ArgoError::Provider(format!("flush: {e}")))?;
    if out.is_some() {
        if include_audit && format == "jsonl" {
            eprintln!(
                "session export: wrote {} message(s) + {audit_count} audit row(s)",
                messages.len()
            );
        } else {
            eprintln!("session export: wrote {} messages", messages.len());
        }
    }
    Ok(())
}

/// Write each message as a JSON object on its own line, then
/// (optionally) every audit row in the same shape. Returns the
/// number of audit rows written so the caller can include it in
/// the summary line.
async fn export_jsonl(
    writer: &mut Box<dyn Write>,
    messages: &[argo_store::MessageRow],
    store: &SessionStore,
    session_id: SessionId,
    include_audit: bool,
) -> ArgoResult<usize> {
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
        let serialized = serde_json::to_string(&line)
            .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?;
        writer
            .write_all(serialized.as_bytes())
            .and_then(|()| writer.write_all(b"\n"))
            .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    }
    let mut audit_count = 0_usize;
    if include_audit {
        let rows = argo_audit::rows_for_session(store, session_id).await?;
        audit_count = rows.len();
        for r in &rows {
            let line = serde_json::to_string(r)
                .map_err(|e| ArgoError::Provider(format!("serialize audit: {e}")))?;
            writer
                .write_all(line.as_bytes())
                .and_then(|()| writer.write_all(b"\n"))
                .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
        }
    }
    Ok(audit_count)
}

/// Aggregated metadata rendered into the markdown transcript
/// header. Loaded once per export call and immutable from then on.
struct MarkdownHeader {
    title: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    ended_at: Option<chrono::DateTime<chrono::Utc>>,
    spend_micros: u64,
    message_count: usize,
}

impl MarkdownHeader {
    /// Pull session row + spend total. Errors are swallowed so a
    /// partially-populated header never blocks the transcript —
    /// the worst case is a missing line, never a missing export.
    async fn load(
        store: &SessionStore,
        sid: SessionId,
        messages: &[argo_store::MessageRow],
    ) -> Self {
        let row = store.get_session(sid).await.ok().flatten();
        let spend_micros =
            super::session_aggregates::session_spend_micros(store, sid)
                .await
                .unwrap_or(0);
        Self {
            title: row.as_ref().and_then(|r| r.title.clone()),
            model: row.as_ref().map(|r| r.model.clone()),
            provider: row.as_ref().map(|r| r.provider.clone()),
            started_at: row.as_ref().map(|r| r.started_at),
            ended_at: row.as_ref().and_then(|r| r.ended_at),
            spend_micros,
            message_count: messages.len(),
        }
    }
}

/// Render a session as a markdown transcript: rich `# Session …`
/// header (title, model, provider, started/ended, spend, message
/// count), then one `## [seq] role` section per message with
/// extracted body text. Always returns 0 (audit rows aren't
/// included in the markdown shape).
fn export_markdown(
    writer: &mut Box<dyn Write>,
    session_raw: &str,
    header: &MarkdownHeader,
    messages: &[argo_store::MessageRow],
) -> ArgoResult<usize> {
    let title = header
        .title
        .as_deref()
        .filter(|t| !t.is_empty())
        .unwrap_or("(untitled)");
    writeln!(writer, "# Session {title} (`{session_raw}`)")
        .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    writeln!(writer).map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    if let Some(model) = &header.model {
        writeln!(writer, "- **Model**: `{model}`")
            .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    }
    if let Some(provider) = &header.provider {
        writeln!(writer, "- **Provider**: `{provider}`")
            .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    }
    if let Some(started_at) = header.started_at {
        writeln!(
            writer,
            "- **Started**: {}",
            started_at.format("%Y-%m-%d %H:%M:%S UTC")
        )
        .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    }
    if let Some(ended_at) = header.ended_at {
        writeln!(
            writer,
            "- **Ended**: {}",
            ended_at.format("%Y-%m-%d %H:%M:%S UTC")
        )
        .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    }
    writeln!(
        writer,
        "- **Spend**: ${:.4}",
        header.spend_micros as f64 / 1_000_000.0
    )
    .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    writeln!(writer, "- **Messages**: {}", header.message_count)
        .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    writeln!(writer).map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
    for row in messages {
        writeln!(writer, "## [{}] {}", row.seq, row.role)
            .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
        writeln!(writer).map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
        let body = extract_markdown_body(&row.content_json);
        if !body.trim().is_empty() {
            writeln!(writer, "{body}").map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
            writeln!(writer).map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
        }
        if let Some(reasoning) = &row.reasoning {
            if !reasoning.trim().is_empty() {
                writeln!(writer, "_(reasoning)_ {reasoning}")
                    .map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
                writeln!(writer).map_err(|e| ArgoError::Provider(format!("write: {e}")))?;
            }
        }
    }
    Ok(0)
}

/// Pull a human-readable body out of `content_json`. Argo stores
/// content in a few shapes — a bare string, an array of typed
/// blocks (`{type:"text", text:"..."}` etc.), or `null` for
/// tool-only assistant turns — so we normalise them to a single
/// markdown body. Tool blocks are summarised so the transcript
/// stays scannable.
pub(super) fn extract_markdown_body(content_json: &str) -> String {
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
                let kind = it
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("block");
                match kind {
                    "text" => {
                        if let Some(t) = it.get("text").and_then(|t| t.as_str()) {
                            out.push_str(t);
                            out.push('\n');
                        }
                    }
                    "tool_use" => {
                        let name = it
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("?");
                        out.push_str(&format!("_(tool_use → {name})_\n"));
                    }
                    "tool_result" => {
                        out.push_str("_(tool_result)_\n");
                    }
                    other => {
                        out.push_str(&format!("_({other})_\n"));
                    }
                }
            }
            out.trim_end().to_string()
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::extract_markdown_body;

    #[test]
    fn extract_string_body() {
        assert_eq!(extract_markdown_body("\"hello\""), "hello");
    }

    #[test]
    fn extract_text_block_array() {
        let json = r#"[{"type":"text","text":"hi"}]"#;
        assert_eq!(extract_markdown_body(json), "hi");
    }

    #[test]
    fn extract_tool_use_block() {
        let json = r#"[{"type":"tool_use","name":"bash"}]"#;
        assert_eq!(extract_markdown_body(json), "_(tool_use → bash)_");
    }

    #[test]
    fn extract_null_is_empty() {
        assert_eq!(extract_markdown_body("null"), "");
    }
}
