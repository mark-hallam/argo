//! `argo session` — list / diff / bisect / replay (offline ops
//! only; the live `replay` against a different model lives with
//! the daemon binding).
use crate::cli::SessionCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};

pub async fn run(cmd: SessionCmd) -> ArgoResult<()> {
    match cmd {
        SessionCmd::List {
            limit,
            json,
            format,
            since_minutes,
            older_than_minutes,
            spent_min,
            spent_max,
            provider,
            model,
            status,
            title_contains,
            sort_by,
        } => {
            list_sessions(
                limit,
                json,
                &format,
                since_minutes,
                older_than_minutes,
                spent_min,
                spent_max,
                provider,
                model,
                status,
                title_contains,
                sort_by,
            )
            .await
        }
        SessionCmd::Show { id, json, format } => show_session(&id, json, &format).await,
        SessionCmd::Count {
            status,
            since_minutes,
            older_than_minutes,
            provider,
            model,
            title_contains,
            spent_min,
            spent_max,
            json,
        } => {
            count_sessions(
                status,
                since_minutes,
                older_than_minutes,
                provider,
                model,
                title_contains,
                spent_min,
                spent_max,
                json,
            )
            .await
        }
        SessionCmd::Watch { id } => super::session_watch::run(&id).await,
        SessionCmd::Messages {
            id,
            json,
            format,
            limit,
            tail,
            role,
            since_minutes,
            seq_min,
            seq_max,
            content_contains,
        } => {
            super::session_messages::run(
                &id,
                json,
                &format,
                limit,
                tail,
                role.as_deref(),
                since_minutes,
                seq_min,
                seq_max,
                content_contains.as_deref(),
            )
            .await
        }
        SessionCmd::AttachSkill { id, skill } => {
            super::session_skills::attach(&id, &skill).await
        }
        SessionCmd::DetachSkill { id, skill } => {
            super::session_skills::detach(&id, &skill).await
        }
        SessionCmd::Skills { id } => super::session_skills::list(&id).await,
        SessionCmd::ClearSkills { id } => super::session_skills::clear(&id).await,
        SessionCmd::Export {
            id,
            out,
            include_audit,
            format,
        } => super::session_export::export_session(
            &id,
            out.as_deref(),
            include_audit,
            &format,
        )
        .await,
        SessionCmd::Search {
            query,
            limit,
            session,
            role,
            json,
            format,
            count,
        } => {
            super::session_search::run(
                &query,
                limit,
                session.as_deref(),
                role.as_deref(),
                json,
                &format,
                count,
            )
            .await
        }
        SessionCmd::Delete { id, force } => delete_session(&id, force).await,
        SessionCmd::Diff {
            id1,
            id2,
            json,
            format,
        } => diff_sessions(&id1, &id2, json, &format).await,
        SessionCmd::Bisect { id, hypothesis } => {
            super::session_bisect::run(&id, &hypothesis).await
        }
        SessionCmd::Title { id, new, clear } => {
            set_title(&id, new.as_deref(), clear).await
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
            stream,
        } => {
            super::session_replay::run(&id, model.as_deref(), provider.as_deref(), stream)
                .await
        }
        SessionCmd::Clone { id } => {
            super::session_clone_branch::clone_session(&id).await
        }
        SessionCmd::Branch { id, through_seq } => {
            super::session_clone_branch::branch_session(&id, through_seq).await
        }
        SessionCmd::Stats { id, json } => {
            super::session_clone_branch::stats_session(&id, json).await
        }
        SessionCmd::ByDay { days, json } => {
            super::session_by_day::run(days, json).await
        }
        SessionCmd::ByProvider { since_minutes, json } => {
            super::session_by_provider::run(since_minutes, json).await
        }
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

/// `argo session title <id> <new> | --clear` — set or clear a
/// session's title. `--clear` writes SQL NULL; `<new>` is
/// trimmed and an empty trim clears too. Mirrors the gateway's
/// `POST /sessions/<id>/title`.
async fn set_title(raw: &str, new: Option<&str>, clear: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    let title = if clear { "" } else { new.unwrap_or("") };
    let n = store.set_session_title(SessionId(id), title).await?;
    if n == 0 {
        println!("(session {raw} not found)");
    } else if clear || title.trim().is_empty() {
        println!("cleared title on {raw}");
    } else {
        println!("set title on {raw} to {title:?}");
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

async fn show_session(raw: &str, json: bool, format: &str) -> ArgoResult<()> {
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
