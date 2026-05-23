//! SQL helpers for [`super::routes_workflow_runs`]. Extracted to keep
//! the parent module under the 500-LOC ceiling once pagination +
//! status filters landed.

use argo_store::SessionStore;
use sqlx::Row;

/// Status filter values from the `?status[]=` query param. Each maps
/// to a SQL predicate fragment in [`status_clauses`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum StatusFilter {
    Success,
    Failed,
    Running,
    Queued,
}

impl StatusFilter {
    pub(super) fn from_str(s: &str) -> Option<Self> {
        match s {
            "success" => Some(Self::Success),
            "failed" => Some(Self::Failed),
            "running" => Some(Self::Running),
            "queued" => Some(Self::Queued),
            _ => None,
        }
    }
}

pub(super) struct AggregateStats {
    pub(super) run_count: i64,
    pub(super) success_count: i64,
    pub(super) completed_count: i64,
    pub(super) sum_duration_micros: i64,
    pub(super) p95_duration_micros: i64,
    pub(super) last_run_at_micros: Option<i64>,
}

pub(super) struct RunRow {
    pub(super) id: String,
    pub(super) started_at_micros: i64,
    pub(super) finished_at_micros: Option<i64>,
    pub(super) ok: Option<bool>,
}

/// Bundle of WHERE-clause inputs shared by the count + page queries.
/// Keeps each query function under the clippy 7-arg ceiling.
pub(super) struct FilterParams<'a> {
    pub(super) compound_key: &'a str,
    pub(super) bare_key: &'a str,
    pub(super) since_ts: i64,
    pub(super) until_ts: Option<i64>,
    pub(super) statuses: &'a [StatusFilter],
}

pub(super) async fn aggregate_stats(
    store: &SessionStore,
    compound_key: &str,
    bare_key: &str,
    since_ts: i64,
    until_ts: Option<i64>,
) -> Result<AggregateStats, String> {
    let until_clause = if until_ts.is_some() { " AND started_at <= ?" } else { "" };
    let sql = format!(
        "SELECT COUNT(*) AS run_count, \
                COALESCE(SUM(CASE WHEN ok = 1 THEN 1 ELSE 0 END), 0) AS success_count, \
                COALESCE(SUM(CASE WHEN finished_at IS NOT NULL THEN 1 ELSE 0 END), 0) AS completed_count, \
                COALESCE(SUM(CASE WHEN finished_at IS NOT NULL THEN (finished_at - started_at) ELSE 0 END), 0) AS sum_duration \
         FROM workflow_run \
         WHERE workflow_id IN (?, ?) AND started_at >= ?{until_clause}"
    );
    let mut q = sqlx::query(&sql)
        .bind(compound_key)
        .bind(bare_key)
        .bind(since_ts);
    if let Some(u) = until_ts {
        q = q.bind(u);
    }
    let agg_row = q
        .fetch_one(store.pool())
        .await
        .map_err(|e| format!("aggregate: {e}"))?;

    let run_count: i64 = agg_row.try_get("run_count").map_err(|e| format!("agg col: {e}"))?;
    let success_count: i64 = agg_row.try_get("success_count").map_err(|e| format!("agg col: {e}"))?;
    let completed_count: i64 = agg_row.try_get("completed_count").map_err(|e| format!("agg col: {e}"))?;
    let sum_duration_micros: i64 = agg_row.try_get("sum_duration").map_err(|e| format!("agg col: {e}"))?;

    let p95_duration_micros = p95_duration(store, compound_key, bare_key, since_ts, until_ts).await?;
    let last_run_at_micros = last_run_at(store, compound_key, bare_key).await?;

    Ok(AggregateStats {
        run_count,
        success_count,
        completed_count,
        sum_duration_micros,
        p95_duration_micros,
        last_run_at_micros,
    })
}

async fn p95_duration(
    store: &SessionStore,
    compound_key: &str,
    bare_key: &str,
    since_ts: i64,
    until_ts: Option<i64>,
) -> Result<i64, String> {
    let until_clause = if until_ts.is_some() { " AND started_at <= ?" } else { "" };
    let sql = format!(
        "SELECT (finished_at - started_at) FROM workflow_run \
         WHERE workflow_id IN (?, ?) AND started_at >= ?{until_clause} AND finished_at IS NOT NULL \
         ORDER BY (finished_at - started_at) ASC"
    );
    let mut q = sqlx::query_scalar(&sql)
        .bind(compound_key)
        .bind(bare_key)
        .bind(since_ts);
    if let Some(u) = until_ts {
        q = q.bind(u);
    }
    let durations: Vec<i64> = q
        .fetch_all(store.pool())
        .await
        .map_err(|e| format!("p95: {e}"))?;

    if durations.is_empty() {
        return Ok(0);
    }
    let idx = ((durations.len() as f64) * 0.95) as usize;
    let bounded = idx.min(durations.len() - 1);
    Ok(durations[bounded])
}

async fn last_run_at(
    store: &SessionStore,
    compound_key: &str,
    bare_key: &str,
) -> Result<Option<i64>, String> {
    let value: Option<i64> = sqlx::query_scalar(
        "SELECT MAX(started_at) FROM workflow_run WHERE workflow_id IN (?, ?)",
    )
    .bind(compound_key)
    .bind(bare_key)
    .fetch_one(store.pool())
    .await
    .map_err(|e| format!("last_run_at: {e}"))?;
    Ok(value)
}

/// SQL fragment for the status-filter `IN (…)` clause. Empty
/// `statuses` (no filter selected) returns an empty string — the
/// caller concatenates nothing into the WHERE clause. Otherwise emits
/// `AND (<predicate> OR <predicate> …)`.
///
/// Schema reminder: the `workflow_run` table has no representation for
/// "queued" (no row exists until execution starts), so requesting only
/// `queued` returns zero rows by design — the chip is preserved in the
/// UI for symmetry with the four canonical states but yields nothing
/// until queue persistence lands.
fn status_clauses(statuses: &[StatusFilter]) -> String {
    if statuses.is_empty() {
        return String::new();
    }
    let parts: Vec<&str> = statuses
        .iter()
        .map(|s| match s {
            StatusFilter::Success => "ok = 1",
            StatusFilter::Failed => "ok = 0",
            StatusFilter::Running => "(ok IS NULL OR finished_at IS NULL)",
            StatusFilter::Queued => "1 = 0",
        })
        .collect();
    format!(" AND ({})", parts.join(" OR "))
}

pub(super) async fn count_filtered(
    store: &SessionStore,
    params: &FilterParams<'_>,
) -> Result<i64, String> {
    let until_clause = if params.until_ts.is_some() { " AND started_at <= ?" } else { "" };
    let status_clause = status_clauses(params.statuses);
    let sql = format!(
        "SELECT COUNT(*) FROM workflow_run \
         WHERE workflow_id IN (?, ?) AND started_at >= ?{until_clause}{status_clause}"
    );
    let mut q = sqlx::query_scalar(&sql)
        .bind(params.compound_key)
        .bind(params.bare_key)
        .bind(params.since_ts);
    if let Some(u) = params.until_ts {
        q = q.bind(u);
    }
    let n: i64 = q
        .fetch_one(store.pool())
        .await
        .map_err(|e| format!("count: {e}"))?;
    Ok(n)
}

pub(super) async fn paged_rows(
    store: &SessionStore,
    params: &FilterParams<'_>,
    limit: i64,
    offset: i64,
) -> Result<Vec<RunRow>, String> {
    let until_clause = if params.until_ts.is_some() { " AND started_at <= ?" } else { "" };
    let status_clause = status_clauses(params.statuses);
    let sql = format!(
        "SELECT id, started_at, finished_at, ok \
         FROM workflow_run \
         WHERE workflow_id IN (?, ?) AND started_at >= ?{until_clause}{status_clause} \
         ORDER BY started_at DESC \
         LIMIT ? OFFSET ?"
    );
    let mut q = sqlx::query(&sql)
        .bind(params.compound_key)
        .bind(params.bare_key)
        .bind(params.since_ts);
    if let Some(u) = params.until_ts {
        q = q.bind(u);
    }
    q = q.bind(limit).bind(offset);
    let rows = q
        .fetch_all(store.pool())
        .await
        .map_err(|e| format!("recent rows: {e}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.try_get("id").map_err(|e| format!("row id: {e}"))?;
        let started_at_micros: i64 = row.try_get("started_at").map_err(|e| format!("row started: {e}"))?;
        let finished_at_micros: Option<i64> = row.try_get("finished_at").map_err(|e| format!("row finished: {e}"))?;
        let ok_raw: Option<i64> = row.try_get("ok").map_err(|e| format!("row ok: {e}"))?;
        out.push(RunRow {
            id,
            started_at_micros,
            finished_at_micros,
            ok: ok_raw.map(|n| n != 0),
        });
    }
    Ok(out)
}
