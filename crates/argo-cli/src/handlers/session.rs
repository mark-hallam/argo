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
        SessionCmd::List { limit, json } => list_sessions(limit, json).await,
        SessionCmd::Show { id, json } => show_session(&id, json).await,
        SessionCmd::Export { id, out } => export_session(&id, out.as_deref()).await,
        SessionCmd::Search {
            query,
            limit,
            session,
        } => search(&query, limit, session.as_deref()).await,
        SessionCmd::Delete { id, force } => delete_session(&id, force).await,
        SessionCmd::Diff { id1, id2 } => diff_sessions(&id1, &id2).await,
        SessionCmd::Bisect { id, hypothesis } => {
            println!(
                "session bisect {id} '{hypothesis}': same as diff — needs the daemon binding."
            );
            Err(ArgoError::Provider(
                "session bisect requires the daemon binding".into(),
            ))
        }
        SessionCmd::Replay {
            id,
            model,
            tools,
            workspace,
        } => {
            println!("session replay {id}");
            if let Some(m) = model {
                println!("  --model {m}");
            }
            if let Some(t) = tools {
                println!("  --tools {t}");
            }
            if let Some(w) = workspace {
                println!("  --workspace {w}");
            }
            println!(
                "  (verbatim replay against the recorded provider response is the daemon's job;"
            );
            println!("   the argo-replay library is built and tested at the lib tier.)");
            Ok(())
        }
    }
}

async fn diff_sessions(raw1: &str, raw2: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let id1 = uuid::Uuid::parse_str(raw1)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw1}': {e}")))?;
    let id2 = uuid::Uuid::parse_str(raw2)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw2}': {e}")))?;
    let s1 = store
        .get_session(SessionId(id1))
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {raw1}")))?;
    let s2 = store
        .get_session(SessionId(id2))
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {raw2}")))?;
    let m1 = store.list_messages(SessionId(id1)).await?;
    let m2 = store.list_messages(SessionId(id2)).await?;
    println!("session diff");
    println!("  a: {} ({} messages)", s1.id.0, m1.len());
    println!("  b: {} ({} messages)", s2.id.0, m2.len());
    let mut header_diffs: Vec<(&str, String, String)> = Vec::new();
    if s1.model != s2.model {
        header_diffs.push(("model", s1.model.clone(), s2.model.clone()));
    }
    if s1.provider != s2.provider {
        header_diffs.push(("provider", s1.provider.clone(), s2.provider.clone()));
    }
    if s1.title != s2.title {
        header_diffs.push((
            "title",
            s1.title.clone().unwrap_or_default(),
            s2.title.clone().unwrap_or_default(),
        ));
    }
    if header_diffs.is_empty() {
        println!("  header: identical (model/provider/title)");
    } else {
        println!("  header diffs:");
        for (field, a, b) in &header_diffs {
            println!("    {field}: a={a:?} b={b:?}");
        }
    }
    let pairs = m1.len().min(m2.len());
    let mut first_div: Option<usize> = None;
    let mut diverged = 0_usize;
    for i in 0..pairs {
        if m1[i].role != m2[i].role || m1[i].content_json != m2[i].content_json {
            diverged += 1;
            if first_div.is_none() {
                first_div = Some(i);
            }
        }
    }
    let trailing = m1.len().abs_diff(m2.len());
    println!(
        "  messages: {} compared, {diverged} diverged, {trailing} trailing-only on the longer side",
        pairs
    );
    if let Some(i) = first_div {
        println!(
            "  first divergence: seq {} (role a={} b={})",
            i, m1[i].role, m2[i].role
        );
    }
    if header_diffs.is_empty() && diverged == 0 && trailing == 0 {
        println!("sessions are byte-identical");
    }
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

async fn search(query: &str, limit: u32, session: Option<&str>) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let hits = match session {
        Some(raw) => {
            let id = uuid::Uuid::parse_str(raw)
                .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
            store
                .search_messages_in_session(SessionId(id), query, limit)
                .await?
        }
        None => store.search_messages(query, limit).await?,
    };
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

async fn export_session(raw: &str, out: Option<&str>) -> ArgoResult<()> {
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
    writer
        .flush()
        .map_err(|e| ArgoError::Provider(format!("flush: {e}")))?;
    if out.is_some() {
        eprintln!("session export: wrote {} messages", messages.len());
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
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    let session = store
        .get_session(SessionId(id))
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no session {raw}")))?;
    let count = store.message_count(SessionId(id)).await?;
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
    Ok(())
}

async fn list_sessions(limit: u32, json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let sessions = store.list_all_sessions(limit).await?;
    if json {
        for s in &sessions {
            let line = serde_json::to_string(s)
                .map_err(|e| ArgoError::Provider(format!("serialize session: {e}")))?;
            println!("{line}");
        }
        return Ok(());
    }
    if sessions.is_empty() {
        println!("(no sessions in {})", db.display());
        return Ok(());
    }
    println!(
        "{sid:<38} {prov:<12} {model:<24} title",
        sid = "session_id",
        prov = "provider",
        model = "model"
    );
    for s in sessions {
        println!(
            "{:<38} {:<12} {:<24} {}",
            s.id.0,
            short(&s.provider, 12),
            short(&s.model, 24),
            s.title.as_deref().unwrap_or("")
        );
    }
    Ok(())
}

fn short(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
