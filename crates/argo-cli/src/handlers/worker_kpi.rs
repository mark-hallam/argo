//! `argo worker kpi record/list/series` (Stream
//! `claude-worker-kpi-tracker`, mission claim 2 runtime evidence).
//!
//! Charter storage owns KPI *definitions*; this handler records and
//! reports observed *values*. SQL contract lives in
//! `argo_store::worker_kpi`. KPI names are not validated against the
//! charter `kpi_json` definition list here — the first MVP need is
//! observable time-series data.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::worker_kpi::{self, Bucket, KpiBucket, KpiValueRow};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde_json::json;

/// `argo worker kpi record <worker_id> <kpi> <value> --tenant <t> [--note <n>] [--json]`.
pub async fn record(
    worker_id: &str,
    kpi: &str,
    value: f64,
    tenant: &str,
    note: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let pool = open_pool().await?;
    let id = worker_kpi::record_value(&pool, worker_id, tenant, kpi, value, note).await?;
    if json {
        let payload = json!({
            "ok": true,
            "id": id,
            "worker_id": worker_id,
            "tenant_id": tenant,
            "kpi": kpi,
            "value": value,
            "note": note,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        let suffix = note.map(|n| format!(" — {n}")).unwrap_or_default();
        println!("worker {worker_id}: {kpi} = {value}{suffix} (#{id})");
    }
    Ok(())
}

/// `argo worker kpi list <worker_id> --tenant <t> [--kpi <name>] [--since <rfc3339>] [--json]`.
pub async fn list(
    worker_id: &str,
    tenant: &str,
    kpi: Option<&str>,
    since: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let since_micros = since.map(parse_since).transpose()?;
    let pool = open_pool().await?;
    let rows = worker_kpi::list_values(&pool, worker_id, tenant, kpi, since_micros).await?;
    if json {
        let payload = json!({
            "ok": true,
            "worker_id": worker_id,
            "tenant_id": tenant,
            "kpi": kpi,
            "count": rows.len(),
            "rows": rows
                .iter()
                .map(|r| {
                    json!({
                        "id": r.id,
                        "kpi": r.kpi_name,
                        "value": r.value,
                        "observed_at": r.observed_at,
                        "observed_at_rfc3339": format_iso(r.observed_at),
                        "note": r.note,
                    })
                })
                .collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else if rows.is_empty() {
        println!("(no kpi observations)");
    } else {
        print_value_table(&rows);
    }
    Ok(())
}

/// `argo worker kpi series <worker_id> <kpi> --tenant <t> --bucket <day|week|month> [--json]`.
pub async fn series(
    worker_id: &str,
    kpi: &str,
    tenant: &str,
    bucket: &str,
    json: bool,
) -> ArgoResult<()> {
    let bucket = Bucket::parse(bucket)?;
    let pool = open_pool().await?;
    let buckets = worker_kpi::series(&pool, worker_id, tenant, kpi, bucket).await?;
    if json {
        let payload = json!({
            "ok": true,
            "worker_id": worker_id,
            "tenant_id": tenant,
            "kpi": kpi,
            "bucket": bucket_label(bucket),
            "count": buckets.len(),
            "rows": buckets
                .iter()
                .map(|b| {
                    json!({
                        "bucket": b.bucket,
                        "avg": b.avg,
                        "min": b.min,
                        "max": b.max,
                        "count": b.count,
                    })
                })
                .collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else if buckets.is_empty() {
        println!("(no kpi observations)");
    } else {
        print_bucket_table(&buckets);
    }
    Ok(())
}

fn print_value_table(rows: &[KpiValueRow]) {
    println!(
        "{:<25}  {:<24}  {:>12}  note",
        "observed_at", "kpi", "value"
    );
    for row in rows {
        let ts = format_iso(row.observed_at);
        let note = row.note.as_deref().unwrap_or("");
        println!(
            "{ts:<25}  {:<24}  {:>12.4}  {note}",
            row.kpi_name, row.value
        );
    }
}

fn print_bucket_table(buckets: &[KpiBucket]) {
    println!(
        "{:<12}  {:>12}  {:>12}  {:>12}  {:>6}",
        "bucket", "avg", "min", "max", "count"
    );
    for b in buckets {
        println!(
            "{:<12}  {:>12.4}  {:>12.4}  {:>12.4}  {:>6}",
            b.bucket, b.avg, b.min, b.max, b.count
        );
    }
}

fn format_iso(micros: i64) -> String {
    DateTime::<Utc>::from_timestamp_micros(micros)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| micros.to_string())
}

fn bucket_label(bucket: Bucket) -> &'static str {
    match bucket {
        Bucket::Day => "day",
        Bucket::Week => "week",
        Bucket::Month => "month",
    }
}

/// Convert an RFC3339 timestamp into micros-since-epoch for the
/// store's `observed_at` filter. Surfaces a friendly parse error
/// instead of leaking chrono's verbose default.
pub(crate) fn parse_since(s: &str) -> ArgoResult<i64> {
    let dt = DateTime::parse_from_rfc3339(s).map_err(|e| {
        ArgoError::Provider(format!(
            "invalid --since '{s}' (expected RFC3339, e.g. 2026-05-01T00:00:00Z): {e}"
        ))
    })?;
    Ok(dt.with_timezone(&Utc).timestamp_micros())
}

async fn open_pool() -> ArgoResult<sqlx::SqlitePool> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store.pool().clone())
}

#[cfg(test)]
#[path = "worker_kpi_tests.rs"]
mod tests;
