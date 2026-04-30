//! `argo session watch <id>` — `tail -f`-style live watch of a
//! recorded session's messages. Polls `list_messages` every
//! 500ms and prints each newly-appended row as a one-liner.
//!
//! Useful for observing a session that's being driven by another
//! process (the gateway, the TUI, or an `argo chat -i` REPL in a
//! sibling terminal) without having to refresh `argo session
//! show <id>` by hand.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::time::Duration;

pub async fn run(raw: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = std::sync::Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let session_id = super::session_resolve::resolve(&store, raw).await?;
    let id = session_id.0;
    if store.get_session(session_id).await?.is_none() {
        return Err(ArgoError::Provider(format!("no session {raw}")));
    }
    eprintln!("watching session {id} (Ctrl-C to exit)…");

    // Print the existing tail (last 10) before entering the loop
    // so the user sees recent context, then track from there.
    let existing = store.list_messages(session_id).await?;
    let recent_start = existing.len().saturating_sub(10);
    for m in &existing[recent_start..] {
        print_row(m);
    }
    let mut last_seq = existing.last().map_or(0, |m| m.seq);

    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let rows = store.list_messages(session_id).await?;
        for m in rows.iter().filter(|m| m.seq > last_seq) {
            print_row(m);
        }
        if let Some(top) = rows.last() {
            last_seq = last_seq.max(top.seq);
        }
    }
}

fn print_row(m: &argo_store::MessageRow) {
    let summary = first_line_of_content(&m.content_json);
    println!("[{:>3}] {:<10} {summary}", m.seq, m.role);
}

fn first_line_of_content(content_json: &str) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(content_json).unwrap_or(serde_json::Value::Null);
    if let Some(arr) = parsed.as_array() {
        for b in arr {
            if let Some(t) = b.get("text").and_then(|v| v.as_str()) {
                return t.lines().next().unwrap_or("").chars().take(120).collect();
            }
            if let Some(name) = b.get("name").and_then(|v| v.as_str()) {
                return format!("→ {name}");
            }
        }
    }
    content_json.chars().take(120).collect()
}
