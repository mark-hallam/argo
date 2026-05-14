//! `argo audit by-vertical [--since-minutes N] [--json]` —
//! per-vertical histogram of audit rows (#688). Joins
//! `audit_record.session_id` → `session.vertical_id` so the
//! operator gets a count for each bound vertical (plus a
//! `null` bucket for rows from unbound sessions or audit rows
//! with no session_id).
//!
//! Mirrors the gateway's `GET /audit/by-vertical` route.
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
    let rows: Vec<(Option<String>, i64)> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT s.vertical_id, COUNT(*) \
                 FROM audit_record a \
                 LEFT JOIN session s ON s.id = a.session_id \
                 WHERE a.created_at >= ? \
                 GROUP BY s.vertical_id \
                 ORDER BY COUNT(*) DESC, s.vertical_id ASC",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT s.vertical_id, COUNT(*) \
                 FROM audit_record a \
                 LEFT JOIN session s ON s.id = a.session_id \
                 GROUP BY s.vertical_id \
                 ORDER BY COUNT(*) DESC, s.vertical_id ASC",
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| ArgoError::Provider(format!("audit_by_vertical: {e}")))?;

    if json {
        let mut by_vertical = serde_json::Map::new();
        for (v, n) in &rows {
            let key = v.clone().unwrap_or_else(|| "(none)".to_string());
            by_vertical.insert(key, serde_json::json!(n));
        }
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "by_vertical": serde_json::Value::Object(by_vertical),
            }))
            .unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        match since_minutes {
            Some(m) => println!("(no audit rows in the last {m} minutes)"),
            None => println!("(no audit rows)"),
        }
        return Ok(());
    }
    let total: i64 = rows.iter().map(|(_, n)| *n).sum();
    println!(
        "audit by vertical ({total} rows across {} vertical(s)):",
        rows.len()
    );
    for (v, n) in &rows {
        let label = v.as_deref().unwrap_or("(none)");
        println!("  {n:>8}  {label}");
    }
    Ok(())
}

#[cfg(test)]
#[path = "audit_by_vertical_tests.rs"]
mod tests;
