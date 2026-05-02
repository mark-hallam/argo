//! `argo session audit-by-kind <id>` + `argo session audit-by-tool <id>`
//! + `argo session tool-errors <id>` — per-session audit drill-downs.
//! Mirror the gateway's `GET /sessions/<id>/audit/by-kind` (#621),
//! `…/audit/by-tool` (#621) and `…/audit/tool-errors` (#628). Same
//! SQL the gateway runs.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run_by_kind(
    session: String,
    since_minutes: Option<i64>,
    json: bool,
) -> ArgoResult<()> {
    let id = uuid::Uuid::parse_str(&session)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{session}': {e}")))?;
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let pool = store.pool();
    let rows: Vec<(String, i64)> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT kind, COUNT(*) FROM audit_record \
                 WHERE session_id = ? AND created_at >= ? \
                 GROUP BY kind ORDER BY kind",
            )
            .bind(id.as_bytes().to_vec())
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT kind, COUNT(*) FROM audit_record \
                 WHERE session_id = ? \
                 GROUP BY kind ORDER BY kind",
            )
            .bind(id.as_bytes().to_vec())
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("session_audit_by_kind: {e}")))?;
    if json {
        let mut by_kind = serde_json::Map::new();
        for (k, n) in &rows {
            by_kind.insert(k.clone(), serde_json::json!(n));
        }
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "by_kind": serde_json::Value::Object(by_kind),
            }))
            .unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        println!("(no audit rows for session {session})");
        return Ok(());
    }
    let mut sorted = rows.clone();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.1));
    let total: i64 = sorted.iter().map(|(_, n)| *n).sum();
    println!(
        "audit by kind for session {session} ({total} rows across {} kinds):",
        sorted.len()
    );
    for (k, n) in &sorted {
        println!("  {n:>8}  {k}");
    }
    Ok(())
}

pub async fn run_by_tool(
    session: String,
    since_minutes: Option<i64>,
    json: bool,
) -> ArgoResult<()> {
    let id = uuid::Uuid::parse_str(&session)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{session}': {e}")))?;
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let pool = store.pool();
    let rows: Vec<(String, i64)> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE session_id = ? AND kind IN ('tool_call', 'tool_result') \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                   AND created_at >= ? \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .bind(id.as_bytes().to_vec())
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE session_id = ? AND kind IN ('tool_call', 'tool_result') \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .bind(id.as_bytes().to_vec())
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("session_audit_by_tool: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(t, n)| serde_json::json!({"tool": t, "count": n}))
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"by_tool": arr})).unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        println!("(no tool calls for session {session})");
        return Ok(());
    }
    let total: i64 = rows.iter().map(|(_, n)| *n).sum();
    println!(
        "audit by tool for session {session} ({total} rows across {} tools):",
        rows.len()
    );
    for (t, n) in &rows {
        println!("  {n:>8}  {t}");
    }
    Ok(())
}

pub async fn run_tool_errors(
    session: String,
    since_minutes: Option<i64>,
    json: bool,
) -> ArgoResult<()> {
    let id = uuid::Uuid::parse_str(&session)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{session}': {e}")))?;
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let pool = store.pool();
    let rows: Vec<(String, i64)> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE session_id = ? AND kind = 'tool_result' \
                   AND json_extract(payload_json, '$.is_error') = 1 \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                   AND created_at >= ? \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .bind(id.as_bytes().to_vec())
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE session_id = ? AND kind = 'tool_result' \
                   AND json_extract(payload_json, '$.is_error') = 1 \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .bind(id.as_bytes().to_vec())
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("session_audit_tool_errors: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(t, n)| serde_json::json!({"tool": t, "count": n}))
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"tool_errors": arr}))
                .unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        match since_minutes {
            Some(m) => {
                println!("(no tool_result errors for session {session} in the last {m} minutes — clean run)")
            }
            None => println!("(no tool_result errors for session {session} — clean run)"),
        }
        return Ok(());
    }
    let total: i64 = rows.iter().map(|(_, n)| *n).sum();
    println!(
        "tool errors for session {session} ({total} failures across {} tools):",
        rows.len()
    );
    for (t, n) in &rows {
        println!("  {n:>8}  {t}");
    }
    Ok(())
}
