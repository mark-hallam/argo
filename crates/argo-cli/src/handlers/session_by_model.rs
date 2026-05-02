//! `argo session by-model [--since-minutes N] [--json]` —
//! histogram of sessions grouped by model. Mirrors the
//! gateway's `GET /sessions/by-model` (#600).
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(since_minutes: Option<i64>, json: bool) -> ArgoResult<()> {
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
                "SELECT model, COUNT(*) FROM session WHERE started_at >= ? \
                 GROUP BY model ORDER BY model",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT model, COUNT(*) FROM session GROUP BY model ORDER BY model",
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("sessions_by_model: {e}")))?;
    if json {
        let mut by_model = serde_json::Map::new();
        for (m, n) in &rows {
            by_model.insert(m.clone(), serde_json::json!(n));
        }
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "by_model": serde_json::Value::Object(by_model),
            }))
            .unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        match since_minutes {
            Some(m) => println!("(no sessions in the last {m} minutes)"),
            None => println!("(no sessions)"),
        }
        return Ok(());
    }
    let mut sorted = rows.clone();
    sorted.sort_by_key(|r| std::cmp::Reverse(r.1));
    let total: i64 = sorted.iter().map(|(_, n)| *n).sum();
    println!(
        "sessions by model ({total} sessions across {} models):",
        sorted.len()
    );
    for (m, n) in &sorted {
        println!("  {n:>8}  {m}");
    }
    Ok(())
}
