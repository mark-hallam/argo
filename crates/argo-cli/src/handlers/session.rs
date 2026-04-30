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
        SessionCmd::List { limit } => list_sessions(limit).await,
        SessionCmd::Show { id } => show_session(&id).await,
        SessionCmd::Export { id, out } => export_session(&id, out.as_deref()).await,
        SessionCmd::Diff { id1, id2 } => {
            // The diff against the live SQLite store needs the
            // tool_call → TurnRecord projection, which lands with
            // the daemon's session-replay-source binding (separate
            // PR). Without it, we surface a clear "needs daemon"
            // message rather than a half-working diff.
            println!(
                "session diff {id1} {id2}: requires the daemon's session-source binding (separate PR)."
            );
            Err(ArgoError::Provider(
                "session diff requires the daemon binding".into(),
            ))
        }
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

async fn show_session(raw: &str) -> ArgoResult<()> {
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

async fn list_sessions(limit: u32) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let sessions = store.list_all_sessions(limit).await?;
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
