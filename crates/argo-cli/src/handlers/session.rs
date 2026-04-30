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
        SessionCmd::Diff { id1, id2, json } => diff_sessions(&id1, &id2, json).await,
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
            provider,
            tools: _,
            workspace: _,
        } => super::session_replay::run(&id, model.as_deref(), provider.as_deref()).await,
    }
}

async fn diff_sessions(raw1: &str, raw2: &str, json: bool) -> ArgoResult<()> {
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
    if json {
        let header_diff_objects: Vec<_> = header_diffs
            .iter()
            .map(|(field, a, b)| serde_json::json!({"field": field, "a": a, "b": b}))
            .collect();
        let row = serde_json::json!({
            "a": {"id": s1.id.0.to_string(), "messages": m1.len()},
            "b": {"id": s2.id.0.to_string(), "messages": m2.len()},
            "header_diffs": header_diff_objects,
            "messages_compared": pairs,
            "messages_diverged": diverged,
            "trailing_only": trailing,
            "first_divergence": first_div,
            "byte_identical": header_diffs.is_empty() && diverged == 0 && trailing == 0,
        });
        let serialized = serde_json::to_string(&row)
            .map_err(|e| ArgoError::Provider(format!("serialize diff: {e}")))?;
        println!("{serialized}");
        return Ok(());
    }
    println!("session diff");
    println!("  a: {} ({} messages)", s1.id.0, m1.len());
    println!("  b: {} ({} messages)", s2.id.0, m2.len());
    if header_diffs.is_empty() {
        println!("  header: identical (model/provider/title)");
    } else {
        println!("  header diffs:");
        for (field, a, b) in &header_diffs {
            println!("    {field}: a={a:?} b={b:?}");
        }
    }
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
    let messages = store.list_messages(SessionId(id)).await?;
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

/// Walk every message's `content_json` and count `tool_use`
/// blocks. One assistant message with three tool calls counts as
/// three.
fn count_tool_use_blocks(messages: &[argo_store::MessageRow]) -> u64 {
    let mut n = 0u64;
    for m in messages {
        let parsed: serde_json::Value =
            serde_json::from_str(&m.content_json).unwrap_or(serde_json::Value::Null);
        let Some(arr) = parsed.as_array() else {
            continue;
        };
        for b in arr {
            if b.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                n += 1;
            }
        }
    }
    n
}

async fn session_spend_micros(store: &SessionStore, sid: SessionId) -> ArgoResult<u64> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger WHERE session_id = ?",
    )
    .bind(sid.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("session spend sum: {e}")))?;
    Ok(u64::try_from(total).unwrap_or(0))
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

async fn list_sessions(limit: u32, json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let sessions = store.list_all_sessions(limit).await?;
    let spend_by_sid = spend_per_session(&store).await.unwrap_or_default();
    if json {
        for s in &sessions {
            let spent = spend_by_sid
                .get(s.id.0.as_bytes().as_slice())
                .copied()
                .unwrap_or(0);
            let mut value = serde_json::to_value(s)
                .map_err(|e| ArgoError::Provider(format!("serialize session: {e}")))?;
            if let serde_json::Value::Object(ref mut m) = value {
                m.insert("spent_usd_micros".into(), serde_json::Value::from(spent));
            }
            let line = serde_json::to_string(&value)
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
        "{sid:<38} {prov:<12} {model:<24} {spend:>10} title",
        sid = "session_id",
        prov = "provider",
        model = "model",
        spend = "spend",
    );
    for s in sessions {
        let spent = spend_by_sid
            .get(s.id.0.as_bytes().as_slice())
            .copied()
            .unwrap_or(0);
        println!(
            "{:<38} {:<12} {:<24} {:>10.4} {}",
            s.id.0,
            short(&s.provider, 12),
            short(&s.model, 24),
            spent as f64 / 1_000_000.0,
            s.title.as_deref().unwrap_or("")
        );
    }
    Ok(())
}

/// `session_id (blob) → SUM(amount_usd_micros)` for every session
/// that has at least one row in `spend_ledger`. One query, no
/// per-session round-trip.
async fn spend_per_session(
    store: &SessionStore,
) -> ArgoResult<std::collections::HashMap<Vec<u8>, u64>> {
    let rows: Vec<(Vec<u8>, i64)> = sqlx::query_as(
        "SELECT session_id, COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \
         GROUP BY session_id",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("spend_per_session: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(sid, total)| (sid, u64::try_from(total).unwrap_or(0)))
        .collect())
}

fn short(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
