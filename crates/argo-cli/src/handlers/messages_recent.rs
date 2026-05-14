//! `argo messages recent [--limit N] [--role <r>] [--since-minutes M] [--json]`
//! — workspace-wide recent message tail across all sessions.
//! Mirrors the gateway's `GET /messages/recent` (#619). Default
//! limit 50, capped 500.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(
    limit: u32,
    role: Option<String>,
    since_minutes: Option<i64>,
    json: bool,
) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let pool = store.pool();
    let limit = limit.clamp(1, 500);
    let since_micros = since_minutes
        .map(|m| (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros());
    // The 6-tuple mirrors the SELECT shape; introducing a
    // type alias for what's local to one query would obscure
    // the column order rather than clarify it.
    #[allow(clippy::type_complexity)]
    let rows: Vec<(i64, Vec<u8>, Vec<u8>, String, String, i64)> = match (role.as_deref(), since_micros) {
        (Some(r), Some(s)) => sqlx::query_as(
            "SELECT seq, id, session_id, role, content_json, created_at \
             FROM message WHERE role = ? AND created_at >= ? \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(r)
        .bind(s)
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await,
        (Some(r), None) => sqlx::query_as(
            "SELECT seq, id, session_id, role, content_json, created_at \
             FROM message WHERE role = ? \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(r)
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await,
        (None, Some(s)) => sqlx::query_as(
            "SELECT seq, id, session_id, role, content_json, created_at \
             FROM message WHERE created_at >= ? \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(s)
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await,
        (None, None) => sqlx::query_as(
            "SELECT seq, id, session_id, role, content_json, created_at \
             FROM message ORDER BY created_at DESC LIMIT ?",
        )
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await,
    }
    .map_err(|e| ArgoError::Provider(format!("messages_recent: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(seq, id, sid, role, content, ts)| {
                let sid_hex = uuid::Uuid::from_slice(sid)
                    .map(|u| u.to_string())
                    .unwrap_or_else(|_| hex::encode(sid));
                serde_json::json!({
                    "seq": seq,
                    "id": hex::encode(id),
                    "session_id": sid_hex,
                    "role": role,
                    "content": serde_json::from_str::<serde_json::Value>(content)
                        .unwrap_or(serde_json::Value::Null),
                    "created_at_micros": ts,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&arr).unwrap_or_default());
        return Ok(());
    }
    if rows.is_empty() {
        println!("(no messages)");
        return Ok(());
    }
    println!("messages recent ({} rows):", rows.len());
    for (seq, _id, sid, role, content, _ts) in &rows {
        let sid_hex = uuid::Uuid::from_slice(sid)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| hex::encode(sid));
        let preview: String = content.chars().take(80).collect();
        let preview = preview.replace('\n', " ");
        println!("  [{seq}] {role:<9} {sid_hex}  {preview}");
    }
    Ok(())
}
