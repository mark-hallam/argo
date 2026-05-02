//! `argo audit by-day-by-kind [--days N] [--json]` — 2D
//! rollup: audit rows bucketed by `(day, kind)`. Mirrors the
//! gateway's `GET /audit/by-day-by-kind` (#638). Same SQL the
//! gateway runs.
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
                 strftime('%Y-%m-%d', created_at / 1000000, 'unixepoch') AS bucket, \
                 kind, \
                 COUNT(*) AS n \
               FROM audit_record WHERE created_at >= ? \
               GROUP BY bucket, kind \
               ORDER BY bucket ASC, n DESC";
    let rows: Vec<(String, String, i64)> = sqlx::query_as(sql)
        .bind(cutoff_micros)
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("audit_by_day_by_kind: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(b, k, c)| serde_json::json!({"bucket": b, "kind": k, "count": c}))
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"by_day_by_kind": arr}))
                .unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        println!("(no audit rows in the last {days} days)");
        return Ok(());
    }
    println!(
        "audit by day × kind ({} cells, last {days} days):",
        rows.len()
    );
    for (b, k, c) in &rows {
        println!("  {b}  {k:<24} {c:>6}");
    }
    Ok(())
}
