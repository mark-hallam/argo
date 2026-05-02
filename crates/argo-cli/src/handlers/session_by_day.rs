//! `argo session by-day [--days N] [--json]` — daily session
//! creation rollup. Mirrors the gateway's
//! `GET /sessions/by-day` (#569). Lives in its own file to
//! keep `handlers/session.rs` under the 500-LOC budget.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(days: u32, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let days = days.clamp(1, 365);
    let cutoff_micros =
        (chrono::Utc::now() - chrono::Duration::days(i64::from(days))).timestamp_micros();
    let sql = "SELECT \
                 strftime('%Y-%m-%d', started_at / 1000000, 'unixepoch') AS bucket, \
                 COUNT(*) AS n \
               FROM session WHERE started_at >= ? \
               GROUP BY bucket ORDER BY bucket ASC";
    let rows: Vec<(String, i64)> = sqlx::query_as(sql)
        .bind(cutoff_micros)
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("sessions_by_day: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(b, c)| serde_json::json!({"bucket": b, "count": c}))
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"by_day": arr})).unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        println!("(no sessions in the last {days} days)");
        return Ok(());
    }
    println!("sessions by day (last {days} days):");
    for (bucket, count) in &rows {
        println!("  {bucket}  {count:>6}");
    }
    Ok(())
}
