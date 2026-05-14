//! `argo session show` impl. Lifted out of `session.rs` so that
//! file stays under the 500-LOC budget as new fields land on the
//! header (e.g. #689 added `vertical_id` to the text + markdown
//! variants). Pure side-effects: prints to stdout, returns Ok(())
//! on success or an `ArgoError::Provider` for serialise / unknown-
//! format paths.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};

use super::session_aggregates::{count_tool_use_blocks, session_spend_micros};

pub(super) async fn show_session(raw: &str, json: bool, format: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let session_id = super::session_resolve::resolve(&store, raw).await?;
    let id = session_id.0;
    let session = store
        .get_session(session_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {raw}")))?;
    let messages = store.list_messages(session_id).await?;
    let count = messages.len() as u64;
    let first = messages.first().map(message_summary);
    let last = if messages.len() > 1 {
        messages.last().map(message_summary)
    } else {
        None
    };
    let tool_calls = count_tool_use_blocks(&messages);
    let spend_micros = session_spend_micros(&store, SessionId(id)).await.unwrap_or(0);
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            let row = serde_json::json!({
                "id": session.id.0.to_string(),
                "tenant_id": session.tenant_id.0.to_string(),
                "workspace_id": session.workspace_id.0.to_string(),
                "provider": session.provider,
                "model": session.model,
                "title": session.title,
                "vertical_id": session.vertical_id,
                "worker_id": session.worker_id.map(|w| w.to_string()),
                "started_at": session.started_at.to_rfc3339(),
                "ended_at": session.ended_at.map(|d| d.to_rfc3339()),
                "message_count": count,
                "tool_call_count": tool_calls,
                "spend_usd_micros": spend_micros,
                "first_message": first.as_ref().map(|(role, snippet)|
                    serde_json::json!({"role": role, "snippet": snippet})
                ),
                "last_message": last.as_ref().map(|(role, snippet)|
                    serde_json::json!({"role": role, "snippet": snippet})
                ),
            });
            let serialized = serde_json::to_string(&row)
                .map_err(|e| ArgoError::Provider(format!("serialize session: {e}")))?;
            println!("{serialized}");
            return Ok(());
        }
        "markdown" => {
            let title = session
                .title
                .as_deref()
                .filter(|t| !t.is_empty())
                .unwrap_or("(untitled)");
            println!("# Session {title} (`{}`)", session.id.0);
            println!();
            println!("- **Provider**: `{}`", session.provider);
            println!("- **Model**: `{}`", session.model);
            // #689: surface vertical binding in the header (only
            // when bound; legacy sessions stay clean).
            if let Some(v) = session.vertical_id.as_deref() {
                println!("- **Vertical**: `{v}`");
            }
            if let Some(worker) = session.worker_id {
                println!("- **Worker**: `{worker}`");
            }
            println!("- **Started**: {}", session.started_at.to_rfc3339());
            if let Some(ended) = session.ended_at {
                println!("- **Ended**: {}", ended.to_rfc3339());
            }
            println!("- **Messages**: {count}");
            println!("- **Tool calls**: {tool_calls}");
            println!(
                "- **Spend**: ${:.4}",
                spend_micros as f64 / 1_000_000.0
            );
            if let Some((role, snippet)) = &first {
                println!();
                println!("**First** ({role}): {snippet}");
            }
            if let Some((role, snippet)) = &last {
                println!("**Last** ({role}): {snippet}");
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
    println!("session {}", session.id.0);
    println!("  tenant_id:    {}", session.tenant_id.0);
    println!("  workspace_id: {}", session.workspace_id.0);
    println!("  provider:     {}", session.provider);
    println!("  model:        {}", session.model);
    // #689: text format shows the vertical binding as well, only
    // when bound — keeps legacy session output unchanged.
    if let Some(v) = session.vertical_id.as_deref() {
        println!("  vertical:     {v}");
    }
    if let Some(worker) = session.worker_id {
        println!("  worker_id:    {worker}");
    }
    println!(
        "  title:        {}",
        session.title.as_deref().unwrap_or("(none)")
    );
    println!("  started_at:   {}", session.started_at);
    println!(
        "  ended_at:     {}",
        session
            .ended_at
            .map(|d| d.to_string())
            .unwrap_or_else(|| "(open)".into())
    );
    println!("  messages:     {count}");
    println!("  tool calls:   {tool_calls}");
    println!(
        "  spend:        ${:.4}",
        spend_micros as f64 / 1_000_000.0
    );
    if let Some((role, snippet)) = &first {
        println!("  first ({role}): {snippet}");
    }
    if let Some((role, snippet)) = &last {
        println!("  last  ({role}): {snippet}");
    }
    Ok(())
}

fn message_summary(row: &argo_store::MessageRow) -> (String, String) {
    let parsed: serde_json::Value =
        serde_json::from_str(&row.content_json).unwrap_or(serde_json::Value::Null);
    let snippet = parsed
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find_map(|b| b.get("text").and_then(|t| t.as_str()))
        })
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(80)
        .collect::<String>();
    (row.role.clone(), snippet)
}
