//! GET + PATCH halves of the J11 worker-settings surface.
//!
//! Read path: bundles the four facets (channels, quiet hours, budget,
//! autonomy) plus the operator-only `advanced` block. PATCH handlers
//! validate the body, emit a `worker.<facet>.changed` audit row, and
//! echo the new shape back to the caller. See the parent module for
//! v1-posture rationale.

use super::super::routes_worker_detail::PrincipalRole;
use super::helpers::{
    bad_request, emit_audit, enforce_archive_410, load_worker_by_id, parse_body,
    parse_role_header, parse_worker_id, preflight, require_string, resolve_tenant,
    server_error, store_unavail,
};
use argo_identity::{NotifyChannelRef, Worker, WorkerId};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

pub(crate) async fn get_settings(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else { return store_unavail() };
    let worker_id = match parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    let tenant = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let worker = match load_worker_by_id(store, tenant, worker_id).await {
        Ok(w) => w,
        Err(resp) => return resp,
    };
    if let Some(resp) = enforce_archive_410(store, tenant, worker_id).await {
        return resp;
    }
    let role = parse_role_header(req).unwrap_or(PrincipalRole::Delegator);
    let body = build_settings_payload(store, &worker, role).await;
    emit_audit(store, req, &worker, "worker.settings.viewed", json!({})).await;
    ("200 OK", body.to_string())
}

pub(crate) async fn patch_channels(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else { return store_unavail() };
    let (_, worker) = match preflight(worker_id_raw, store, true).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let body: Value = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let all: Vec<String> = match body.get("channels").and_then(Value::as_array) {
        Some(arr) => arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        None => return bad_request("missing 'channels' array"),
    };
    let primary = match body.get("primary").and_then(Value::as_str) {
        Some(p) if !p.is_empty() => p.to_string(),
        _ => return bad_request("missing 'primary' string"),
    };
    if !all.iter().any(|c| c == &primary) {
        return bad_request("'primary' must be present in 'channels'");
    }
    let payload = json!({ "channels": all, "primary": primary });
    emit_audit(store, req, &worker, "worker.channels.changed", payload.clone()).await;
    ("200 OK", json!({ "channels": payload }).to_string())
}

pub(crate) async fn patch_quiet_hours(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else { return store_unavail() };
    let (_, worker) = match preflight(worker_id_raw, store, true).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let body: Value = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let start = require_string(&body, "start").map(str::to_string);
    let end = require_string(&body, "end").map(str::to_string);
    let tz = require_string(&body, "tz").map(str::to_string);
    let (start, end, tz) = match (start, end, tz) {
        (Some(s), Some(e), Some(t)) => (s, e, t),
        _ => return bad_request("body must include 'start', 'end', 'tz'"),
    };
    let payload = json!({ "start": start, "end": end, "tz": tz });
    emit_audit(store, req, &worker, "worker.quiet_hours.changed", payload.clone()).await;
    ("200 OK", json!({ "quiet_hours": payload }).to_string())
}

pub(crate) async fn patch_budget(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else { return store_unavail() };
    let (_, worker) = match preflight(worker_id_raw, store, true).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let body: Value = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let daily = body.get("daily_usd").and_then(Value::as_f64);
    let monthly = body.get("monthly_usd").and_then(Value::as_f64);
    let (daily, monthly) = match (daily, monthly) {
        (Some(d), Some(m)) if d >= 0.0 && m >= 0.0 => (d, m),
        _ => return bad_request("body must include non-negative 'daily_usd' and 'monthly_usd'"),
    };
    let payload = json!({ "daily_usd": daily, "monthly_usd": monthly });
    emit_audit(store, req, &worker, "worker.budget.changed", payload.clone()).await;
    ("200 OK", json!({ "budget": payload }).to_string())
}

pub(crate) async fn patch_autonomy(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else { return store_unavail() };
    let (_, worker) = match preflight(worker_id_raw, store, true).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let body: Value = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let level = match body.get("level").and_then(Value::as_str) {
        Some(l) if matches!(l, "conservative" | "balanced" | "aggressive") => l.to_string(),
        _ => return bad_request("'level' must be conservative | balanced | aggressive"),
    };
    let payload = json!({ "level": level });
    emit_audit(store, req, &worker, "worker.autonomy.changed", payload.clone()).await;
    if level == "conservative" {
        emit_audit(
            store,
            req,
            &worker,
            "worker.trust_ratchet.reset",
            json!({ "reason": "autonomy_set_conservative" }),
        )
        .await;
    }
    ("200 OK", json!({ "autonomy": payload }).to_string())
}

async fn build_settings_payload(
    store: &Arc<SessionStore>,
    worker: &Worker,
    role: PrincipalRole,
) -> Value {
    let (primary, all) = derive_channels(store, worker).await;
    let quiet = latest_setting(store, &worker.id, "worker.quiet_hours.changed").await;
    let budget = latest_setting(store, &worker.id, "worker.budget.changed").await;
    let autonomy = latest_setting(store, &worker.id, "worker.autonomy.changed").await;
    let advanced = (role == PrincipalRole::Operator).then(|| {
        json!({
            "namespace_id": worker.memory_namespace.worker_id.to_string(),
            "worker_id_full": worker.id.to_string(),
            "manifest_url": format!("/api/v1/workers/{}/manifest", worker.id),
        })
    });
    json!({
        "channels": { "primary": primary, "all": all },
        "quiet_hours": {
            "start": quiet.get("start").and_then(Value::as_str).unwrap_or("22:00"),
            "end":   quiet.get("end").and_then(Value::as_str).unwrap_or("07:00"),
            "tz":    quiet.get("tz").and_then(Value::as_str).unwrap_or("UTC"),
        },
        "budget": {
            "daily_usd":      budget.get("daily_usd").and_then(Value::as_f64).unwrap_or(5.0),
            "monthly_usd":    budget.get("monthly_usd").and_then(Value::as_f64).unwrap_or(100.0),
            "used_today_usd": 0.0,
            "used_month_usd": 0.0,
        },
        "autonomy": {
            "level": autonomy.get("level").and_then(Value::as_str).unwrap_or("balanced"),
        },
        "advanced": advanced,
    })
}

async fn derive_channels(store: &Arc<SessionStore>, worker: &Worker) -> (String, Vec<String>) {
    let from_audit = latest_setting(store, &worker.id, "worker.channels.changed").await;
    if let (Some(primary), Some(arr)) = (
        from_audit.get("primary").and_then(Value::as_str),
        from_audit.get("channels").and_then(Value::as_array),
    ) {
        let all: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect();
        return (primary.to_string(), all);
    }
    let all: Vec<String> = worker
        .channels
        .iter()
        .map(|c: &NotifyChannelRef| c.channel.clone())
        .collect();
    let primary = all.first().cloned().unwrap_or_else(|| "web".to_string());
    (primary, all)
}

async fn latest_setting(store: &Arc<SessionStore>, worker_id: &WorkerId, endpoint: &str) -> Value {
    let needle = format!("%\"endpoint\":\"{endpoint}\"%");
    let worker_needle = format!("%\"worker_id\":\"{worker_id}\"%");
    let row: Option<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE payload_json LIKE ? AND payload_json LIKE ? \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&needle)
    .bind(&worker_needle)
    .fetch_optional(store.pool())
    .await
    .ok()
    .flatten();
    row.and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or(Value::Null)
}
