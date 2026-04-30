//! `argo resume <id>` — re-render a recorded session: prints
//! the header (provider, model, title, started_at, ended_at,
//! message_count) and every message in seq order.
//!
//! Read-only. The live "continue against an LLM" flow lives with
//! the daemon's `/sessions/<id>/runs` binding.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};

pub async fn run(raw: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;

    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    let session = store
        .get_session(SessionId(id))
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {raw}")))?;
    let messages = store.list_messages(SessionId(id)).await?;

    println!("=== session {} ===", session.id.0);
    println!("  provider:    {}", session.provider);
    println!("  model:       {}", session.model);
    println!(
        "  title:       {}",
        session.title.as_deref().unwrap_or("(none)")
    );
    println!("  started_at:  {}", session.started_at);
    println!(
        "  ended_at:    {}",
        session
            .ended_at
            .map(|d| d.to_string())
            .unwrap_or_else(|| "(open)".into())
    );
    println!("  messages:    {}", messages.len());
    println!();

    for m in &messages {
        let body = render_content(&m.content_json);
        println!("--- [{:>3}] {} ---", m.seq, m.role);
        if body.is_empty() {
            println!("(empty content; raw: {})", m.content_json);
        } else {
            println!("{body}");
        }
        println!();
    }
    Ok(())
}

/// Render every content block in a message's `content_json` into
/// a human-readable transcript:
/// - `text` → printed as-is
/// - `tool_use` → `→ <tool>(<args>)`
/// - `tool_result` → `← <tool_call_id> [error?]: <text>`
///
/// Blocks Argo doesn't display verbatim (image / audio / document)
/// fall back to a one-line stub.
fn render_content(content_json: &str) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(content_json).unwrap_or(serde_json::Value::Null);
    let arr = match parsed.as_array() {
        Some(a) => a,
        None => return String::new(),
    };
    let mut out: Vec<String> = Vec::new();
    for b in arr {
        let kind = b.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match kind {
            "text" => {
                if let Some(t) = b.get("text").and_then(|v| v.as_str()) {
                    out.push(t.to_string());
                }
            }
            "tool_use" => {
                let name = b.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let args = b
                    .get("arguments")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "{}".into());
                out.push(format!("→ {name}({args})"));
            }
            "tool_result" => {
                let id = b.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let is_error = b.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
                let body = b
                    .get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.iter().find_map(|cb| cb.get("text").and_then(|t| t.as_str())))
                    .unwrap_or("");
                let tag = if is_error { " [error]" } else { "" };
                out.push(format!("← {id}{tag}: {body}"));
            }
            "image" | "audio" | "document" => {
                let url = b.get("url").and_then(|v| v.as_str()).unwrap_or("?");
                out.push(format!("[{kind}: {url}]"));
            }
            _ => {}
        }
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_text_tool_use_and_tool_result_in_order() {
        let json = serde_json::json!([
            {"type": "text", "text": "thinking..."},
            {"type": "tool_use", "id": "01", "name": "ls", "arguments": {"path": "/"}},
            {"type": "tool_result", "id": "abc", "is_error": false,
             "content": [{"type": "text", "text": "ok"}]}
        ])
        .to_string();
        let out = render_content(&json);
        assert!(out.contains("thinking..."));
        assert!(out.contains("→ ls("));
        assert!(out.contains("← abc"));
        assert!(out.contains(": ok"));
    }

    #[test]
    fn empty_array_renders_empty_string() {
        assert_eq!(render_content("[]"), "");
    }
}
