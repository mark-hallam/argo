//! `argo costs by-provider [--json]` — top-50 spend buckets
//! grouped by provider only (no model split). Mirrors the
//! gateway's `GET /costs/by-provider` (#632). Closes the CLI
//! parity gap where /costs/by-model bundles (provider, model)
//! jointly.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let sql = "SELECT provider AS bucket, \
               COALESCE(SUM(amount_usd_micros), 0) AS total \
               FROM spend_ledger GROUP BY provider \
               ORDER BY total DESC LIMIT 50";
    let rows: Vec<(String, i64)> = sqlx::query_as(sql)
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("costs_by_provider: {e}")))?;
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
            serde_json::to_string(&serde_json::json!({"by_provider": arr}))
                .unwrap_or_default()
        );
        return Ok(());
    }
    if rows.is_empty() {
        println!("(no spend rows — run `argo chat` against a priced model first)");
        return Ok(());
    }
    println!("spend by provider (USD, top {}):", rows.len());
    for (bucket, total) in &rows {
        println!("  ${:>10.4}  {bucket}", *total as f64 / 1_000_000.0);
    }
    Ok(())
}
