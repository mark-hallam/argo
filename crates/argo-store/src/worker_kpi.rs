//! Per-worker KPI time-series store
//! (Stream `claude-worker-kpi-tracker`, mission claim 2 runtime evidence).
//!
//! Charter storage already records the *definition* of a worker's KPIs
//! on the worker row (`kpi_json`). This module records the *observations*:
//! one row per (worker, kpi name, value, timestamp) — append-only — so an
//! operator can answer "how is Maya tracking against her CSAT target this
//! week?" without grafting time-series state onto the identity row.
//!
//! KPI names are not validated against the worker's charter definition
//! list here. The first MVP need is observable values; definition-list
//! enforcement is a follow-up.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use argo_types::{ArgoError, ArgoResult};

const WORKER_KPI_SCHEMA: &str = include_str!("../migrations/39__worker_kpi_value.sql");

/// One observed KPI value. Mirrors the `worker_kpi_value` table 1:1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KpiValueRow {
    /// Auto-increment row id.
    pub id: i64,
    /// Worker id this observation belongs to.
    pub worker_id: String,
    /// Tenant boundary.
    pub tenant_id: String,
    /// KPI name (e.g. `csat`, `reply_sla_hours`). Matches whatever
    /// label the recording call used; not constrained.
    pub kpi_name: String,
    /// Observed numeric value.
    pub value: f64,
    /// Micros since epoch when the observation was recorded.
    pub observed_at: i64,
    /// Optional free-text note.
    pub note: Option<String>,
}

/// One bucketed aggregate over a contiguous range of observations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KpiBucket {
    /// Bucket start in `YYYY-MM-DD` form for `day` / `week`, or
    /// `YYYY-MM` for `month`. Stable across the row order and easy
    /// to render or sort lexically.
    pub bucket: String,
    /// Mean of `value` across the bucket.
    pub avg: f64,
    /// Minimum `value` in the bucket.
    pub min: f64,
    /// Maximum `value` in the bucket.
    pub max: f64,
    /// Number of observations in the bucket.
    pub count: i64,
}

/// Aggregation granularity for `series`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bucket {
    /// Calendar day (`YYYY-MM-DD`).
    Day,
    /// ISO calendar week, expressed as the Monday `YYYY-MM-DD`.
    Week,
    /// Calendar month (`YYYY-MM`).
    Month,
}

impl Bucket {
    /// Parse the operator-facing string used by `argo worker kpi
    /// series --bucket day|week|month`.
    pub fn parse(s: &str) -> ArgoResult<Self> {
        match s {
            "day" => Ok(Self::Day),
            "week" => Ok(Self::Week),
            "month" => Ok(Self::Month),
            _ => Err(ArgoError::Provider(format!(
                "invalid bucket '{s}': expected day|week|month"
            ))),
        }
    }
}

/// Apply the schema. Idempotent — safe to call from every entry
/// point so callers don't have to coordinate with the global
/// migration runner.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    for stmt in WORKER_KPI_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("worker_kpi schema: {e}")))?;
    }
    Ok(())
}

/// Record one KPI observation. Returns the inserted row id.
pub async fn record_value(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
    kpi_name: &str,
    value: f64,
    note: Option<&str>,
) -> ArgoResult<i64> {
    if !value.is_finite() {
        return Err(ArgoError::Provider(format!(
            "worker_kpi: value must be finite, got {value}"
        )));
    }
    if kpi_name.is_empty() {
        return Err(ArgoError::Provider(
            "worker_kpi: kpi_name must not be empty".into(),
        ));
    }
    ensure_schema(pool).await?;
    let now = Utc::now().timestamp_micros();
    let res = sqlx::query(
        "INSERT INTO worker_kpi_value
            (worker_id, tenant_id, kpi_name, value, observed_at, note)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(worker_id)
    .bind(tenant_id)
    .bind(kpi_name)
    .bind(value)
    .bind(now)
    .bind(note)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_kpi insert: {e}")))?;
    Ok(res.last_insert_rowid())
}

/// List observations for one worker, newest-first, optionally
/// filtered by KPI name and/or a `since` cutoff (inclusive).
pub async fn list_values(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
    kpi_name: Option<&str>,
    since_micros: Option<i64>,
) -> ArgoResult<Vec<KpiValueRow>> {
    ensure_schema(pool).await?;
    let mut sql = String::from(
        "SELECT id, worker_id, tenant_id, kpi_name, value, observed_at, note
         FROM worker_kpi_value
         WHERE worker_id = ? AND tenant_id = ?",
    );
    if kpi_name.is_some() {
        sql.push_str(" AND kpi_name = ?");
    }
    if since_micros.is_some() {
        sql.push_str(" AND observed_at >= ?");
    }
    sql.push_str(" ORDER BY observed_at DESC, id DESC");
    let mut q = sqlx::query(&sql).bind(worker_id).bind(tenant_id);
    if let Some(k) = kpi_name {
        q = q.bind(k);
    }
    if let Some(s) = since_micros {
        q = q.bind(s);
    }
    let rows = q
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_kpi list: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| KpiValueRow {
            id: row.get("id"),
            worker_id: row.get("worker_id"),
            tenant_id: row.get("tenant_id"),
            kpi_name: row.get("kpi_name"),
            value: row.get("value"),
            observed_at: row.get("observed_at"),
            note: row.get("note"),
        })
        .collect())
}

/// Compute bucketed avg/min/max/count for one (worker, KPI). Buckets
/// are derived in Rust from the observed timestamps so the math
/// matches the operator-facing labels regardless of the underlying
/// SQLite version's date-function support.
pub async fn series(
    pool: &SqlitePool,
    worker_id: &str,
    tenant_id: &str,
    kpi_name: &str,
    bucket: Bucket,
) -> ArgoResult<Vec<KpiBucket>> {
    let rows = list_values(pool, worker_id, tenant_id, Some(kpi_name), None).await?;
    Ok(bucket_rows(&rows, bucket))
}

/// Group values into time buckets and roll up avg/min/max/count.
/// Public so tests can drive the math without round-tripping through
/// SQLite for every assertion.
pub fn bucket_rows(rows: &[KpiValueRow], bucket: Bucket) -> Vec<KpiBucket> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for row in rows {
        let label = bucket_label(row.observed_at, bucket);
        groups.entry(label).or_default().push(row.value);
    }
    groups
        .into_iter()
        .map(|(bucket, values)| {
            let count = values.len() as i64;
            let sum: f64 = values.iter().copied().sum();
            let avg = sum / values.len() as f64;
            let min = values.iter().copied().fold(f64::INFINITY, f64::min);
            let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            KpiBucket {
                bucket,
                avg,
                min,
                max,
                count,
            }
        })
        .collect()
}

fn bucket_label(micros: i64, bucket: Bucket) -> String {
    let dt = DateTime::<Utc>::from_timestamp_micros(micros).unwrap_or_else(Utc::now);
    match bucket {
        Bucket::Day => dt.format("%Y-%m-%d").to_string(),
        Bucket::Week => {
            // ISO-week Monday — sortable lexically, easy to render.
            use chrono::{Datelike, Duration};
            let date = dt.date_naive();
            let monday = date - Duration::days(date.weekday().num_days_from_monday() as i64);
            monday.format("%Y-%m-%d").to_string()
        }
        Bucket::Month => dt.format("%Y-%m").to_string(),
    }
}

#[cfg(test)]
#[path = "worker_kpi_tests.rs"]
mod tests;
