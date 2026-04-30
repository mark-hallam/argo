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
        let snippet = first_text_block(&m.content_json);
        println!("--- [{:>3}] {} ---", m.seq, m.role);
        if !snippet.is_empty() {
            println!("{snippet}");
        } else {
            // Fall back to the raw content_json so the user sees
            // *something* meaningful for tool-use / non-text blocks.
            println!("(non-text content; raw: {})", m.content_json);
        }
        println!();
    }
    Ok(())
}

fn first_text_block(content_json: &str) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(content_json).unwrap_or(serde_json::Value::Null);
    let arr = match parsed.as_array() {
        Some(a) => a,
        None => return String::new(),
    };
    arr.iter()
        .find_map(|b| b.get("text").and_then(|t| t.as_str()))
        .unwrap_or("")
        .to_string()
}
