//! SQL-backed loaders for `argo worker review`. Sibling of
//! `worker_review`; pure-data types and rendering live there. Each
//! `load_*` reads a single section of the [`super::ReviewReport`].

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use super::worker_review::{
    kpi_deltas_from_rows, micros_to_rfc3339, success_from_kinds, AnomalySection, FeedbackSample,
    HandoffSample, KpiDelta, SuccessSection, SummarySection,
};

const FEEDBACK_SAMPLE_LIMIT: i64 = 3;
const HANDOFF_SAMPLE_LIMIT: i64 = 3;

pub async fn lookup_worker_name(pool: &SqlitePool, worker_id: &str) -> ArgoResult<Option<String>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT name FROM persona WHERE worker_id = ? LIMIT 1")
            .bind(worker_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("worker review name: {e}")))?;
    Ok(row.map(|(name,)| name))
}

pub async fn load_summary(pool: &SqlitePool, worker_id: &str) -> ArgoResult<SummarySection> {
    let blob = parse_worker_blob(worker_id)?;
    let row: Option<(i64, Option<i64>)> = sqlx::query_as(
        "SELECT COUNT(*) AS n, MAX(started_at) AS last_at \
         FROM session WHERE worker_id = ?",
    )
    .bind(blob.as_slice())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review summary: {e}")))?;
    let (count, last) = row.unwrap_or((0, None));
    Ok(SummarySection {
        session_count: count,
        last_session_rfc3339: last.and_then(micros_to_rfc3339),
    })
}

pub async fn load_kpi_deltas(
    pool: &SqlitePool,
    worker_id: &str,
    period_start: DateTime<Utc>,
) -> ArgoResult<Vec<KpiDelta>> {
    // worker_kpi_value is created lazily on first write — ensure it
    // before SELECTing or an empty-db review errors.
    argo_store::worker_kpi::ensure_schema(pool).await?;
    let raw = sqlx::query(
        "SELECT kpi_name, value, observed_at \
         FROM worker_kpi_value WHERE worker_id = ? \
         ORDER BY observed_at ASC, id ASC",
    )
    .bind(worker_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review kpi: {e}")))?;
    let rows: Vec<(String, i64, f64)> = raw
        .into_iter()
        .map(|r| (r.get("kpi_name"), r.get("observed_at"), r.get("value")))
        .collect();
    Ok(kpi_deltas_from_rows(&rows, period_start))
}

pub async fn load_success(
    pool: &SqlitePool,
    worker_id: &str,
    period_start: DateTime<Utc>,
) -> ArgoResult<SuccessSection> {
    let blob = parse_worker_blob(worker_id)?;
    let cutoff = period_start.timestamp_micros();
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT a.kind FROM audit_record a \
         JOIN session s ON s.id = a.session_id \
         WHERE s.worker_id = ? AND a.created_at >= ? \
           AND a.kind IN ('approval.granted','approval.revoked','approval.consumed')",
    )
    .bind(blob.as_slice())
    .bind(cutoff)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review success: {e}")))?;
    let kinds: Vec<String> = rows.into_iter().map(|(k,)| k).collect();
    Ok(success_from_kinds(&kinds))
}

pub async fn load_feedback(pool: &SqlitePool, worker_id: &str) -> ArgoResult<Vec<FeedbackSample>> {
    let rows = sqlx::query(
        "SELECT vote, action_id, comment, given_at \
         FROM operator_feedback WHERE worker_id = ? \
         ORDER BY given_at DESC LIMIT ?",
    )
    .bind(worker_id)
    .bind(FEEDBACK_SAMPLE_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review feedback: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| FeedbackSample {
            vote: row.get("vote"),
            action_id: row.get("action_id"),
            comment: row.get::<String, _>("comment"),
            given_at_rfc3339: micros_to_rfc3339(row.get("given_at")).unwrap_or_default(),
        })
        .collect())
}

pub async fn load_handoffs_sent(
    pool: &SqlitePool,
    worker_id: &str,
) -> ArgoResult<Vec<HandoffSample>> {
    let rows = sqlx::query(
        "SELECT id, \"to\", expected_outcome, status, created_at \
         FROM handoff WHERE from_worker_id = ? \
         ORDER BY created_at DESC LIMIT ?",
    )
    .bind(worker_id)
    .bind(HANDOFF_SAMPLE_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review handoffs sent: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| HandoffSample {
            handoff_id: row.get("id"),
            counterpart: counterpart_from_to(&row.get::<String, _>("to")),
            status: row.get("status"),
            expected_outcome: row.get("expected_outcome"),
            created_at_rfc3339: micros_to_rfc3339(row.get("created_at")).unwrap_or_default(),
        })
        .collect())
}

pub async fn load_handoffs_received(
    pool: &SqlitePool,
    worker_id: &str,
) -> ArgoResult<Vec<HandoffSample>> {
    let rows = sqlx::query(
        "SELECT id, from_worker_id, expected_outcome, status, created_at \
         FROM handoff WHERE json_extract(\"to\", '$.worker_id') = ? \
         ORDER BY created_at DESC LIMIT ?",
    )
    .bind(worker_id)
    .bind(HANDOFF_SAMPLE_LIMIT)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review handoffs received: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| HandoffSample {
            handoff_id: row.get("id"),
            counterpart: row.get("from_worker_id"),
            status: row.get("status"),
            expected_outcome: row.get("expected_outcome"),
            created_at_rfc3339: micros_to_rfc3339(row.get("created_at")).unwrap_or_default(),
        })
        .collect())
}

pub async fn load_anomaly_count(
    pool: &SqlitePool,
    worker_id: &str,
    period_start: DateTime<Utc>,
) -> ArgoResult<Option<AnomalySection>> {
    if !table_exists(pool, "worker_anomaly_event").await? {
        return Ok(None);
    }
    let cutoff = period_start.timestamp_micros();
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT COUNT(*) FROM worker_anomaly_event \
         WHERE worker_id = ? AND observed_at >= ?",
    )
    .bind(worker_id)
    .bind(cutoff)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review anomalies: {e}")))?;
    Ok(Some(AnomalySection {
        count: row.map(|(n,)| n).unwrap_or(0),
    }))
}

async fn table_exists(pool: &SqlitePool, name: &str) -> ArgoResult<bool> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker review table-exists: {e}")))?;
    Ok(row.is_some())
}

/// Decode the handoff `to` JSON column into a printable counterpart.
fn counterpart_from_to(raw: &str) -> String {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) else {
        return raw.to_string();
    };
    match parsed.get("kind").and_then(|v| v.as_str()) {
        Some("worker") => parsed
            .get("worker_id")
            .and_then(|v| v.as_str())
            .unwrap_or("(worker)")
            .to_string(),
        Some("human") => parsed
            .get("addr")
            .and_then(|v| v.as_str())
            .unwrap_or("(human)")
            .to_string(),
        _ => raw.to_string(),
    }
}

/// `session.worker_id` is a 16-byte BLOB; round-trip through `Uuid`.
fn parse_worker_blob(worker_id: &str) -> ArgoResult<Vec<u8>> {
    let parsed = Uuid::from_str(worker_id)
        .map_err(|e| ArgoError::Provider(format!("invalid worker_id '{worker_id}': {e}")))?;
    Ok(parsed.as_bytes().to_vec())
}

pub async fn open_pool() -> ArgoResult<SqlitePool> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store.pool().clone())
}
