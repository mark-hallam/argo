//! SQL-facing helpers for `argo budget watch` — schema bootstrap,
//! per-worker queries, alert persistence, and row mapping.
//!
//! Loaded via `#[path = "budget_watch_store.rs"] mod budget_watch_store;`
//! from `budget_watch.rs` and re-exported with `pub(crate) use` so
//! existing call sites (and the tests file) are unchanged.

use super::{AlertCandidate, BudgetAlertRow, alert_day, micros_to_usd};
use argo_identity::AutoApprovalPolicy;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use sqlx::Row;

pub(crate) const ALERT_SCHEMA: &str =
    include_str!("../../../argo-store/migrations/26__worker_budget_alert.sql");
pub(crate) const WORKER_BUDGET_SCHEMA: &str =
    include_str!("../../../argo-store/migrations/14__worker_budget.sql");

pub(crate) async fn ensure_alert_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in ALERT_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if !sql.is_empty() {
            sqlx::query(sql)
                .execute(store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("budget alert schema: {e}")))?;
        }
    }
    Ok(())
}

pub(crate) async fn ensure_worker_budget_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in WORKER_BUDGET_SCHEMA.split_terminator(';') {
        let sql = stmt.trim();
        if !sql.is_empty() {
            sqlx::query(sql)
                .execute(store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("worker budget schema: {e}")))?;
        }
    }
    Ok(())
}

pub(crate) async fn table_exists(store: &SessionStore, table: &str) -> ArgoResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("table lookup {table}: {e}")))?;
    Ok(count > 0)
}

pub(crate) async fn worker_ids(
    store: &SessionStore,
    only: Option<&str>,
) -> ArgoResult<Vec<String>> {
    if let Some(worker) = only {
        return Ok(vec![worker.to_string()]);
    }
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT DISTINCT worker_id FROM worker_budget ORDER BY worker_id")
            .fetch_all(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("budget watch workers: {e}")))?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

pub(crate) async fn daily_cap_for_worker(
    store: &SessionStore,
    worker_id: &str,
) -> ArgoResult<Option<u64>> {
    if !table_exists(store, "worker").await? {
        return Ok(None);
    }
    let row: Option<(String,)> =
        sqlx::query_as("SELECT auto_approval_json FROM worker WHERE id = ? LIMIT 1")
            .bind(worker_id)
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("worker policy lookup: {e}")))?;
    let Some((body,)) = row else {
        return Ok(None);
    };
    let policy: AutoApprovalPolicy = serde_json::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("worker policy JSON: {e}")))?;
    Ok(policy.daily_budget_usd_micros)
}

pub(crate) async fn spend_this_month_for_worker(
    store: &SessionStore,
    worker_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<u64> {
    let start = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .unwrap_or(now)
        .timestamp();
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \
         WHERE worker_id = ? AND charged_at >= ?",
    )
    .bind(worker_id)
    .bind(start)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("budget alert month spend: {e}")))?;
    Ok(u64::try_from(total).unwrap_or(0))
}

pub(crate) async fn has_fired_on_day(
    store: &SessionStore,
    worker_id: &str,
    threshold: u8,
    day: &str,
) -> ArgoResult<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM worker_budget_alert \
         WHERE worker_id = ? AND threshold = ? AND fired_on = ? AND synthetic = 0",
    )
    .bind(worker_id)
    .bind(i64::from(threshold))
    .bind(day)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("budget alert dedup: {e}")))?;
    Ok(count > 0)
}

pub(crate) async fn insert_alert(
    store: &SessionStore,
    candidate: &AlertCandidate,
    now: DateTime<Utc>,
    notified_via: &str,
) -> ArgoResult<BudgetAlertRow> {
    let id = uuid::Uuid::now_v7().to_string();
    let day = alert_day(now);
    sqlx::query(
        "INSERT INTO worker_budget_alert \
         (id, worker_id, threshold, fired_on, fired_at, spend_amount, headroom_amount, notified_via, synthetic) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&candidate.worker_id)
    .bind(i64::from(candidate.threshold))
    .bind(&day)
    .bind(now.timestamp())
    .bind(i64::try_from(candidate.spend_amount).unwrap_or(i64::MAX))
    .bind(i64::try_from(candidate.headroom_amount).unwrap_or(i64::MAX))
    .bind(notified_via)
    .bind(if candidate.synthetic { 1_i64 } else { 0_i64 })
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("insert budget alert: {e}")))?;
    Ok(BudgetAlertRow {
        id,
        worker_id: candidate.worker_id.clone(),
        threshold: candidate.threshold,
        fired_on: day,
        fired_at: now.to_rfc3339(),
        fired_at_epoch: now.timestamp(),
        spend_amount_usd_micros: candidate.spend_amount,
        spend_amount_usd: micros_to_usd(candidate.spend_amount),
        headroom_amount_usd_micros: candidate.headroom_amount,
        headroom_amount_usd: micros_to_usd(candidate.headroom_amount),
        notified_via: notified_via.to_string(),
        synthetic: candidate.synthetic,
    })
}

pub(crate) async fn list_alerts(
    store: &SessionStore,
    worker: Option<&str>,
    since: Option<DateTime<Utc>>,
    limit: u32,
) -> ArgoResult<Vec<BudgetAlertRow>> {
    ensure_alert_schema(store).await?;
    let mut sql = String::from(
        "SELECT id, worker_id, threshold, fired_on, fired_at, spend_amount, \
         headroom_amount, notified_via, synthetic FROM worker_budget_alert",
    );
    let mut clauses = Vec::new();
    if worker.is_some() {
        clauses.push("worker_id = ?");
    }
    if since.is_some() {
        clauses.push("fired_at >= ?");
    }
    if !clauses.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&clauses.join(" AND "));
    }
    sql.push_str(" ORDER BY fired_at DESC LIMIT ?");
    let mut query = sqlx::query(&sql);
    if let Some(worker) = worker {
        query = query.bind(worker);
    }
    if let Some(since) = since {
        query = query.bind(since.timestamp());
    }
    let rows = query
        .bind(i64::from(limit.clamp(1, 500)))
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("budget alert list: {e}")))?;
    rows.into_iter().map(row_from_sql).collect()
}

pub(crate) fn row_from_sql(row: sqlx::sqlite::SqliteRow) -> ArgoResult<BudgetAlertRow> {
    let threshold = u8::try_from(row.get::<i64, _>("threshold")).unwrap_or(100);
    let spend = u64::try_from(row.get::<i64, _>("spend_amount")).unwrap_or(0);
    let headroom = u64::try_from(row.get::<i64, _>("headroom_amount")).unwrap_or(0);
    let fired_at_epoch = row.get::<i64, _>("fired_at");
    Ok(BudgetAlertRow {
        id: row.get("id"),
        worker_id: row.get("worker_id"),
        threshold,
        fired_on: row.get("fired_on"),
        fired_at: Utc
            .timestamp_opt(fired_at_epoch, 0)
            .single()
            .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
            .to_rfc3339(),
        fired_at_epoch,
        spend_amount_usd_micros: spend,
        spend_amount_usd: micros_to_usd(spend),
        headroom_amount_usd_micros: headroom,
        headroom_amount_usd: micros_to_usd(headroom),
        notified_via: row.get("notified_via"),
        synthetic: row.get::<i64, _>("synthetic") != 0,
    })
}
