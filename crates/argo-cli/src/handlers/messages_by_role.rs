//! `argo messages by-role [--since-minutes N] [--json]` — histogram
//! of messages grouped by `role` (user/assistant/tool/system).
//! Mirrors the gateway's `GET /messages/by-role` (#611).
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(since_minutes: Option<i64>, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
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
                "SELECT role, COUNT(*) FROM message WHERE created_at >= ? \
                 GROUP BY role ORDER BY role",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as("SELECT role, COUNT(*) FROM message GROUP BY role ORDER BY role")
                .fetch_all(pool)
                .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("messages_by_role: {e}")))?;
    if json {
        let mut by_role = serde_json::Map::new();
        for (r, n) in &rows {
            by_role.insert(r.clone(), serde_json::json!(n));
        }
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "by_role": serde_json::Value::Object(by_role),
            }))
            .unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        match since_minutes {
            Some(m) => println!("(no messages in the last {m} minutes)"),
            None => println!("(no messages)"),
        }
        return Ok(());
    }
    let mut sorted = rows.clone();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.1));
    let total: i64 = sorted.iter().map(|(_, n)| *n).sum();
    println!(
        "messages by role ({total} rows across {} roles):",
        sorted.len()
    );
    for (r, n) in &sorted {
        println!("  {n:>8}  {r}");
    }
    Ok(())
}
