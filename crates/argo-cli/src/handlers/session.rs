//! `argo session` — list / diff / bisect / replay (offline ops
//! only; the live `replay` against a different model lives with
//! the daemon binding).
use crate::cli::SessionCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::io::{BufWriter, Write};

pub async fn run(cmd: SessionCmd) -> ArgoResult<()> {
    match cmd {
        SessionCmd::List {
            limit,
            json,
            since_minutes,
            older_than_minutes,
            spent_min,
            spent_max,
            provider,
            model,
            status,
            title_contains,
        } => {
            list_sessions(
                limit,
                json,
                since_minutes,
                older_than_minutes,
                spent_min,
                spent_max,
                provider,
                model,
                status,
                title_contains,
            )
            .await
        }
        SessionCmd::Show { id, json } => show_session(&id, json).await,
        SessionCmd::Count {
            status,
            since_minutes,
            older_than_minutes,
            provider,
            model,
            json,
        } => {
            count_sessions(
                status,
                since_minutes,
                older_than_minutes,
                provider,
                model,
                json,
            )
            .await
        }
        SessionCmd::Watch { id } => super::session_watch::run(&id).await,
        SessionCmd::AttachSkill { id, skill } => {
            super::session_skills::attach(&id, &skill).await
        }
        SessionCmd::DetachSkill { id, skill } => {
            super::session_skills::detach(&id, &skill).await
        }
        SessionCmd::Skills { id } => super::session_skills::list(&id).await,
        SessionCmd::Export {
            id,
            out,
            include_audit,
        } => export_session(&id, out.as_deref(), include_audit).await,
        SessionCmd::Search {
            query,
            limit,
            session,
            role,
        } => search(&query, limit, session.as_deref(), role.as_deref()).await,
        SessionCmd::Delete { id, force } => delete_session(&id, force).await,
        SessionCmd::Diff { id1, id2, json } => diff_sessions(&id1, &id2, json).await,
        SessionCmd::Bisect { id, hypothesis } => {
            super::session_bisect::run(&id, &hypothesis).await
        }
        SessionCmd::End { id } => end_session(&id).await,
        SessionCmd::EndAll { older_than_minutes } => end_all(older_than_minutes).await,
        SessionCmd::Reopen { id } => reopen_session(&id).await,
        SessionCmd::Replay {
            id,
            model,
            provider,
            tools: _,
            workspace: _,
        } => super::session_replay::run(&id, model.as_deref(), provider.as_deref()).await,
    }
}

use super::session_diff::diff_sessions;

/// `argo session reopen <id>` — clears `ended_at` back to NULL.
/// Inverse of `argo session end`. Idempotent.
async fn reopen_session(raw: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    let n = store.reopen_session(SessionId(id)).await?;
    if n == 0 {
        println!("(session {raw} not found)");
    } else {
        println!("reopened {raw}");
    }
    Ok(())
}

/// `argo session end <id>` — stamps `ended_at` on the session
/// row. Idempotent; ending an already-ended session is a no-op.
async fn end_session(raw: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    let n = store.end_session(SessionId(id)).await?;
    if n == 0 {
        println!("(session {raw} already ended or not found)");
    } else {
        println!("ended {raw}");
    }
    Ok(())
}

/// `argo session end-all [--older-than-minutes N]` — bulk-end
/// open sessions. Without the filter, every open session
/// (ended_at IS NULL) is ended; with the filter, only sessions
/// whose started_at is older than N minutes ago. Mirrors the
/// gateway's POST /sessions/end-all (#438).
async fn end_all(older_than_minutes: Option<i64>) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let rows = store.list_all_sessions(10_000).await?;
    let cutoff = older_than_minutes
        .map(|m| chrono::Utc::now() - chrono::Duration::minutes(m));
    let mut ended: u64 = 0;
    for s in rows
        .into_iter()
        .filter(|s| s.ended_at.is_none())
        .filter(|s| match cutoff {
            Some(c) => s.started_at < c,
            None => true,
        })
    {
        if let Ok(n) = store.end_session(SessionId(s.id.0)).await {
            ended += n;
        }
    }
    println!("ended {ended} session(s)");
    Ok(())
}

async fn delete_session(raw: &str, force: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    if !force {
        // Show what we're about to delete; refuse without --force.
        let header = store
            .get_session(SessionId(id))
            .await?
            .ok_or_else(|| ArgoError::Provider(format!("no session {raw} (already deleted?)")))?;
        return Err(ArgoError::Provider(format!(
            "would delete session '{}' ({}/{}) — re-run with --force",
            header.title.as_deref().unwrap_or("(untitled)"),
            header.provider,
            header.model
        )));
    }
    let n = store.delete_session(SessionId(id)).await?;
    if n == 0 {
        println!("(no session {raw} — nothing deleted)");
    } else {
        println!("deleted {raw} ({n} session row + cascaded children)");
    }
    Ok(())
}

async fn search(
    query: &str,
    limit: u32,
    session: Option<&str>,
    role: Option<&str>,
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

async fn export_session(
    raw: &str,
    out: Option<&str>,
    include_audit: bool,
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
    for row in &messages {
        // Re-shape into a stable, self-describing JSON object so
        // external tools don't need to know the SQLite schema.
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
        let rows = argo_audit::rows_for_session(&store, SessionId(id)).await?;
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
    writer
        .flush()
        .map_err(|e| ArgoError::Provider(format!("flush: {e}")))?;
    if out.is_some() {
        if include_audit {
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

async fn show_session(raw: &str, json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
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
    if json {
        let row = serde_json::json!({
            "id": session.id.0.to_string(),
            "tenant_id": session.tenant_id.0.to_string(),
            "workspace_id": session.workspace_id.0.to_string(),
            "provider": session.provider,
            "model": session.model,
            "title": session.title,
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
    println!("session {}", session.id.0);
    println!("  tenant_id:    {}", session.tenant_id.0);
    println!("  workspace_id: {}", session.workspace_id.0);
    println!("  provider:     {}", session.provider);
    println!("  model:        {}", session.model);
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

use super::session_aggregates::{
    count_tool_use_blocks, session_spend_micros,
};

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

use super::session_list::{count_sessions, list_sessions};
