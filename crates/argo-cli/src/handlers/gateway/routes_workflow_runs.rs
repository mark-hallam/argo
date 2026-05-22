//! `web-workflow-per-workflow-runs-table` — workflow-run aggregate +
//! recent-rows endpoint backing the `/workflows/<v>/<n>` detail page.
//!
//! Replaces the hardcoded `82` runs / `94%` success-rate / `6.2s`
//! latency literals on that page with real counts pulled from the
//! `workflow_run` table (migration 18).
//!
//! ## Path
//!
//! ```text
//! GET /api/v1/verticals/<vertical_id>/workflows/<name>/runs[?days=30&limit=100]
//! ```
//!
//! ## Response
//!
//! ```json
//! {
//!   "vertical_id": "pr_checker",
//!   "workflow_name": "standard_review",
//!   "window_days": 30,
//!   "run_count": 0,
//!   "success_rate": null,
//!   "avg_duration_ms": 0,
//!   "p95_duration_ms": 0,
//!   "last_run_at": null,
//!   "runs": []
//! }
//! ```
//!
//! `success_rate` is `null` when `run_count == 0` (zero of zero is
//! undefined; the frontend renders "—" in that case). Otherwise it is
//! a fraction in `[0.0, 1.0]`.
//!
//! ## workflow_id matching
//!
//! Migration 18's `workflow_run.workflow_id` stores whichever `id` the
//! `ExecutionReport` carries — historically that is the
//! `workflow.id` from the YAML body. The brief hinted at a synthetic
//! `"{vertical_id}/{name}"` key, but the table on operator workspaces
//! today stores either the bare `name` (matching the YAML id) or the
//! synthetic compound key. We match BOTH so the endpoint works
//! regardless of which seeding convention an operator used.

use argo_store::SessionStore;
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

const PREFIX: &str = "/api/v1/verticals/";
const DEFAULT_DAYS: i64 = 30;
const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 1000;

/// GET dispatcher. Returns `Some` for any path under
/// `/api/v1/verticals/<vertical_id>/workflows/<name>/runs`; `None`
/// lets the parent dispatcher fall through to other GET arms.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let (vertical_id, name) = parse_runs_path(req)?;
    let store = match store {
        Some(s) => s,
        None => return Some(super::store_unavailable()),
    };
    Some(list_runs_response(&vertical_id, &name, req, store).await)
}

/// Parses `GET /api/v1/verticals/<vertical_id>/workflows/<name>/runs[?...]`.
/// Returns owned strings so the borrow on `req` does not have to
/// outlive the await point inside `dispatch_get`.
fn parse_runs_path(req: &str) -> Option<(String, String)> {
    let after_verb = req.strip_prefix("GET ")?;
    let path_with_query = after_verb.split_whitespace().next()?;
    let path = path_with_query.split('?').next()?;
    let rest = path.strip_prefix(PREFIX)?;
    let mut iter = rest.splitn(2, '/');
    let vertical = iter.next().filter(|s| !s.is_empty())?;
    let tail = iter.next()?;
    let tail = tail.strip_prefix("workflows/")?;
    let segments: Vec<&str> = tail.split('/').collect();
    match segments.as_slice() {
        [name, "runs"] if !name.is_empty() => {
            Some((vertical.to_string(), (*name).to_string()))
        }
        _ => None,
    }
}

async fn list_runs_response(
    vertical_id: &str,
    name: &str,
    req: &str,
    store: &Arc<SessionStore>,
) -> (&'static str, String) {
    let days = parse_param(req, "days", DEFAULT_DAYS).clamp(1, 365);
    let limit = parse_param(req, "limit", DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    let window_micros = days
        .checked_mul(86_400_000_000)
        .unwrap_or(i64::MAX / 2);
    let cutoff = Utc::now()
        .timestamp_micros()
        .saturating_sub(window_micros);

    let compound_key = format!("{vertical_id}/{name}");
    let bare_key = name.to_string();

    let stats = match aggregate_stats(store, &compound_key, &bare_key, cutoff).await {
        Ok(s) => s,
        Err(msg) => return server(&msg),
    };

    let rows = match recent_rows(store, &compound_key, &bare_key, cutoff, limit).await {
        Ok(r) => r,
        Err(msg) => return server(&msg),
    };

    let last_run_at_iso = stats
        .last_run_at_micros
        .and_then(micros_to_rfc3339)
        .map_or(Value::Null, Value::String);

    let success_rate = if stats.run_count > 0 {
        json!(stats.success_count as f64 / stats.run_count as f64)
    } else {
        Value::Null
    };

    let avg_duration_ms = if stats.completed_count > 0 {
        stats.sum_duration_micros / stats.completed_count / 1000
    } else {
        0
    };

    let runs_json: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let duration_ms = r.finished_at_micros.map(|f| (f - r.started_at_micros) / 1000);
            json!({
                "id": r.id,
                "started_at": micros_to_rfc3339(r.started_at_micros),
                "finished_at": r.finished_at_micros.and_then(micros_to_rfc3339),
                "ok": r.ok.unwrap_or(false),
                "duration_ms": duration_ms,
            })
        })
        .collect();

    let body = json!({
        "vertical_id": vertical_id,
        "workflow_name": name,
        "window_days": days,
        "run_count": stats.run_count,
        "success_rate": success_rate,
        "avg_duration_ms": avg_duration_ms,
        "p95_duration_ms": stats.p95_duration_micros / 1000,
        "last_run_at": last_run_at_iso,
        "runs": runs_json,
    });
    ("200 OK", body.to_string())
}

struct AggregateStats {
    run_count: i64,
    success_count: i64,
    completed_count: i64,
    sum_duration_micros: i64,
    p95_duration_micros: i64,
    last_run_at_micros: Option<i64>,
}

async fn aggregate_stats(
    store: &SessionStore,
    compound_key: &str,
    bare_key: &str,
    cutoff: i64,
) -> Result<AggregateStats, String> {
    let agg_row = sqlx::query(
        "SELECT COUNT(*) AS run_count, \
                COALESCE(SUM(CASE WHEN ok = 1 THEN 1 ELSE 0 END), 0) AS success_count, \
                COALESCE(SUM(CASE WHEN finished_at IS NOT NULL THEN 1 ELSE 0 END), 0) AS completed_count, \
                COALESCE(SUM(CASE WHEN finished_at IS NOT NULL THEN (finished_at - started_at) ELSE 0 END), 0) AS sum_duration \
         FROM workflow_run \
         WHERE workflow_id IN (?, ?) AND started_at >= ?",
    )
    .bind(compound_key)
    .bind(bare_key)
    .bind(cutoff)
    .fetch_one(store.pool())
    .await
    .map_err(|e| format!("aggregate: {e}"))?;

    let run_count: i64 = agg_row.try_get("run_count").map_err(|e| format!("agg col: {e}"))?;
    let success_count: i64 = agg_row.try_get("success_count").map_err(|e| format!("agg col: {e}"))?;
    let completed_count: i64 = agg_row.try_get("completed_count").map_err(|e| format!("agg col: {e}"))?;
    let sum_duration_micros: i64 = agg_row.try_get("sum_duration").map_err(|e| format!("agg col: {e}"))?;

    let p95_duration_micros = p95_duration(store, compound_key, bare_key, cutoff).await?;
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
    cutoff: i64,
) -> Result<i64, String> {
    let durations: Vec<i64> = sqlx::query_scalar(
        "SELECT (finished_at - started_at) FROM workflow_run \
         WHERE workflow_id IN (?, ?) AND started_at >= ? AND finished_at IS NOT NULL \
         ORDER BY (finished_at - started_at) ASC",
    )
    .bind(compound_key)
    .bind(bare_key)
    .bind(cutoff)
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

struct RunRow {
    id: String,
    started_at_micros: i64,
    finished_at_micros: Option<i64>,
    ok: Option<bool>,
}

async fn recent_rows(
    store: &SessionStore,
    compound_key: &str,
    bare_key: &str,
    cutoff: i64,
    limit: i64,
) -> Result<Vec<RunRow>, String> {
    let rows = sqlx::query(
        "SELECT id, started_at, finished_at, ok \
         FROM workflow_run \
         WHERE workflow_id IN (?, ?) AND started_at >= ? \
         ORDER BY started_at DESC \
         LIMIT ?",
    )
    .bind(compound_key)
    .bind(bare_key)
    .bind(cutoff)
    .bind(limit)
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

fn parse_param(req: &str, key: &str, default: i64) -> i64 {
    let after_verb = match req.strip_prefix("GET ") {
        Some(s) => s,
        None => return default,
    };
    let path_with_query = match after_verb.split_whitespace().next() {
        Some(s) => s,
        None => return default,
    };
    let query = match path_with_query.split_once('?') {
        Some((_, q)) => q,
        None => return default,
    };
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return v.parse::<i64>().unwrap_or(default);
            }
        }
    }
    default
}

fn micros_to_rfc3339(value: i64) -> Option<String> {
    chrono::DateTime::<Utc>::from_timestamp_micros(value).map(|dt| dt.to_rfc3339())
}

fn server(msg: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": { "code": "internal", "message": msg } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_workflow_runs_tests.rs"]
mod tests;
