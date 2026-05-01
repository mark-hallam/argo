//! `argo session messages <id>` — list the messages on one
//! session, mirroring the gateway's
//! `GET /sessions/<id>/messages?limit&role&since-minutes&seq-min&seq-max&content-contains`
//! query surface. Pure read — no mutation.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};

use super::session_export::extract_markdown_body;

#[allow(clippy::too_many_arguments)]
pub(super) async fn run(
    raw: &str,
    json: bool,
    format: &str,
    limit: Option<u32>,
    tail: Option<u32>,
    role: Option<&str>,
    since_minutes: Option<i64>,
    seq_min: Option<i64>,
    seq_max: Option<i64>,
    content_contains: Option<&str>,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    let mut messages = store.list_messages(SessionId(id)).await?;

    // Apply filters in a single pass — same semantics as the
    // gateway helper.
    let cutoff_micros = since_minutes.map(|m| {
        (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros()
    });
    let needle = content_contains.map(str::to_lowercase);
    messages.retain(|m| {
        if let Some(want) = role {
            if m.role != want {
                return false;
            }
        }
        if let Some(c) = cutoff_micros {
            if m.created_at < c {
                return false;
            }
        }
        if let Some(lo) = seq_min {
            if m.seq < lo {
                return false;
            }
        }
        if let Some(hi) = seq_max {
            if m.seq > hi {
                return false;
            }
        }
        if let Some(n) = needle.as_deref() {
            if !m.content_json.to_lowercase().contains(n) {
                return false;
            }
        }
        true
    });
    if let Some(n) = limit {
        messages.truncate(n as usize);
    } else if let Some(n) = tail {
        let n = n as usize;
        if messages.len() > n {
            let drop = messages.len() - n;
            messages.drain(..drop);
        }
    }

    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            for m in &messages {
                let row = serde_json::json!({
                    "seq": m.seq,
                    "id": hex::encode(&m.id),
                    "role": m.role,
                    "content": serde_json::from_str::<serde_json::Value>(&m.content_json)
                        .unwrap_or(serde_json::Value::Null),
                    "created_at_micros": m.created_at,
                });
                let serialized = serde_json::to_string(&row)
                    .map_err(|e| ArgoError::Provider(format!("serialize message: {e}")))?;
                println!("{serialized}");
            }
            return Ok(());
        }
        "markdown" => {
            for m in &messages {
                println!("## [{}] {}", m.seq, m.role);
                println!();
                let body = extract_markdown_body(&m.content_json);
                if !body.trim().is_empty() {
                    println!("{body}");
                    println!();
                }
            }
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    if messages.is_empty() {
        println!("(no messages match)");
        return Ok(());
    }
    for m in &messages {
        let snippet: String = serde_json::from_str::<serde_json::Value>(&m.content_json)
            .ok()
            .and_then(|v| {
                v.as_array()
                    .and_then(|arr| {
                        arr.iter()
                            .find_map(|b| b.get("text").and_then(|t| t.as_str()).map(str::to_string))
                    })
                    .or_else(|| v.as_str().map(str::to_string))
            })
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(80)
            .collect();
        println!("[{:>4}] {:<10} {snippet}", m.seq, m.role);
    }
    Ok(())
}
