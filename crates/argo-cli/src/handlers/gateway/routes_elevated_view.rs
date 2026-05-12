//! Elevated-view request + grant endpoints (Stream J16).
//!
//! Three HTTP routes that manage time-bounded, per-category overrides
//! of the J2 server-side redact fence:
//!
//! - `POST /api/v1/audit/elevated-view-request` — request an unmask
//!   for the viewer's principal across one-or-more categories. v1
//!   auto-grants; `mission-enterprise-elevated-view-approval` adds
//!   the operator-approval step.
//! - `GET  /api/v1/audit/elevated-view-grants[?active=true|false]`
//!   — viewer's grant list (defaults to active-only).
//! - `DELETE /api/v1/audit/elevated-view-grants/<id>` — self-revoke
//!   or operator-revoke. v1 treats every authenticated caller as an
//!   operator; RBAC granularity lands in a follow-up brief.
//!
//! Bearer auth runs upstream in `dispatch_post`; cross-tenant lookups
//! 403 inside the GET/DELETE arms.

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_store::elevated_view_grant::{
    ElevatedViewGrantRow, ElevatedViewGrantStore, NewElevatedViewGrant,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

/// Reason length floor for `POST /elevated-view-request`. Mirrors the
/// frontend dialog's helper-text minimum so the audit trail carries a
/// meaningful justification.
const REASON_MIN_LEN: usize = 20;

/// Duration cap (`duration_hours <= 24`). Per-tenant policy lands in
/// `mission-enterprise-elevated-view-duration-policy`.
const DURATION_HOURS_MAX: u32 = 24;

/// Wire labels accepted by the request endpoint. Superset of the
/// `argo_redact::Category` enum so the frontend can request
/// forward-compat categories alongside the v1 5-category PII set.
const ALLOWED_CATEGORIES: &[&str] = &[
    "email",
    "phone",
    "card",
    "iban",
    "ird",
    "classification",
    "approver",
    "counterparty",
];

/// `POST /api/v1/audit/elevated-view-request` —
/// body `{ reason, categories, duration_hours }`,
/// response `201 { grant_id, expires_at, auto_granted, note }`.
pub(super) async fn request_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let body = body_from_request(req);
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let reason = match parsed.get("reason").and_then(Value::as_str) {
        Some(s) if s.trim().len() >= REASON_MIN_LEN => s.trim().to_string(),
        _ => {
            return bad_request(&format!(
                "`reason` (string ≥ {REASON_MIN_LEN} chars) is required"
            ));
        }
    };
    let categories = match parse_categories(parsed.get("categories")) {
        Ok(v) => v,
        Err(msg) => return bad_request(&msg),
    };
    let duration_hours = match parse_duration_hours(parsed.get("duration_hours")) {
        Ok(n) => n,
        Err(msg) => return bad_request(&msg),
    };
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let (tenant_id, principal_id) = match resolve_tenant_principal(store, req).await {
        Ok(pair) => pair,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };

    let now = Utc::now();
    let expires_at = now + chrono::Duration::hours(i64::from(duration_hours));
    let new_grant = NewElevatedViewGrant {
        tenant_id,
        principal_id: principal_id.clone(),
        categories: categories.clone(),
        reason: reason.clone(),
        granted_at: now,
        expires_at,
    };
    let grant_id = match ElevatedViewGrantStore::new(store.pool()).insert(new_grant).await {
        Ok(id) => id,
        Err(e) => return server_error(&format!("insert grant: {e}")),
    };
    emit_audit(
        store,
        &principal_id,
        "elevated_view.requested",
        json!({
            "actor": principal_id,
            "categories": categories,
            "reason": reason,
            "duration_hours": duration_hours,
        }),
    )
    .await;
    emit_audit(
        store,
        &principal_id,
        "elevated_view.granted",
        json!({
            "grant_id": grant_id.to_string(),
            "categories": categories,
            "expires_at": expires_at.to_rfc3339(),
            "auto_granted": true,
        }),
    )
    .await;
    (
        "201 Created",
        json!({
            "grant_id": grant_id.to_string(),
            "expires_at": expires_at.to_rfc3339(),
            "auto_granted": true,
            "note": "v1 auto-grants elevated-view requests; v1.5 will require operator approval.",
        })
        .to_string(),
    )
}

/// `GET /api/v1/audit/elevated-view-grants[?active=true|false]` —
/// caller's grants. Default active-only. The query is keyed by the
/// caller's principal id, so cross-principal reads are structurally
/// blocked at the storage boundary.
pub(super) async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let active_only = match query_param(req, "active") {
        Some(v) => parse_bool_param(&v).unwrap_or(true),
        None => true,
    };
    let principal_id = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let rows = match ElevatedViewGrantStore::new(store.pool())
        .list_for_principal(&principal_id, active_only, Utc::now())
        .await
    {
        Ok(v) => v,
        Err(e) => return server_error(&format!("list grants: {e}")),
    };
    let items: Vec<Value> = rows.iter().map(grant_to_json).collect();
    (
        "200 OK",
        json!({ "grants": items, "active_only": active_only }).to_string(),
    )
}

/// `DELETE /api/v1/audit/elevated-view-grants/<id>` — revoke one
/// grant. 204 on success (idempotent on already-revoked); 403 on a
/// cross-tenant grant.
pub(super) async fn revoke_response(
    grant_id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let id = match Uuid::parse_str(grant_id) {
        Ok(u) => u,
        Err(_) => return bad_request("invalid grant id"),
    };
    let (caller_tenant, caller_principal) = match resolve_tenant_principal(store, req).await {
        Ok(pair) => pair,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let grant_store = ElevatedViewGrantStore::new(store.pool());
    let row = match grant_store.get(id).await {
        Ok(Some(r)) => r,
        Ok(None) => return not_found(grant_id),
        Err(e) => return server_error(&format!("get grant: {e}")),
    };
    if row.tenant_id != caller_tenant {
        return forbidden("grant belongs to a different tenant");
    }
    match grant_store.revoke(id, Utc::now()).await {
        Ok(_) => {}
        Err(e) => return server_error(&format!("revoke: {e}")),
    }
    emit_audit(
        store,
        &caller_principal,
        "elevated_view.revoked",
        json!({
            "grant_id": id.to_string(),
            "revoker_id": caller_principal,
            "self_revoke": caller_principal == row.principal_id,
        }),
    )
    .await;
    ("204 No Content", String::new())
}

fn parse_categories(value: Option<&Value>) -> Result<Vec<String>, String> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Err("`categories` (non-empty array of strings) is required".to_string());
    };
    if arr.is_empty() {
        return Err("`categories` must be non-empty".to_string());
    }
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        let Some(s) = v.as_str() else {
            return Err("`categories` entries must be strings".to_string());
        };
        if !ALLOWED_CATEGORIES.contains(&s) {
            return Err(format!(
                "unknown category '{s}'; allowed: {}",
                ALLOWED_CATEGORIES.join(", ")
            ));
        }
        // De-dupe while preserving first-seen order.
        if !out.iter().any(|existing: &String| existing == s) {
            out.push(s.to_string());
        }
    }
    Ok(out)
}

fn parse_duration_hours(value: Option<&Value>) -> Result<u32, String> {
    let Some(n) = value.and_then(Value::as_u64) else {
        return Err(format!(
            "`duration_hours` (integer in [1, {DURATION_HOURS_MAX}]) is required"
        ));
    };
    let cast = u32::try_from(n).map_err(|_| {
        format!("`duration_hours` must be in [1, {DURATION_HOURS_MAX}]")
    })?;
    if !(1..=DURATION_HOURS_MAX).contains(&cast) {
        return Err(format!(
            "`duration_hours` must be in [1, {DURATION_HOURS_MAX}]"
        ));
    }
    Ok(cast)
}

async fn resolve_tenant_principal(
    store: &SessionStore,
    req: &str,
) -> ArgoResult<(TenantId, String)> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    let principal_id =
        principal_id_for_audit(req).unwrap_or_else(|| "system:elevated-view".to_string());
    Ok((tenant, principal_id))
}

fn grant_to_json(row: &ElevatedViewGrantRow) -> Value {
    json!({
        "grant_id": row.id.to_string(),
        "principal_id": row.principal_id,
        "categories": row.categories,
        "reason": row.reason,
        "granted_at": row.granted_at.to_rfc3339(),
        "expires_at": row.expires_at.to_rfc3339(),
        "revoked_at": row.revoked_at.map(|t| t.to_rfc3339()),
        "active": row.revoked_at.is_none() && row.expires_at > Utc::now(),
    })
}

async fn emit_audit(
    store: &Arc<SessionStore>,
    principal_id: &str,
    event_type: &str,
    extras: Value,
) {
    let mut payload = json!({
        "endpoint": "audit.elevated_view",
        "event_type": event_type,
    });
    if let (Value::Object(ref mut head), Value::Object(rest)) = (&mut payload, extras) {
        for (k, v) in rest {
            head.insert(k, v);
        }
    }
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            Some(principal_id.to_string()),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::elevated_view",
            error = %e,
            event_type = event_type,
            "elevated-view audit write failed"
        );
    }
}

fn query_param(req: &str, key: &str) -> Option<String> {
    let line = req.lines().next()?;
    let path = line.split_whitespace().nth(1)?;
    let qs = path.split_once('?').map(|(_, q)| q)?;
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

fn parse_bool_param(raw: &str) -> Option<bool> {
    match raw {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn forbidden(message: &str) -> (&'static str, String) {
    error_envelope("403 Forbidden", "forbidden", message)
}

fn not_found(id: &str) -> (&'static str, String) {
    error_envelope(
        "404 Not Found",
        "grant_not_found",
        &format!("no grant with id '{id}'"),
    )
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
#[path = "routes_elevated_view_tests.rs"]
mod tests;
