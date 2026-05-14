//! Per-worker retrospective endpoints (Stream `mission-retro-review-ui`,
//! MISSION_BACKLOG Section 6 #209).
//!
//! Five HTTP routes wired through the workspace SQLite store:
//!
//! - `POST   /api/v1/workers/<id>/retros` — create a retro over a
//!   time window (default: last 7 days). Aggregates audit metrics
//!   frozen at creation. Returns `201 { retro_id, metrics, sections, ... }`.
//! - `PATCH  /api/v1/workers/<id>/retros/<retro_id>` — overwrite
//!   `sections_json`. Returns 409 once the retro is finalised.
//! - `POST   /api/v1/workers/<id>/retros/<retro_id>/finalize` — lock
//!   the retro. Subsequent PATCH/finalize calls return 409.
//! - `GET    /api/v1/workers/<id>/retros` — list retros for one
//!   worker, newest first.
//! - `GET    /api/v1/retros[?q=...]` — index across the tenant; `q`
//!   is a case-insensitive substring search against the sections
//!   JSON column.
//!
//! Audit side-effects: each `create` emits an
//! `AuditKind::RetroCreated` row; `finalize` emits
//! `AuditKind::RetroFinalized`. These were originally recorded as
//! `AuditKind::HttpRequest` rows with a `payload.event_type` carrier
//! because `argo-audit` was outside the may-touch list of the
//! `mission-retro-review-ui` brief; the variants were promoted to
//! first-class kinds in `mission-audit-kind-runtime-events`. Legacy
//! rows on disk still carry `kind="http.request"` with
//! `payload.event_type="retro.created"`/`"retro.finalized"` and
//! remain chain-valid; audit-feed clients should accept either
//! representation during the transition.

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::WorkerId;
use argo_store::retro::{
    aggregate_metrics, metrics_template, NewRetro, RetroError, RetroRow, RetroStore,
    DEFAULT_WINDOW_DAYS,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

/// Hard cap on the index/list response size. Far above any realistic
/// per-tenant retro count for the foreseeable future; keeps the JSON
/// payload bounded if the table ever grows past expectations.
const LIST_LIMIT_MAX: i64 = 200;

/// `POST /api/v1/workers/<id>/retros` — create a retro covering
/// `[window_start, window_end]`. Body keys (all optional):
///
/// - `window_start_unix` / `window_end_unix` — bracket. Defaults to
///   `[now - DEFAULT_WINDOW_DAYS, now]` when either is missing.
///
/// Response shape: `201 { retro_id, worker_id, window_start, window_end,
/// metrics, sections, created_at }`.
pub(super) async fn create_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let worker_id = match worker_id_raw.parse::<WorkerId>() {
        Ok(w) => w,
        Err(_) => return bad_request("invalid worker id"),
    };
    let body = body_from_request(req);
    let (start_unix, end_unix) = match parse_window(body) {
        Ok(pair) => pair,
        Err(msg) => return bad_request(&msg),
    };
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let (tenant_id, principal_id) = match resolve_tenant_principal(store, req).await {
        Ok(p) => p,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };

    let worker_blob = worker_id.0.as_bytes().to_vec();
    let metrics = aggregate_metrics(store.pool(), &worker_blob, start_unix, end_unix)
        .await
        .unwrap_or_else(|_| metrics_template());
    let now = Utc::now().timestamp();
    let new_retro = NewRetro {
        tenant_id: tenant_id.0.as_bytes().to_vec(),
        worker_id: worker_blob,
        window_start: start_unix,
        window_end: end_unix,
        metrics_json: metrics.clone(),
        created_at: now,
    };
    let retro_id = match RetroStore::new(store.pool()).create(new_retro).await {
        Ok(id) => id,
        Err(e) => return server_error(&format!("create retro: {e}")),
    };
    emit_audit(
        store,
        &principal_id,
        AuditKind::RetroCreated,
        json!({
            "retro_id": retro_id.to_string(),
            "worker_id": worker_id.to_string(),
            "window_start": start_unix,
            "window_end": end_unix,
        }),
    )
    .await;

    let row = match RetroStore::new(store.pool()).get(retro_id).await {
        Ok(r) => r,
        Err(e) => return server_error(&format!("readback retro: {e}")),
    };
    ("201 Created", retro_to_json(&row).to_string())
}

/// `PATCH /api/v1/workers/<id>/retros/<retro_id>` — overwrite
/// `sections_json`. Body keys are the five template slots; unknown
/// keys are rejected so the operator catches typos.
pub(super) async fn update_response(
    _worker_id_raw: &str,
    retro_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let id = match Uuid::parse_str(retro_id_raw) {
        Ok(u) => u,
        Err(_) => return bad_request("invalid retro id"),
    };
    let body = body_from_request(req);
    let sections = match parse_sections(body) {
        Ok(v) => v,
        Err(msg) => return bad_request(&msg),
    };
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    match RetroStore::new(store.pool()).update_sections(id, &sections).await {
        Ok(()) => {}
        Err(RetroError::NotFound) => return not_found(retro_id_raw),
        Err(RetroError::Finalized) => return conflict("retro is finalized"),
        Err(RetroError::Backend(e)) => return server_error(&format!("update retro: {e}")),
    };
    let row = match RetroStore::new(store.pool()).get(id).await {
        Ok(r) => r,
        Err(e) => return server_error(&format!("readback retro: {e}")),
    };
    ("200 OK", retro_to_json(&row).to_string())
}

/// `POST /api/v1/workers/<id>/retros/<retro_id>/finalize` — lock the
/// retro. Idempotent-on-error: a second finalize returns 409 so the
/// operator notices the no-op.
pub(super) async fn finalize_response(
    _worker_id_raw: &str,
    retro_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let id = match Uuid::parse_str(retro_id_raw) {
        Ok(u) => u,
        Err(_) => return bad_request("invalid retro id"),
    };
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let now = Utc::now().timestamp();
    match RetroStore::new(store.pool()).finalize(id, now).await {
        Ok(()) => {}
        Err(RetroError::NotFound) => return not_found(retro_id_raw),
        Err(RetroError::Finalized) => return conflict("retro already finalized"),
        Err(RetroError::Backend(e)) => return server_error(&format!("finalize retro: {e}")),
    };
    let principal_id = principal_id_for_audit(req).unwrap_or_else(|| "system:retro".to_string());
    emit_audit(
        store,
        &principal_id,
        AuditKind::RetroFinalized,
        json!({
            "retro_id": id.to_string(),
            "finalized_at": now,
        }),
    )
    .await;
    let row = match RetroStore::new(store.pool()).get(id).await {
        Ok(r) => r,
        Err(e) => return server_error(&format!("readback retro: {e}")),
    };
    ("200 OK", retro_to_json(&row).to_string())
}

/// `GET /api/v1/workers/<id>/retros` — list retros for one worker,
/// newest first.
pub(super) async fn list_for_worker_response(
    worker_id_raw: &str,
    _req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let worker_id = match worker_id_raw.parse::<WorkerId>() {
        Ok(w) => w,
        Err(_) => return bad_request("invalid worker id"),
    };
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let worker_blob = worker_id.0.as_bytes().to_vec();
    let rows = match RetroStore::new(store.pool())
        .list_for_worker(&worker_blob, LIST_LIMIT_MAX)
        .await
    {
        Ok(r) => r,
        Err(e) => return server_error(&format!("list retros: {e}")),
    };
    let items: Vec<Value> = rows.iter().map(retro_to_json).collect();
    (
        "200 OK",
        json!({ "retros": items, "worker_id": worker_id.to_string() }).to_string(),
    )
}

/// `GET /api/v1/retros[?q=...]` — tenant-scoped retro index. The
/// optional `q` query param does a case-insensitive substring search
/// against the sections JSON; results are still newest-first.
pub(super) async fn index_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let q = query_param(req, "q");
    let (tenant_id, _principal_id) = match resolve_tenant_principal(store, req).await {
        Ok(p) => p,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let rows = match RetroStore::new(store.pool())
        .list_all(tenant_id.0.as_bytes(), q.as_deref(), LIST_LIMIT_MAX)
        .await
    {
        Ok(r) => r,
        Err(e) => return server_error(&format!("list retros: {e}")),
    };
    let items: Vec<Value> = rows.iter().map(retro_to_json).collect();
    (
        "200 OK",
        json!({ "retros": items, "q": q.unwrap_or_default() }).to_string(),
    )
}

fn parse_window(body: &str) -> Result<(i64, i64), String> {
    let now = Utc::now().timestamp();
    let default_start = now - DEFAULT_WINDOW_DAYS * 24 * 3600;
    if body.trim().is_empty() {
        return Ok((default_start, now));
    }
    let parsed: Value = serde_json::from_str(body).map_err(|e| format!("parse body: {e}"))?;
    let start = parsed
        .get("window_start_unix")
        .and_then(Value::as_i64)
        .unwrap_or(default_start);
    let end = parsed
        .get("window_end_unix")
        .and_then(Value::as_i64)
        .unwrap_or(now);
    if end < start {
        return Err("`window_end_unix` must be >= `window_start_unix`".to_string());
    }
    Ok((start, end))
}

fn parse_sections(body: &str) -> Result<Value, String> {
    let parsed: Value = serde_json::from_str(body).map_err(|e| format!("parse body: {e}"))?;
    let obj = parsed
        .as_object()
        .ok_or_else(|| "body must be a JSON object".to_string())?;

    // The wire shape is fixed: exactly the 5 template slots. Unknown
    // keys reject so typos surface immediately. Missing keys default
    // to empty string so partial updates still land.
    const ALLOWED: &[&str] = &[
        "surface_metrics",
        "what_went_well",
        "surprises",
        "what_to_change",
        "action_items",
    ];
    for key in obj.keys() {
        if !ALLOWED.contains(&key.as_str()) {
            return Err(format!(
                "unknown section '{key}'; allowed: {}",
                ALLOWED.join(", ")
            ));
        }
    }
    let mut out = serde_json::Map::new();
    for key in ALLOWED {
        let value = obj
            .get(*key)
            .cloned()
            .unwrap_or_else(|| Value::String(String::new()));
        if !value.is_string() {
            return Err(format!("section '{key}' must be a string"));
        }
        out.insert((*key).to_string(), value);
    }
    Ok(Value::Object(out))
}

async fn resolve_tenant_principal(
    store: &SessionStore,
    req: &str,
) -> ArgoResult<(TenantId, String)> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    let principal_id =
        principal_id_for_audit(req).unwrap_or_else(|| "system:retro".to_string());
    Ok((tenant, principal_id))
}

fn retro_to_json(row: &RetroRow) -> Value {
    json!({
        "retro_id": Uuid::from_slice(row.id.as_bytes()).unwrap_or_default().to_string(),
        "worker_id": worker_blob_to_uuid_string(&row.worker_id),
        "window_start": row.window_start,
        "window_end": row.window_end,
        "metrics": row.metrics_json,
        "sections": row.sections_json,
        "created_at": row.created_at,
        "finalized_at": row.finalized_at,
        "finalized": row.finalized_at.is_some(),
    })
}

fn worker_blob_to_uuid_string(blob: &[u8]) -> String {
    Uuid::from_slice(blob)
        .map(|u| u.to_string())
        .unwrap_or_default()
}

async fn emit_audit(
    store: &Arc<SessionStore>,
    principal_id: &str,
    kind: AuditKind,
    extras: Value,
) {
    let mut payload = json!({
        "endpoint": "workers.retros",
    });
    if let (Value::Object(ref mut head), Value::Object(rest)) = (&mut payload, extras) {
        for (k, v) in rest {
            head.insert(k, v);
        }
    }
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, kind, payload, Some(principal_id.to_string()))
        .await
    {
        tracing::warn!(
            target: "gateway::retros",
            error = %e,
            kind = kind.label(),
            "retro audit write failed"
        );
    }
}

fn query_param(req: &str, key: &str) -> Option<String> {
    let line = req.lines().next()?;
    let path = line.split_whitespace().nth(1)?;
    let qs = path.split_once('?').map(|(_, q)| q)?;
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| urldecode(v))
    })
}

fn urldecode(input: &str) -> String {
    // Minimal `%XX` / `+` decoder. Mirrors `routes_audit_feed`'s helper;
    // the gateway has no shared utility yet.
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push(((h * 16 + l) as u8) as char);
                    i += 3;
                } else {
                    out.push(bytes[i] as char);
                    i += 1;
                }
            }
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn not_found(id: &str) -> (&'static str, String) {
    error_envelope(
        "404 Not Found",
        "retro_not_found",
        &format!("no retro with id '{id}'"),
    )
}

fn conflict(message: &str) -> (&'static str, String) {
    error_envelope("409 Conflict", "retro_finalized", message)
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({
            "error": {
                "code": code,
                "message": message,
            }
        })
        .to_string(),
    )
}

#[cfg(test)]
#[path = "routes_retros_tests.rs"]
mod tests;
