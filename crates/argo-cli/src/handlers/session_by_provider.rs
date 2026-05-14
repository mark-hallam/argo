//! `argo session by-provider [--since-minutes N] [--json]` —
//! histogram of sessions grouped by provider id. Mirrors the
//! gateway's `GET /sessions/by-provider` (#594). Lives in its
//! own file so handlers/session.rs stays under the LOC budget.
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
                "SELECT provider, COUNT(*) FROM session WHERE started_at >= ? \
                 GROUP BY provider ORDER BY provider",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT provider, COUNT(*) FROM session GROUP BY provider ORDER BY provider",
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("sessions_by_provider: {e}")))?;
    if json {
        let mut by_provider = serde_json::Map::new();
        for (p, n) in &rows {
            by_provider.insert(p.clone(), serde_json::json!(n));
        }
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "by_provider": serde_json::Value::Object(by_provider),
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
        "sessions by provider ({total} sessions across {} providers):",
        sorted.len()
    );
    for (p, n) in &sorted {
        println!("  {n:>8}  {p}");
    }
    Ok(())
}
