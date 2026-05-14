//! `GET /api/v1/audit` — production audit feed (Stream J14).
//!
//! Cursor-paginated JSON list over `audit_record`: 7 filter dimensions
//! (worker / channel / action_type / status / approver / cost_band /
//! classification), `?q=` substring search, symbolic range presets
//! (1h / today / yesterday / 7d / 30d / qtd / lq + custom from/to).
//!
//! ## Filter path posture (post-v67 migration)
//!
//! The 7 dimensions are first-class columns on `audit_record` per
//! migration v67 (Stream `mission-enterprise-audit-full-text-search`).
//! The handler picks a path per request:
//!
//! - **Indexed SQL path** (preferred). When the migration is applied
//!   (detected via `PRAGMA table_info`), the handler builds a SQL
//!   `WHERE` that ANDs the dim filters directly, using
//!   `(col IN (?,…) OR col IS NULL)` per dim so legacy rows still
//!   surface. The app-layer pass then runs to enforce filters on
//!   any legacy NULL rows that snuck through.
//! - **App-layer fallback** (downgrade safety). When the column is
//!   absent (un-migrated DB), the existing `tail_rows_filtered`
//!   `created_at`-bounded fetch + payload_json filter pass runs
//!   unchanged. This is the source of correctness — if the SQL path
//!   and the app-layer path ever diverge, the app-layer is right.
//!
//! `?q=` substring search stays app-layer for v1 — SQLite `LIKE` is
//! fine on 10k rows; FTS5 promotion is closed by a follow-up brief.
//! Tenant scoping is per-workspace via the `tenant_id` denorm added by
//! Stream
//! `mission-enterprise-cross-tenant-audit`). The workspace tenant
//! resolves from `X-Argo-Tenant-Id` (set upstream from
//! `argo_config::workspace_tenant_id`); absent → `"ws-default"`.
//! Default reads UNION the migration-63 `__legacy__` bucket so
//! pre-migration rows stay visible. `?as_tenant=<other>` overrides
//! the workspace tenant ONLY when `X-Argo-Audit-Cross-Tenant: 1` is
//! set (a v1 seam — the dispatcher resolves
//! [`argo_capability::audit_view_cross_tenant_capability`] for the
//! principal and stamps the header; the handler trusts it). Without
//! the cap header the override is silently ignored — refusing with a
//! status code would leak the target tenant's existence. The legacy
//! bucket is never unioned into cross-tenant probes. PII fence: J2's
//! [`redact_text`] over a fixed set of string fields.

use super::routes_audit_feed_indexed::fetch_raw_rows;
use super::routes_audit_feed_scope::{resolve_tenant_scope, row_in_tenant_scope};
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink, ExportedRow};
use argo_redact::{redact_text, RedactionPolicy};
use argo_store::SessionStore;
use base64::{engine::general_purpose, Engine as _};
use chrono::Datelike;
use serde_json::{json, Value};
use std::sync::Arc;

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;
/// Raw-row ceiling per request before the in-memory filter pass —
/// bounds memory both on the indexed and the legacy app-layer
/// fetch paths.
pub(super) const RAW_FETCH_CAP: u32 = 5_000;

/// `GET /api/v1/audit` — see module docs.
pub(super) async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let params = match parse_query_params(tail) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let scope = resolve_tenant_scope(req, &params);
    let raw = match fetch_raw_rows(store, &params).await {
        Ok(rows) => rows,
        Err(e) => return server_error(&format!("audit fetch: {e}")),
    };
    let mut ordered = raw;
    // tail_rows_filtered returns ASC inside the LIMIT window — flip to DESC.
    ordered.sort_by(|a, b| {
        b.created_at_micros
            .cmp(&a.created_at_micros)
            .then_with(|| b.id.cmp(&a.id))
    });
    if let Some(to_micros) = params.to_micros {
        ordered.retain(|r| r.created_at_micros <= to_micros);
    }
    let policy = RedactionPolicy::default();
    let page: Vec<&ExportedRow> = ordered
        .iter()
        .filter(|row| row_in_tenant_scope(row, &scope))
        .filter(|row| row_passes_filters(row, &params))
        .filter(|row| past_cursor(row, params.cursor.as_ref()))
        .take(params.limit)
        .collect();
    let next_cursor = (page.len() == params.limit)
        .then(|| {
            page.last()
                .map(|r| encode_cursor(r.created_at_micros, &r.id))
        })
        .flatten();
    let items: Vec<Value> = page
        .iter()
        .map(|row| audit_row_json(row, &policy))
        .collect();
    emit_feed_audit(store, req, &params, items.len()).await;
    let mut body = json!({ "items": items });
    if let Some(c) = next_cursor {
        body["next_cursor"] = Value::String(c);
    }
    ("200 OK", body.to_string())
}

#[derive(Debug, Default)]
pub(super) struct FeedParams {
    pub(super) limit: usize,
    pub(super) since_micros: Option<i64>,
    pub(super) to_micros: Option<i64>,
    pub(super) q: Option<String>,
    pub(super) worker: Vec<String>,
    pub(super) channel: Vec<String>,
    pub(super) action_type: Vec<String>,
    pub(super) status: Vec<String>,
    pub(super) approver: Vec<String>,
    pub(super) cost_band: Vec<String>,
    pub(super) classification: Vec<String>,
    pub(super) cursor: Option<(i64, String)>,
    pub(super) range: Option<String>,
    /// `?as_tenant=<id>` cross-tenant override. Honored only when
    /// the request carries the `X-Argo-Audit-Cross-Tenant` header;
    /// otherwise silently ignored (the param is still parsed so the
    /// telemetry payload can report whether the caller attempted
    /// the override).
    pub(super) as_tenant: Option<String>,
}

fn parse_query_params(tail: &str) -> Result<FeedParams, (&'static str, String)> {
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
    let range = query_kv(tail, "range");
    let mut since_micros = match range.as_deref() {
        Some(r) => {
            range_to_since_micros(r).ok_or_else(|| bad_request(&format!("unknown range '{r}'")))?
        }
        None => None,
    };
    if let Some(from) = query_kv(tail, "from") {
        since_micros = Some(
            parse_iso_or_micros(&from)
                .ok_or_else(|| bad_request(&format!("invalid from '{from}'")))?,
        );
    }
    let to_micros = query_kv(tail, "to")
        .map(|to| {
            parse_iso_or_micros(&to).ok_or_else(|| bad_request(&format!("invalid to '{to}'")))
        })
        .transpose()?;
    let cursor = query_kv(tail, "cursor")
        .map(|c| decode_cursor(&c).ok_or_else(|| bad_request("invalid cursor")))
        .transpose()?;
    Ok(FeedParams {
        limit,
        since_micros,
        to_micros,
        q: query_kv(tail, "q").filter(|s| !s.is_empty()),
        worker: csv_param(tail, "worker"),
        channel: csv_param(tail, "channel"),
        action_type: csv_param(tail, "action_type"),
        status: csv_param(tail, "status"),
        approver: csv_param(tail, "approver"),
        cost_band: csv_param(tail, "cost_band"),
        classification: csv_param(tail, "classification"),
        cursor,
        range,
        as_tenant: query_kv(tail, "as_tenant").filter(|s| !s.is_empty()),
    })
}

fn row_passes_filters(row: &ExportedRow, params: &FeedParams) -> bool {
    let p = &row.payload;
    if !any_or(&params.worker, payload_str(p, "worker_id").as_deref())
        || !any_or(&params.channel, payload_str(p, "channel").as_deref())
        || !any_or(&params.action_type, action_type(row).as_deref())
        || !any_or(&params.status, outcome_status(p).as_deref())
        || !any_or(&params.approver, payload_str(p, "approver_id").as_deref())
        || !any_or(
            &params.cost_band,
            Some(cost_band_label(payload_cost_usd(p))),
        )
        || !any_or(&params.classification, classification(p).as_deref())
    {
        return false;
    }
    if let Some(q) = params.q.as_deref() {
        let q_lower = q.to_lowercase();
        let verb = payload_str(p, "verb").unwrap_or_default();
        let worker_name = payload_str(p, "worker_name").unwrap_or_default();
        let hits = verb.to_lowercase().contains(&q_lower)
            || worker_name.to_lowercase().contains(&q_lower)
            || row.id.to_lowercase().contains(&q_lower);
        if !hits {
            return false;
        }
    }
    true
}

fn audit_row_json(row: &ExportedRow, policy: &RedactionPolicy) -> Value {
    let p = &row.payload;
    let worker_id = payload_str(p, "worker_id").unwrap_or_default();
    let worker_name = payload_str(p, "worker_name").unwrap_or_else(|| {
        if worker_id.is_empty() {
            "unknown".into()
        } else {
            worker_id.clone()
        }
    });
    let (verb, _) = redact_text(
        &payload_str(p, "verb").unwrap_or_else(|| row.kind.clone()),
        policy,
    );
    let timestamp = chrono::DateTime::<chrono::Utc>::from_timestamp_micros(row.created_at_micros)
        .map_or_else(|| row.created_at_micros.to_string(), |dt| dt.to_rfc3339());
    let approver_json = payload_str(p, "approver_id").map_or(Value::Null, |id| {
        let raw = payload_str(p, "approver_name").unwrap_or_else(|| id.clone());
        let (name, _) = redact_text(&raw, policy);
        json!({"id": id, "name": name})
    });
    json!({
        "id": row.id,
        "timestamp": timestamp,
        "worker": { "id": worker_id, "name": worker_name },
        "action_type": action_type(row).unwrap_or_else(|| row.kind.clone()),
        "verb": verb,
        "status": outcome_status(p).unwrap_or_else(|| "succeeded".into()),
        "approver": approver_json,
        "channel": payload_str(p, "channel").unwrap_or_else(|| "web".into()),
        "cost": payload_cost_usd(p).map_or(Value::Null, |usd| json!({"usd": usd})),
        "data_classification": classification(p).unwrap_or_else(|| "internal".into()),
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

/// Maps a USD cost to the same string bands as the frontend E5-b
/// filter UI: `free` (None/0), `lt001` (<$0.01),
/// `from001to10` (≥$0.01, <$10), `gt10` (≥$10).
fn cost_band_label(cost: Option<f64>) -> &'static str {
    match cost {
        None => "free",
        Some(c) if c <= 0.0 => "free",
        Some(c) if c < 0.01 => "lt001",
        Some(c) if c < 10.0 => "from001to10",
        Some(_) => "gt10",
    }
}

fn any_or(allowed: &[String], candidate: Option<&str>) -> bool {
    allowed.is_empty() || candidate.is_some_and(|c| allowed.iter().any(|a| a == c))
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

fn range_to_since_micros(range: &str) -> Option<Option<i64>> {
    let now = chrono::Utc::now();
    let day_start = |d: chrono::NaiveDate| d.and_hms_opt(0, 0, 0).map(|nd| nd.and_utc());
    let q_start =
        |y: i32, qi: u32| chrono::NaiveDate::from_ymd_opt(y, qi * 3 + 1, 1).and_then(day_start);
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

fn parse_iso_or_micros(raw: &str) -> Option<i64> {
    if let Ok(micros) = raw.parse::<i64>() {
        return Some(micros);
    }
    let decoded = raw.replace("%3A", ":").replace('+', " ");
    chrono::DateTime::parse_from_rfc3339(&decoded)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc).timestamp_micros())
}

fn query_kv(tail: &str, key: &str) -> Option<String> {
    let head = tail.split_once(' ').map(|(p, _)| p).unwrap_or(tail);
    let qs = head.split_once('?').map(|(_, q)| q)?;
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| url_decode(v))
    })
}

fn csv_param(tail: &str, key: &str) -> Vec<String> {
    query_kv(tail, key)
        .map(|raw| {
            raw.split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Minimal `+`→space + `%xx`→byte URL decoder; passes malformed
/// input through unchanged. Avoids a new dep for this one endpoint.
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

async fn emit_feed_audit(
    store: &Arc<SessionStore>,
    req: &str,
    params: &FeedParams,
    returned: usize,
) {
    let dims: Vec<&str> = [
        ("worker", !params.worker.is_empty()),
        ("channel", !params.channel.is_empty()),
        ("action_type", !params.action_type.is_empty()),
        ("status", !params.status.is_empty()),
        ("approver", !params.approver.is_empty()),
        ("cost_band", !params.cost_band.is_empty()),
        ("classification", !params.classification.is_empty()),
        ("q", params.q.is_some()),
        ("range", params.range.is_some()),
        ("cursor", params.cursor.is_some()),
        ("as_tenant", params.as_tenant.is_some()),
    ]
    .into_iter()
    .filter_map(|(name, used)| used.then_some(name))
    .collect();
    let payload = json!({
        "endpoint": "audit.feed.queried",
        "filter_dims_used": dims,
        "limit": params.limit,
        "returned": returned,
    });
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            principal_id_for_audit(req),
        )
        .await
    {
        tracing::warn!(target: "gateway::audit_feed", error = %e, "audit feed telemetry write failed");
    }
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({"error": {"code": code, "message": message}}).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_audit_feed_tests.rs"]
mod tests;
