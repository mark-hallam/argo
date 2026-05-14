//! `argo costs anomalies` — read-only spend anomaly view.
use crate::cli::cli_workspace_cmds::CostsAnomaliesFormat;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::spend_anomaly::{
    configured_threshold, list_anomalies, AnomalyFilters, SpendAnomaly,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde_json::json;

pub async fn run(
    since: Option<&str>,
    limit: u32,
    format: CostsAnomaliesFormat,
    tenant: Option<&str>,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let since_micros = parse_since(since)?;
    let limit = limit.clamp(1, 1000);
    let fetch_limit = if tenant.is_some() { 1000 } else { limit };
    let rows = list_anomalies(
        store.pool(),
        &AnomalyFilters {
            worker_id: None,
            since_micros: Some(since_micros),
        },
        fetch_limit,
    )
    .await?;
    let rows = filter_tenant(store.pool(), rows, tenant, limit).await?;
    match format {
        CostsAnomaliesFormat::Json => print_json(&rows, store.pool()).await,
        CostsAnomaliesFormat::Markdown => print_markdown(&rows, store.pool()).await,
    }
}

fn parse_since(since: Option<&str>) -> ArgoResult<i64> {
    let dt = match since {
        None => Utc::now() - Duration::days(7),
        Some(s) => match DateTime::parse_from_rfc3339(s) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(_) => {
                let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .map_err(|e| ArgoError::Provider(format!("parse --since: {e}")))?;
                let time = date
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| ArgoError::Provider("parse --since: invalid date".into()))?;
                DateTime::<Utc>::from_naive_utc_and_offset(time, Utc)
            }
        },
    };
    Ok(dt.timestamp_micros())
}

async fn filter_tenant(
    pool: &sqlx::SqlitePool,
    rows: Vec<SpendAnomaly>,
    tenant: Option<&str>,
    limit: u32,
) -> ArgoResult<Vec<SpendAnomaly>> {
    let mut out = Vec::new();
    for row in rows {
        if let Some(t) = tenant {
            if tenant_for_worker(pool, &row.worker_id).await?.as_deref() != Some(t) {
                continue;
            }
        }
        out.push(row);
        if out.len() >= limit as usize {
            break;
        }
    }
    Ok(out)
}

async fn worker_table_exists(pool: &sqlx::SqlitePool) -> ArgoResult<bool> {
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'worker' LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker table lookup: {e}")))?;
    Ok(exists.is_some())
}

async fn tenant_for_worker(pool: &sqlx::SqlitePool, worker_id: &str) -> ArgoResult<Option<String>> {
    if !worker_table_exists(pool).await? {
        return Ok(None);
    }
    sqlx::query_scalar("SELECT tenant_id FROM worker WHERE id = ? LIMIT 1")
        .bind(worker_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker tenant lookup: {e}")))
}

async fn print_json(rows: &[SpendAnomaly], pool: &sqlx::SqlitePool) -> ArgoResult<()> {
    let mut out = Vec::new();
    for row in rows {
        out.push(json_row(
            row,
            tenant_for_worker(pool, &row.worker_id).await?,
        ));
    }
    let body =
        serde_json::to_string(&out).map_err(|e| ArgoError::Provider(format!("json: {e}")))?;
    println!("{body}");
    Ok(())
}

async fn print_markdown(rows: &[SpendAnomaly], pool: &sqlx::SqlitePool) -> ArgoResult<()> {
    if rows.is_empty() {
        println!("No anomalies in window");
        return Ok(());
    }
    println!("| timestamp | tenant_id | worker | baseline | observed | z_score | severity |");
    println!("|---|---|---|---:|---:|---:|---|");
    for row in rows {
        let tenant = tenant_for_worker(pool, &row.worker_id)
            .await?
            .unwrap_or_else(|| "-".into());
        println!(
            "| {} | {} | {} | ${:.4} | ${:.4} | {:+.2} | {} |",
            timestamp(row.observed_at_micros),
            tenant,
            row.worker_id,
            row.baseline_mean_usd,
            row.observed_usd_per_hour,
            row.z_score,
            severity(row.z_score),
        );
    }
    Ok(())
}

fn json_row(row: &SpendAnomaly, tenant_id: Option<String>) -> serde_json::Value {
    json!({
        "id": &row.id,
        "timestamp": timestamp(row.observed_at_micros),
        "tenant_id": tenant_id,
        "worker_id": &row.worker_id,
        "baseline_usd_per_hour": row.baseline_mean_usd,
        "observed_usd_per_hour": row.observed_usd_per_hour,
        "baseline_stddev_usd": row.baseline_stddev_usd,
        "z_score": row.z_score,
        "severity": severity(row.z_score),
    })
}

fn timestamp(micros: i64) -> String {
    match DateTime::<Utc>::from_timestamp_micros(micros) {
        Some(dt) => dt.to_rfc3339(),
        None => micros.to_string(),
    }
}

fn severity(z: f64) -> &'static str {
    let z = z.abs();
    let threshold = configured_threshold();
    if z >= threshold * 2.0 {
        "crit"
    } else if z >= threshold {
        "warn"
    } else {
        "info"
    }
}
