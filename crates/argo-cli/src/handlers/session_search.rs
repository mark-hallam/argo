//! `argo session search` — FTS5-backed search of the message
//! corpus. Lifted out of `session.rs` so that file stays under
//! the 500-LOC budget. Mirrors the gateway's `GET /search`
//! query surface (q, limit, session, role).
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};

#[allow(clippy::too_many_arguments)]
pub(super) async fn run(
    query: &str,
    limit: u32,
    session: Option<&str>,
    role: Option<&str>,
    json: bool,
    format: &str,
    count_only: bool,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    if let Some(want) = role {
        if !matches!(want, "user" | "assistant" | "tool" | "system") {
            return Err(ArgoError::Provider(format!(
                "unknown --role '{want}' — use user|assistant|tool|system"
            )));
        }
    }
    // --count mode: short-circuit. Mirrors the gateway's
    // GET /search/count: hard cap at 10000.
    if count_only {
        const HARD_CAP: u32 = 10_000;
        let raw = match session {
            Some(raw) => {
                let id = uuid::Uuid::parse_str(raw).map_err(|e| {
                    ArgoError::Provider(format!("invalid session id '{raw}': {e}"))
                })?;
                store
                    .search_messages_in_session(SessionId(id), query, HARD_CAP)
                    .await?
            }
            None => store.search_messages(query, HARD_CAP).await?,
        };
        let count = raw
            .into_iter()
            .filter(|h| match role {
                Some(want) => h.role == want,
                None => true,
            })
            .count();
        let truncated = count as u32 >= HARD_CAP;
        if json {
            let payload = serde_json::json!({
                "count": count,
                "truncated": truncated,
            });
            println!("{}", serde_json::to_string(&payload).unwrap_or_default());
            return Ok(());
        }
        if truncated {
            println!("{count}+");
        } else {
            println!("{count}");
        }
        return Ok(());
    }
    let mut hits = match session {
        Some(raw) => {
            let id = uuid::Uuid::parse_str(raw)
                .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
            store
                .search_messages_in_session(SessionId(id), query, limit)
                .await?
        }
        None => store.search_messages(query, limit).await?,
    };
    if let Some(want) = role {
        hits.retain(|h| h.role == want);
    }
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            for h in &hits {
                let row = serde_json::json!({
                    "session_id": h.session_id.0.to_string(),
                    "seq": h.seq,
                    "role": h.role,
                    "snippet": h.snippet,
                });
                let serialized = serde_json::to_string(&row)
                    .map_err(|e| ArgoError::Provider(format!("serialize hit: {e}")))?;
                println!("{serialized}");
            }
            return Ok(());
        }
        "markdown" => {
            println!(
                "# Search results for `{}` ({} match{})",
                query,
                hits.len(),
                if hits.len() == 1 { "" } else { "es" }
            );
            println!();
            if hits.is_empty() {
                return Ok(());
            }
            println!("| session_id | seq | role | snippet |");
            println!("| --- | ---: | --- | --- |");
            for h in &hits {
                let snippet = h.snippet.replace('|', "\\|");
                println!(
                    "| `{}` | {} | `{}` | {snippet} |",
                    h.session_id.0, h.seq, h.role
                );
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
    if hits.is_empty() {
        println!("(no matches for '{query}')");
        return Ok(());
    }
    println!("{} match(es):", hits.len());
    for h in hits {
        // FTS5 snippet has `[...]` markers around matches; render
        // it as-is so users can spot the highlight in a terminal.
        println!(
            "  {sid}  seq={seq:<5} {role:<10} {snippet}",
            sid = h.session_id.0,
            seq = h.seq,
            role = h.role,
            snippet = h.snippet
        );
    }
    Ok(())
}
