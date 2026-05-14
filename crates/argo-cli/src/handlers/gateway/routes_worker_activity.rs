//! `GET /api/v1/workers/<id>/activity` — per-worker audit timeline (Stream J9).
//!
//! Worker-scoped slice of J14's `/api/v1/audit` feed. Cursor-paginated
//! JSON over `audit_record` filtered to one `worker_id` with the v1
//! three-dim filter set (`range`, `type`, `status`). Same redaction +
//! audit-emit pattern as J14, same 404/403/410 contract as J7.
//!
//! Sensible deviations from the brief:
//!
//! - Audit kind is `AuditKind::HttpRequest` with payload
//!   `endpoint: "worker.activity.viewed"` (J7/J14 convention; an enum
//!   variant addition would touch every crate's exhaustive match).
//! - `?type` and `?status` are single-valued for v1 (per brief sample
//!   `?type=message_sent`). Csv multi-value follows under
//!   `mission-enterprise-activity-filter-parity`.
//! - `worker.name` comes from the live `Worker.persona`, not from the
//!   audit row's embedded `worker_name` — audit rows can carry stale
//!   names after a rename.

use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{tail_rows_filtered, AuditKind, AuditSink, ExportedRow, TailFilters};
use argo_identity::{IdentityStore, WorkerId};
use argo_redact::{redact_text, RedactionPolicy};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Datelike, Duration, Utc};
use serde_json::{json, Value};
use std::sync::Arc;

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 100;
const DEFAULT_RANGE: &str = "7d";
const RAW_FETCH_CAP: u32 = 5_000;
const RECOVERABLE_DAYS: i64 = 30;

/// `GET /api/v1/workers/<id>/activity` entry point. `worker_id_raw`
/// is the path segment after `/workers/` and before `/activity`; the
/// caller in `routes_dispatch_get` already strips the `/activity` tail.
pub(super) async fn list_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match worker_id_raw.parse::<WorkerId>() {
        Ok(id) => id,
        Err(e) => return bad_request(&format!("invalid worker_id '{worker_id_raw}': {e}")),
    };
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let params = match parse_query_params(tail) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let identities = IdentityStore::new(store);
    let worker = match identities.get_worker(tenant, worker_id).await {
        Ok(Some(w)) => w,
        Ok(None) => match exists_in_other_tenant(store, tenant, worker_id).await {
            Ok(true) => return forbidden(&format!("worker {worker_id} not in this tenant")),
            Ok(false) => return not_found(&format!("worker {worker_id} not found")),
            Err(e) => return server_error(&format!("cross-tenant probe: {e}")),
        },
        Err(e) => return server_error(&format!("get worker: {e}")),
    };
    match fetch_archived_at(store, tenant, worker_id).await {
        Ok(Some(ts)) => return gone_response(worker_id, &ts),
        Ok(None) => {}
        Err(e) => return server_error(&format!("archived lookup: {e}")),
    }

    let filters = TailFilters {
        since_micros: params.since_micros,
        ..Default::default()
    };
    let raw = match tail_rows_filtered(store, RAW_FETCH_CAP, &filters).await {
        Ok(rows) => rows,
        Err(e) => return server_error(&format!("audit fetch: {e}")),
    };
    let worker_id_str = worker_id.to_string();
    let mut ordered = raw;
    // tail_rows_filtered returns ASC within the LIMIT window — flip to DESC.
    ordered.sort_by(|a, b| {
        b.created_at_micros
            .cmp(&a.created_at_micros)
            .then_with(|| b.id.cmp(&a.id))
    });
    let policy = RedactionPolicy::default();
    let page: Vec<&ExportedRow> = ordered
        .iter()
        .filter(|r| row_belongs_to_worker(r, &worker_id_str))
        .filter(|r| row_passes_filters(r, &params))
        .filter(|r| past_cursor(r, params.cursor.as_ref()))
        .take(params.limit)
        .collect();
    let next_cursor = (page.len() == params.limit)
        .then(|| page.last().map(|r| encode_cursor(r.created_at_micros, &r.id)))
        .flatten();
    let worker_name = worker.persona.name.clone();
    let items: Vec<Value> = page
        .iter()
        .map(|row| activity_row_json(row, &worker_id_str, &worker_name, &policy))
        .collect();
    emit_view_audit(store, req, worker_id, &params, items.len()).await;
    let mut body = json!({ "items": items });
    if let Some(c) = next_cursor {
        body["next_cursor"] = Value::String(c);
    }
    ("200 OK", body.to_string())
}

#[derive(Debug, Default)]
struct ActivityParams {
    limit: usize,
    since_micros: Option<i64>,
    action_type: Option<String>,
    status: Option<String>,
    cursor: Option<(i64, String)>,
    range: Option<String>,
}

fn parse_query_params(tail: &str) -> Result<ActivityParams, (&'static str, String)> {
    let limit = match query_kv(tail, "limit") {
        Some(raw) => {
            let n: usize = raw
                .parse()
                .map_err(|_| bad_request(&format!("limit '{raw}' not a number")))?;
            if n == 0 || n > MAX_LIMIT {
                return Err(bad_request(&format!("limit must be 1..={MAX_LIMIT}")));
            }
            n
        }
        None => DEFAULT_LIMIT,
    };
    let range = query_kv(tail, "range").unwrap_or_else(|| DEFAULT_RANGE.to_string());
    let since_micros = range_to_since_micros(&range)
        .ok_or_else(|| bad_request(&format!("unknown range '{range}'")))?;
    let cursor = query_kv(tail, "cursor")
        .map(|c| decode_cursor(&c).ok_or_else(|| bad_request("invalid cursor")))
        .transpose()?;
    Ok(ActivityParams {
        limit,
        since_micros,
        action_type: query_kv(tail, "type").filter(|s| !s.is_empty()),
        status: query_kv(tail, "status").filter(|s| !s.is_empty()),
        cursor,
        range: Some(range),
    })
}

fn row_belongs_to_worker(row: &ExportedRow, worker_id: &str) -> bool {
    payload_str(&row.payload, "worker_id")
        .as_deref()
        .is_some_and(|w| w == worker_id)
}

fn row_passes_filters(row: &ExportedRow, params: &ActivityParams) -> bool {
    if let Some(t) = params.action_type.as_deref() {
        if action_type(row).as_deref() != Some(t) {
            return false;
        }
    }
    if let Some(s) = params.status.as_deref() {
        if outcome_status(&row.payload).as_deref() != Some(s) {
            return false;
        }
    }
    true
}

fn activity_row_json(
    row: &ExportedRow,
    worker_id: &str,
    worker_name: &str,
    policy: &RedactionPolicy,
) -> Value {
    let p = &row.payload;
    let (verb, _) = redact_text(
        &payload_str(p, "verb").unwrap_or_else(|| row.kind.clone()),
        policy,
    );
    let timestamp = DateTime::<Utc>::from_timestamp_micros(row.created_at_micros)
        .map_or_else(|| row.created_at_micros.to_string(), |dt| dt.to_rfc3339());
    let approver_json = payload_str(p, "approver_id").map_or(Value::Null, |id| {
        let raw = payload_str(p, "approver_name").unwrap_or_else(|| id.clone());
        let (name, _) = redact_text(&raw, policy);
        json!({ "id": id, "name": name })
    });
    let (classification, _) = redact_text(
        &classification(p).unwrap_or_else(|| "internal".into()),
        policy,
    );
    json!({
        "id": row.id,
        "timestamp": timestamp,
        "worker": { "id": worker_id, "name": worker_name },
        "action_type": action_type(row).unwrap_or_else(|| row.kind.clone()),
        "verb": verb,
        "status": outcome_status(p).unwrap_or_else(|| "succeeded".into()),
        "approver": approver_json,
        "channel": payload_str(p, "channel").unwrap_or_else(|| "web".into()),
        "cost": payload_cost_usd(p).map_or(Value::Null, |usd| json!({ "usd": usd })),
        "data_classification": classification,
    })
}

fn action_type(row: &ExportedRow) -> Option<String> {
    payload_str(&row.payload, "action_type")
        .or_else(|| payload_str(&row.payload, "endpoint"))
        .or_else(|| Some(row.kind.clone()))
}

fn outcome_status(p: &Value) -> Option<String> {
    payload_str(p, "outcome_status").or_else(|| payload_str(p, "status"))
}

fn classification(p: &Value) -> Option<String> {
    payload_str(p, "data_classification").or_else(|| payload_str(p, "classification"))
}

fn payload_str(p: &Value, key: &str) -> Option<String> {
    p.get(key).and_then(Value::as_str).map(str::to_string)
}

fn payload_cost_usd(p: &Value) -> Option<f64> {
    p.get("cost_usd").and_then(Value::as_f64)
}

fn past_cursor(row: &ExportedRow, cursor: Option<&(i64, String)>) -> bool {
    let Some((ts, id)) = cursor else {
        return true;
    };
    row.created_at_micros < *ts || (row.created_at_micros == *ts && row.id < *id)
}

fn encode_cursor(ts_micros: i64, id: &str) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(format!("{ts_micros}|{id}"))
}

fn decode_cursor(c: &str) -> Option<(i64, String)> {
    let bytes = general_purpose::URL_SAFE_NO_PAD.decode(c).ok()?;
    let s = String::from_utf8(bytes).ok()?;
    let (ts, id) = s.split_once('|')?;
    Some((ts.parse().ok()?, id.to_string()))
}

/// Symbolic range → UTC cutoff micros. `None` outer = unknown range
/// (caller surfaces 400); `Some(None)` would be "no cutoff" but every
/// supported preset yields a cutoff, so the inner Option always
/// flattens to `Some(micros)` on the happy path.
fn range_to_since_micros(range: &str) -> Option<Option<i64>> {
    let now = Utc::now();
    let day_start =
        |d: chrono::NaiveDate| d.and_hms_opt(0, 0, 0).map(|nd| nd.and_utc());
    let q_start = |y: i32, qi: u32| {
        chrono::NaiveDate::from_ymd_opt(y, qi * 3 + 1, 1).and_then(day_start)
    };
    let cutoff = match range {
        "1h" => Some(now - chrono::Duration::hours(1)),
        "today" => day_start(now.date_naive()),
        "yesterday" => now.date_naive().pred_opt().and_then(day_start),
        "7d" => Some(now - chrono::Duration::days(7)),
        "30d" => Some(now - chrono::Duration::days(30)),
        "qtd" => q_start(now.year(), now.date_naive().month0() / 3),
        "lq" => {
            let qi = now.date_naive().month0() / 3;
            let (y, q) = if qi == 0 {
                (now.year() - 1, 3)
            } else {
                (now.year(), qi - 1)
            };
            q_start(y, q)
        }
        _ => return None,
    };
    Some(cutoff.map(|dt| dt.timestamp_micros()))
}

fn query_kv(tail: &str, key: &str) -> Option<String> {
    let head = tail.split_once(' ').map(|(p, _)| p).unwrap_or(tail);
    let qs = head.split_once('?').map(|(_, q)| q)?;
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| url_decode(v))
    })
}

/// Minimal URL decoder; passes malformed input through unchanged.
/// Mirrors the sibling `routes_audit_feed` helper to avoid a new
/// dep for this one endpoint.
fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '+' => out.push(' '),
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(b) = u8::from_str_radix(&hex, 16) {
                    out.push(b as char);
                } else {
                    out.push('%');
                    out.push_str(&hex);
                }
            }
            other => out.push(other),
        }
    }
    out
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

async fn exists_in_other_tenant(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<bool> {
    let row: Option<String> = sqlx::query_scalar(
        "SELECT id FROM worker WHERE id = ? AND tenant_id != ? LIMIT 1",
    )
    .bind(worker_id.to_string())
    .bind(tenant.to_string())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("cross-tenant probe: {e}")))?;
    Ok(row.is_some())
}

async fn fetch_archived_at(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<Option<DateTime<Utc>>> {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT archived_at FROM worker \
         WHERE id = ? AND tenant_id = ? AND archived_at IS NOT NULL",
    )
    .bind(worker_id.to_string())
    .bind(tenant.to_string())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("archived_at lookup: {e}")))?;
    Ok(raw.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }))
}

async fn emit_view_audit(
    store: &Arc<SessionStore>,
    req: &str,
    worker_id: WorkerId,
    params: &ActivityParams,
    returned: usize,
) {
    let dims: Vec<&str> = [
        ("action_type", params.action_type.is_some()),
        ("status", params.status.is_some()),
        ("range", params.range.is_some()),
        ("cursor", params.cursor.is_some()),
    ]
    .into_iter()
    .filter_map(|(name, used)| used.then_some(name))
    .collect();
    let payload = json!({
        "endpoint": "worker.activity.viewed",
        "worker_id": worker_id.to_string(),
        "filter_dims_used": dims,
        "limit": params.limit,
        "returned": returned,
    });
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, principal_id_for_audit(req))
        .await
    {
        tracing::warn!(
            target: "gateway::worker_activity",
            error = %e,
            "worker.activity.viewed audit write failed"
        );
    }
}

fn gone_response(worker_id: WorkerId, archived_at: &DateTime<Utc>) -> (&'static str, String) {
    let recoverable = *archived_at + Duration::days(RECOVERABLE_DAYS);
    let body = json!({
        "error": {
            "code": "gone",
            "message": format!("worker {worker_id} is archived"),
        },
        "archived_at": archived_at.to_rfc3339(),
        "recoverable_until_at": recoverable.to_rfc3339(),
        "retry_after_seconds": (recoverable - Utc::now()).num_seconds().max(0),
    });
    ("410 Gone", body.to_string())
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn not_found(message: &str) -> (&'static str, String) {
    error_envelope("404 Not Found", "not_found", message)
}

fn forbidden(message: &str) -> (&'static str, String) {
    error_envelope("403 Forbidden", "forbidden", message)
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({ "error": { "code": code, "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_worker_activity_tests.rs"]
mod tests;
