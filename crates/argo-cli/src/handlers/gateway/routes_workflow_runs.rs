//! `web-workflow-per-workflow-runs-table` — workflow-run aggregate +
//! recent-rows endpoint backing the `/workflows/<v>/<n>` detail page.
//!
//! Extended by `web-workflow-runs-table-pagination-filters` with
//! pagination (`page`, `page_size`), status filters
//! (`status[]=success|failed|running|queued`), and absolute date
//! range overrides (`since_ts`, `until_ts`).
//!
//! ## Path
//!
//! ```text
//! GET /api/v1/verticals/<vertical_id>/workflows/<name>/runs
//!     [?days=30]                    # legacy date window (default 30)
//!     [&since_ts=…&until_ts=…]      # absolute date range in epoch micros (overrides days)
//!     [&page=1&page_size=25]        # pagination (1-indexed; sizes 25/50/100)
//!     [&status[]=success&status[]=failed&status[]=running&status[]=queued]
//! ```
//!
//! ## Response
//!
//! ```json
//! {
//!   "vertical_id": "pr_checker",
//!   "workflow_name": "standard_review",
//!   "window_days": 30,
//!   "since_ts": 1716196800000000,
//!   "until_ts": null,
//!   "run_count": 0,        // aggregate over the date window (ignores status filters)
//!   "success_rate": null,
//!   "avg_duration_ms": 0,
//!   "p95_duration_ms": 0,
//!   "last_run_at": null,
//!   "page": 1,
//!   "page_size": 25,
//!   "total_filtered": 0,   // total rows matching status filters (drives pagination)
//!   "runs": []             // current page only
//! }
//! ```
//!
//! Aggregate fields intentionally ignore status filters — they answer
//! "what does this workflow look like over the window?" rather than
//! "what does the filtered slice look like?". Pagination
//! (`total_filtered`, `page`, `page_size`) is the only place status
//! filters apply.
//!
//! `success_rate` is `null` when `run_count == 0` (zero of zero is
//! undefined; the frontend renders "—"). Otherwise a fraction in
//! `[0.0, 1.0]`.
//!
//! ## workflow_id matching
//!
//! Migration 18's `workflow_run.workflow_id` stores whichever `id` the
//! `ExecutionReport` carries — historically that is the `workflow.id`
//! from the YAML body. We match both the synthetic
//! `"{vertical_id}/{name}"` compound key and the bare YAML id so the
//! endpoint works regardless of seeding convention.

use argo_store::SessionStore;
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;

use super::routes_workflow_runs_queries::{
    aggregate_stats, count_filtered, paged_rows, FilterParams, StatusFilter,
};

const PREFIX: &str = "/api/v1/verticals/";
const DEFAULT_DAYS: i64 = 30;
const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 1000;
const DEFAULT_PAGE_SIZE: i64 = 25;
const MAX_PAGE_SIZE: i64 = 100;

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
    let days = parse_i64(req, "days", DEFAULT_DAYS).clamp(1, 365);
    let limit = parse_i64(req, "limit", DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let page = parse_i64(req, "page", 1).max(1);
    let page_size = parse_i64(req, "page_size", DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
    let statuses = parse_statuses(req);

    let window_micros = days
        .checked_mul(86_400_000_000)
        .unwrap_or(i64::MAX / 2);
    let default_since = Utc::now()
        .timestamp_micros()
        .saturating_sub(window_micros);
    let since_ts = parse_optional_i64(req, "since_ts").unwrap_or(default_since);
    let until_ts = parse_optional_i64(req, "until_ts");

    let compound_key = format!("{vertical_id}/{name}");
    let bare_key = name.to_string();

    let stats = match aggregate_stats(store, &compound_key, &bare_key, since_ts, until_ts).await {
        Ok(s) => s,
        Err(msg) => return server(&msg),
    };

    let filter_params = FilterParams {
        compound_key: &compound_key,
        bare_key: &bare_key,
        since_ts,
        until_ts,
        statuses: &statuses,
    };

    let total_filtered = match count_filtered(store, &filter_params).await {
        Ok(c) => c,
        Err(msg) => return server(&msg),
    };

    // `limit` (legacy) still caps the per-request slice; the smaller
    // of (page_size, limit) wins so old + new params coexist safely.
    let slice = page_size.min(limit);
    let offset = (page - 1) * slice;

    let rows = match paged_rows(store, &filter_params, slice, offset).await {
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
            let status = derive_row_status(r.ok, r.finished_at_micros);
            json!({
                "id": r.id,
                "started_at": micros_to_rfc3339(r.started_at_micros),
                "finished_at": r.finished_at_micros.and_then(micros_to_rfc3339),
                "ok": r.ok.unwrap_or(false),
                "duration_ms": duration_ms,
                "status": status,
            })
        })
        .collect();

    let body = json!({
        "vertical_id": vertical_id,
        "workflow_name": name,
        "window_days": days,
        "since_ts": since_ts,
        "until_ts": until_ts,
        "run_count": stats.run_count,
        "success_rate": success_rate,
        "avg_duration_ms": avg_duration_ms,
        "p95_duration_ms": stats.p95_duration_micros / 1000,
        "last_run_at": last_run_at_iso,
        "page": page,
        "page_size": slice,
        "total_filtered": total_filtered,
        "runs": runs_json,
    });
    ("200 OK", body.to_string())
}

fn derive_row_status(ok: Option<bool>, finished_at: Option<i64>) -> &'static str {
    match (ok, finished_at) {
        (Some(true), _) => "success",
        (Some(false), _) => "failed",
        // `ok IS NULL` (with or without `finished_at`) is treated as
        // running so we never drop the row from the table.
        (None, _) => "running",
    }
}

fn parse_i64(req: &str, key: &str, default: i64) -> i64 {
    parse_optional_i64(req, key).unwrap_or(default)
}

fn parse_optional_i64(req: &str, key: &str) -> Option<i64> {
    let query = request_query(req)?;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return v.parse::<i64>().ok();
            }
        }
    }
    None
}

/// Parses both `?status=foo&status=bar` and the PHP-style
/// `?status[]=foo&status[]=bar` so callers can pick whichever form
/// their HTTP-client library encodes naturally.
fn parse_statuses(req: &str) -> Vec<StatusFilter> {
    let Some(query) = request_query(req) else { return Vec::new() };
    let mut out = Vec::new();
    for pair in query.split('&') {
        let Some((k, v)) = pair.split_once('=') else { continue };
        if k != "status" && k != "status[]" && k != "status%5B%5D" {
            continue;
        }
        let decoded = percent_decode(v);
        if let Some(s) = StatusFilter::from_str(&decoded) {
            if !out.contains(&s) {
                out.push(s);
            }
        }
    }
    out
}

fn request_query(req: &str) -> Option<&str> {
    let after_verb = req.strip_prefix("GET ")?;
    let path_with_query = after_verb.split_whitespace().next()?;
    let (_, q) = path_with_query.split_once('?')?;
    Some(q)
}

/// Decode the small set of percent-escapes we expect in status values
/// (`%5B`, `%5D`, …). Avoids pulling `percent-encoding` for one caller.
fn percent_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push(((h * 16 + l) as u8) as char);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(' ');
        } else {
            out.push(bytes[i] as char);
        }
        i += 1;
    }
    out
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
