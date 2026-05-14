//! `argo costs by-day [--days N] [--json]` — daily spend
//! rollup. Mirrors the gateway's `GET /costs/by-day` (#568)
//! so scripts can hit the same numbers offline. Lives in its
//! own file to keep `handlers/costs.rs` under the 500-LOC
//! budget as the CLI subcommand surface grows.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(days: u32, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let days = days.clamp(1, 365);
    let cutoff_micros =
        (chrono::Utc::now() - chrono::Duration::days(i64::from(days))).timestamp_micros();
    let sql = "SELECT \
                 strftime('%Y-%m-%d', charged_at_micros / 1000000, 'unixepoch') AS bucket, \
                 COALESCE(SUM(amount_usd_micros), 0) AS total \
               FROM spend_ledger WHERE charged_at_micros >= ? \
               GROUP BY bucket ORDER BY bucket ASC";
    let rows: Vec<(String, i64)> = sqlx::query_as(sql)
        .bind(cutoff_micros)
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("costs_by_day: {e}")))?;
    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(b, t)| {
                serde_json::json!({
                    "bucket": b,
                    "spent_usd_micros": u64::try_from(*t).unwrap_or(0),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"by_day": arr})).unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        println!(
            "(no spend rows in the last {days} days — run `argo chat` against a priced model first)"
        );
        return Ok(());
    }
    println!("spend by day (USD, last {days} days):");
    for (bucket, total) in &rows {
        println!("  {bucket}  ${:>10.4}", *total as f64 / 1_000_000.0);
    }
    Ok(())
}
