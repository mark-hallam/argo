//! `argo messages by-session [--limit N] [--since-minutes M] [--json]`
//! — top-N sessions by message row count. Mirrors the gateway's
//! `GET /messages/by-session` (#615). Pairs with
//! `argo audit by-session` (#609) for the audit-row counterpart.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(limit: u32, since_minutes: Option<i64>, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let pool = store.pool();
    let limit = limit.clamp(1, 500);
    let rows: Vec<(Vec<u8>, i64)> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT session_id, COUNT(*) FROM message \
                 WHERE created_at >= ? \
                 GROUP BY session_id ORDER BY COUNT(*) DESC LIMIT ?",
            )
            .bind(cutoff)
            .bind(i64::from(limit))
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT session_id, COUNT(*) FROM message \
                 GROUP BY session_id ORDER BY COUNT(*) DESC LIMIT ?",
            )
            .bind(i64::from(limit))
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("messages_by_session: {e}")))?;
    let mut buckets: Vec<(String, i64)> = Vec::with_capacity(rows.len());
    for (sid_bytes, n) in rows {
        let sid_hex = match uuid::Uuid::from_slice(&sid_bytes) {
            Ok(u) => u.to_string(),
            Err(_) => hex::encode(&sid_bytes),
        };
        buckets.push((sid_hex, n));
    }
    if json {
        let arr: Vec<serde_json::Value> = buckets
            .iter()
            .map(|(s, n)| serde_json::json!({"session_id": s, "count": n}))
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"by_session": arr}))
                .unwrap_or_default()
        );
        return Ok(());
    }
    if buckets.is_empty() {
        match since_minutes {
            Some(m) => println!("(no messages in the last {m} minutes)"),
            None => println!("(no messages)"),
        }
        return Ok(());
    }
    let total: i64 = buckets.iter().map(|(_, n)| *n).sum();
    println!(
        "messages by session ({total} rows across {} session{}):",
        buckets.len(),
        if buckets.len() == 1 { "" } else { "s" }
    );
    for (s, n) in &buckets {
        println!("  {n:>8}  {s}");
    }
    Ok(())
}
