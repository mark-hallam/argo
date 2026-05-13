//! CLI handlers for `argo session clone`, `argo session branch`,
//! and `argo session stats` — each mirrors the corresponding
//! gateway endpoint shipped in #561 / #562 / #560 so callers
//! who prefer the offline CLI can run the same operations
//! against the local store without going through the daemon.
//!
//! Lives in its own file to keep `handlers/session.rs` under
//! the 500-LOC budget as the SessionCmd enum grows.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, MessageRole, SessionId};

/// `argo session clone <id>` — duplicate a session into a
/// fresh row. Same shape as the gateway's
/// `POST /sessions/<id>/clone`.
pub async fn clone_session(raw: &str) -> ArgoResult<()> {
    let store = open_store().await?;
    let src = parse_id(raw)?;
    let new_id = clone_or_branch(&store, src, None, "clone").await?;
    println!("cloned {raw} → {}", new_id.0);
    Ok(())
}

/// `argo session branch <id> [--through-seq N]` — fork a
/// session at a specific seq. Same shape as the gateway's
/// `POST /sessions/<id>/branch?through-seq=N`.
pub async fn branch_session(raw: &str, through_seq: Option<i64>) -> ArgoResult<()> {
    let store = open_store().await?;
    let src = parse_id(raw)?;
    let new_id = clone_or_branch(&store, src, through_seq, "branch").await?;
    match through_seq {
        Some(n) => println!("branched {raw} @{n} → {}", new_id.0),
        None => println!("branched {raw} → {}", new_id.0),
    }
    Ok(())
}

/// `argo session stats <id> [--json]` — per-session summary
/// view. Same numbers the gateway's
/// `GET /sessions/<id>/stats` returns; this just runs the
/// math against the local store and prints either a paste-able
/// markdown doc or the JSON shape.
pub async fn stats_session(raw: &str, json: bool) -> ArgoResult<()> {
    let store = open_store().await?;
    let id = parse_id(raw)?;
    let rows = store
        .list_messages(id)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_messages: {e}")))?;
    let (mut user, mut assistant, mut tool, mut system, mut other): (
        u64,
        u64,
        u64,
        u64,
        u64,
    ) = (0, 0, 0, 0, 0);
    for r in &rows {
        match r.role.as_str() {
            "user" => user += 1,
            "assistant" => assistant += 1,
            "tool" => tool += 1,
            "system" => system += 1,
            _ => other += 1,
        }
    }
    let first_at = rows.first().map(|r| r.created_at);
    let last_at = rows.last().map(|r| r.created_at);
    let active_duration_seconds = match (first_at, last_at) {
        (Some(a), Some(b)) => (b - a) as f64 / 1_000_000.0,
        _ => 0.0,
    };
    let mut gaps: Vec<f64> = Vec::with_capacity(rows.len().saturating_sub(1));
    for w in rows.windows(2) {
        gaps.push((w[1].created_at - w[0].created_at) as f64 / 1_000_000.0);
    }
    let avg_gap_seconds = if gaps.is_empty() {
        0.0
    } else {
        gaps.iter().sum::<f64>() / gaps.len() as f64
    };
    let max_gap_seconds = gaps.iter().cloned().fold(0.0_f64, f64::max);
    let total = rows.len() as u64;
    if json {
        let mut by_role = serde_json::Map::new();
        by_role.insert("user".into(), user.into());
        by_role.insert("assistant".into(), assistant.into());
        by_role.insert("tool".into(), tool.into());
        by_role.insert("system".into(), system.into());
        if other > 0 {
            by_role.insert("other".into(), other.into());
        }
        let payload = serde_json::json!({
            "session_id": raw,
            "total_messages": total,
            "by_role": serde_json::Value::Object(by_role),
            "first_message_at_micros": first_at,
            "last_message_at_micros": last_at,
            "active_duration_seconds": active_duration_seconds,
            "avg_gap_seconds": avg_gap_seconds,
            "max_gap_seconds": max_gap_seconds,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
        return Ok(());
    }
    println!("# Stats for session `{raw}`\n");
    println!("- **total_messages**: {total}\n");
    println!("## by role\n");
    println!("- **user**: {user}");
    println!("- **assistant**: {assistant}");
    println!("- **tool**: {tool}");
    println!("- **system**: {system}");
    if other > 0 {
        println!("- **other**: {other}");
    }
    println!();
    println!("## timing\n");
    if let Some(t) = first_at {
        println!("- **first_message_at_micros**: {t}");
    }
    if let Some(t) = last_at {
        println!("- **last_message_at_micros**: {t}");
    }
    println!("- **active_duration_seconds**: {active_duration_seconds:.3}");
    println!("- **avg_gap_seconds**: {avg_gap_seconds:.3}");
    println!("- **max_gap_seconds**: {max_gap_seconds:.3}");
    Ok(())
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

fn parse_id(raw: &str) -> ArgoResult<SessionId> {
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Schema(format!("invalid session id '{raw}': {e}")))?;
    Ok(SessionId(id))
}

async fn clone_or_branch(
    store: &SessionStore,
    src: SessionId,
    through_seq: Option<i64>,
    mode: &str,
) -> ArgoResult<SessionId> {
    let header = store
        .get_session(src)
        .await
        .map_err(|e| ArgoError::Provider(format!("get_session: {e}")))?
        .ok_or_else(|| ArgoError::Provider(format!("no session {}", src.0)))?;
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, workspace) = store
        .ensure_workspace(name, &root)
        .await
        .map_err(|e| ArgoError::Provider(format!("ensure_workspace: {e}")))?;
    let suffix = match (mode, through_seq) {
        ("branch", Some(n)) => format!("(branch @{n})"),
        ("branch", None) => "(branch)".into(),
        _ => "(clone)".into(),
    };
    let new_title = match header.title.as_deref() {
        Some(t) if !t.is_empty() => format!("{t} {suffix}"),
        _ => suffix.clone(),
    };
    let new_id = store
        .create_session(
            tenant,
            workspace,
            &header.model,
            &header.provider,
            Some(new_title.as_str()),
            header.vertical_id.as_deref(),
        )
        .await
        .map_err(|e| ArgoError::Provider(format!("create_session: {e}")))?;
    let mut rows = store
        .list_messages(src)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_messages: {e}")))?;
    if let Some(n) = through_seq {
        rows.retain(|r| r.seq <= n);
    }
    for r in rows {
        let role = match r.role.as_str() {
            "system" => MessageRole::System,
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "tool" => MessageRole::Tool,
            other => {
                return Err(ArgoError::Provider(format!(
                    "unknown role '{other}' in source session"
                )));
            }
        };
        store
            .append_message(new_id, role, &r.content_json)
            .await
            .map_err(|e| ArgoError::Provider(format!("append_message: {e}")))?;
    }
    Ok(new_id)
}
