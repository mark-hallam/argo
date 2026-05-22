//! `GET /api/v1/cohorts` + `GET /api/v1/cohorts/<id>` — fleet-wide
//! skill-cohort read surface (Stream `mission-skill-cohort-ui`,
//! MISSION_BACKLOG #198).
//!
//! Workspace-tenant-scoped. Bearer auth is gated upstream by
//! `routes::dispatch_route`. Cross-tenant `?tenant_id=` query honors
//! the same 403 rule as `routes_team_list`.
//!
//! ## Surface
//!
//! - `GET /api/v1/cohorts` →
//!   `{ "cohorts": [CohortSummary], "total_count": usize }`
//! - `GET /api/v1/cohorts/<cohort_id>` → flattened
//!   `CohortSummary` + `{ "members": [...], "skills": [...] }`
//!
//! Cohort identity is `"<vertical>::<trust_band>"`. See
//! `argo_store::cohort` for band definitions.
//!
//! ## Sensible deviations
//!
//! - The brief asks for `[{...}]` at the list level; we wrap in
//!   `{ "cohorts": [...], "total_count": n }` to match the existing
//!   `routes_team_list` convention (gateway response writers are
//!   fixed-shape).
//! - Detail flattens `CohortSummary` into the top-level JSON object
//!   per the brief's `{ ...summary, members, skills }` shape.

use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, WorkerId};
use argo_store::cohort::{cohort_detail, list_cohorts, CohortMember, CohortSummary};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::BTreeSet;
use std::sync::Arc;
use uuid::Uuid;

/// Body shape for `POST /api/v1/cohorts/<id>/bulk-assign-skill`.
#[derive(Debug, Deserialize)]
struct BulkAssignBody {
    worker_ids: Vec<String>,
    skill_id: String,
}

/// Default proficiency assigned to bulk-applied skills. Matches the
/// single-worker writer (`routes_worker_skills_helpers::DEFAULT_PROFICIENCY`).
const BULK_DEFAULT_PROFICIENCY: i64 = 3;

/// Cap on recent audit rows surfaced in the cohort detail envelope.
/// Brief calls for "3-5 most recent" — pick 5 as the upper bound.
const COHORT_RECENT_ACTIVITY_LIMIT: i64 = 5;

/// `GET /api/v1/cohorts`.
pub(super) async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let workspace_tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    match parse_tenant_query(req) {
        Ok(Some(req_t)) if req_t != workspace_tenant => {
            return forbidden("cross-tenant query not permitted");
        }
        Ok(_) => {}
        Err(msg) => return bad_request(&msg),
    }

    let cohorts: Vec<CohortSummary> = match list_cohorts(store.as_ref(), workspace_tenant).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("list cohorts: {e}")),
    };
    let total = cohorts.len();
    emit_list_audit(store, req, total, workspace_tenant).await;
    let body = json!({ "cohorts": cohorts, "total_count": total }).to_string();
    ("200 OK", body)
}

/// `GET /api/v1/cohorts/<cohort_id>`.
pub(super) async fn detail_response(
    req: &str,
    raw_cohort_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let workspace_tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    match parse_tenant_query(req) {
        Ok(Some(req_t)) if req_t != workspace_tenant => {
            return forbidden("cross-tenant query not permitted");
        }
        Ok(_) => {}
        Err(msg) => return bad_request(&msg),
    }
    let decoded = decode_path_segment(raw_cohort_id);
    if decoded.is_empty() {
        return bad_request("cohort_id required");
    }
    let detail = match cohort_detail(store.as_ref(), workspace_tenant, &decoded).await {
        Ok(Some(d)) => d,
        Ok(None) => return not_found(&format!("cohort '{decoded}' not found")),
        Err(e) => return server_error(&format!("cohort detail: {e}")),
    };
    let recent_activity =
        match recent_cohort_activity(store, workspace_tenant, &detail.members).await {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(
                    target: "gateway::cohorts",
                    error = %e,
                    cohort_id = %decoded,
                    "cohorts.detail recent_activity query failed; falling back to empty list"
                );
                Vec::new()
            }
        };
    emit_detail_audit(store, req, &decoded, workspace_tenant).await;
    let mut envelope = serde_json::to_value(&detail).unwrap_or_else(|_| json!({}));
    if let Value::Object(map) = &mut envelope {
        map.insert("recent_activity".into(), Value::Array(recent_activity));
    }
    ("200 OK", envelope.to_string())
}

/// Fetch the most recent `audit_record` rows tied to any of the cohort's
/// member worker ids, capped at [`COHORT_RECENT_ACTIVITY_LIMIT`]. Returns
/// an empty list (rather than erroring) when the cohort has no members,
/// so the front-end can render the "no members yet" empty state without
/// a 500.
///
/// Sort is `seq DESC` — the autoincrement column is the chain order and
/// is cheaper than the (TEXT-encoded) `created_at` column for ORDER BY.
async fn recent_cohort_activity(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    members: &[CohortMember],
) -> ArgoResult<Vec<Value>> {
    if members.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = members.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT seq, kind, action_type, worker_id, principal_id, channel, \
                outcome_status, created_at, payload_json \
         FROM audit_record \
         WHERE tenant_id = ? AND worker_id IN ({placeholders}) \
         ORDER BY seq DESC \
         LIMIT ?"
    );
    let mut query = sqlx::query(&sql).bind(tenant.to_string());
    for m in members {
        query = query.bind(m.worker_id.clone());
    }
    query = query.bind(COHORT_RECENT_ACTIVITY_LIMIT);
    let rows = query
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("cohort recent_activity: {e}")))?;
    let mut out: Vec<Value> = Vec::with_capacity(rows.len());
    for row in rows {
        let payload_raw: String = row.try_get("payload_json").unwrap_or_default();
        let payload: Value = serde_json::from_str(&payload_raw).unwrap_or(Value::Null);
        let summary = summarize_audit_payload(&payload);
        out.push(json!({
            "seq": row.try_get::<i64, _>("seq").unwrap_or(0),
            "kind": row.try_get::<String, _>("kind").unwrap_or_default(),
            "action_type": row.try_get::<Option<String>, _>("action_type").ok().flatten(),
            "worker_id": row.try_get::<Option<String>, _>("worker_id").ok().flatten(),
            "principal_id": row.try_get::<Option<String>, _>("principal_id").ok().flatten(),
            "channel": row.try_get::<Option<String>, _>("channel").ok().flatten(),
            "outcome_status": row.try_get::<Option<String>, _>("outcome_status").ok().flatten(),
            "created_at_ms": row.try_get::<i64, _>("created_at").unwrap_or(0),
            "summary": summary,
        }));
    }
    Ok(out)
}

/// Pull a short, human-readable phrase out of the audit payload. The
/// payload shape varies by kind; we look for the canonical keys
/// `summary`, then `endpoint`, then `message`, then fall back to an
/// empty string. The drawer renders this as the activity row's body.
fn summarize_audit_payload(payload: &Value) -> Option<String> {
    for key in &["summary", "endpoint", "message", "intent"] {
        if let Some(s) = payload.get(*key).and_then(Value::as_str) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

fn parse_tenant_query(req: &str) -> Result<Option<TenantId>, String> {
    // The request line is `<METHOD> <path>[?query] HTTP/1.1`. Skip the
    // method (anything before the first space) so this parser works for
    // GET and POST (mission-cohort-detail-bulk-assign-skill needs POST
    // tenant-query enforcement too — the original GET-only strip_prefix
    // dropped POST `?tenant_id=` overrides through, surfacing a 200
    // instead of the required 403).
    let tail = req
        .split_once(' ')
        .map(|(_, rest)| rest)
        .unwrap_or(req);
    let head = tail.split_once(' ').map_or(tail, |(p, _)| p);
    let Some(qs) = head.split_once('?').map(|(_, q)| q) else {
        return Ok(None);
    };
    let raw = qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == "tenant_id").then(|| v.to_string())
    });
    let Some(raw) = raw else { return Ok(None) };
    Uuid::parse_str(&raw)
        .map(|u| Some(TenantId(u)))
        .map_err(|e| format!("invalid tenant_id '{raw}': {e}"))
}

/// Strip the trailing ` HTTP/1.1` and any `?query` from the path
/// segment, then percent-decode `%3A`-style `:` separators if a client
/// has encoded them. v1 implementation is permissive — the cohort id
/// is opaque to the route layer; the store layer validates the format.
fn decode_path_segment(raw: &str) -> String {
    let trimmed = raw
        .split_whitespace()
        .next()
        .unwrap_or(raw)
        .split('?')
        .next()
        .unwrap_or(raw);
    let mut out = String::with_capacity(trimmed.len());
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(s) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(b) = u8::from_str_radix(s, 16) {
                    out.push(b as char);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

async fn emit_list_audit(store: &Arc<SessionStore>, req: &str, count: usize, tenant: TenantId) {
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let payload = json!({
        "endpoint": "cohorts.list.queried",
        "actor": actor,
        "tenant_id": tenant.to_string(),
        "count": count,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, Some(actor.clone()))
        .await
    {
        tracing::warn!(
            target: "gateway::cohorts",
            error = %e,
            actor = %actor,
            "cohorts.list.queried audit write failed"
        );
    }
}

async fn emit_detail_audit(
    store: &Arc<SessionStore>,
    req: &str,
    cohort_id: &str,
    tenant: TenantId,
) {
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let payload = json!({
        "endpoint": "cohorts.detail.queried",
        "actor": actor,
        "tenant_id": tenant.to_string(),
        "cohort_id": cohort_id,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, Some(actor.clone()))
        .await
    {
        tracing::warn!(
            target: "gateway::cohorts",
            error = %e,
            actor = %actor,
            "cohorts.detail.queried audit write failed"
        );
    }
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn forbidden(message: &str) -> (&'static str, String) {
    error_envelope("403 Forbidden", "forbidden", message)
}

fn not_found(message: &str) -> (&'static str, String) {
    error_envelope("404 Not Found", "not_found", message)
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

/// `POST /api/v1/cohorts/<id>/bulk-assign-skill` — apply one skill to
/// every selected worker in a cohort. Body shape:
/// `{ "worker_ids": [string], "skill_id": "<uuid-or-name>" }`. Tenant
/// scope: cohort must belong to the workspace tenant; every
/// `worker_id` must be a current member of the cohort. Idempotent —
/// workers already holding the skill count in `already_had` rather
/// than failing the call.
pub(super) async fn bulk_assign_skill_response(
    req: &str,
    raw_cohort_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    if let Err(msg) = enforce_same_tenant_query(req, tenant) {
        return msg;
    }
    let decoded = decode_path_segment(raw_cohort_id);
    if decoded.is_empty() {
        return bad_request("cohort_id required");
    }
    let body: BulkAssignBody = match parse_bulk_body(req) {
        Ok(b) => b,
        Err(r) => return r,
    };
    if body.worker_ids.is_empty() {
        return bad_request("worker_ids must not be empty");
    }
    if body.skill_id.trim().is_empty() {
        return bad_request("skill_id must not be empty");
    }
    let detail = match cohort_detail(store.as_ref(), tenant, &decoded).await {
        Ok(Some(d)) => d,
        Ok(None) => return not_found(&format!("cohort '{decoded}' not found")),
        Err(e) => return server_error(&format!("cohort detail: {e}")),
    };
    let member_set: BTreeSet<&str> =
        detail.members.iter().map(|m| m.worker_id.as_str()).collect();
    let invalid: Vec<&str> = body
        .worker_ids
        .iter()
        .filter(|id| !member_set.contains(id.as_str()))
        .map(String::as_str)
        .collect();
    if !invalid.is_empty() {
        return bad_request(&format!(
            "worker_ids not in cohort: {}",
            invalid.join(", ")
        ));
    }
    let parsed_ids: Vec<WorkerId> = match parse_worker_ids(&body.worker_ids) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let identity = IdentityStore::new(store);
    let outcome = match identity
        .bulk_assign_skill_to_workers(&parsed_ids, &body.skill_id, BULK_DEFAULT_PROFICIENCY)
        .await
    {
        Ok(o) => o,
        Err(e) => return server_error(&format!("bulk_assign_skill: {e}")),
    };
    emit_bulk_assign_audits(store, req, tenant, &decoded, &body, &parsed_ids).await;
    let response = json!({
        "assigned": outcome.assigned,
        "already_had": outcome.already_had,
        "errors": Vec::<Value>::new(),
    });
    ("200 OK", response.to_string())
}

/// Block cross-tenant `?tenant_id=` overrides on the POST surface,
/// matching the GET behavior (caller's session tenant is the only
/// scope; an explicit different tenant_id is a 403).
fn enforce_same_tenant_query(req: &str, tenant: TenantId) -> Result<(), (&'static str, String)> {
    match parse_tenant_query(req) {
        Ok(Some(req_t)) if req_t != tenant => Err(forbidden("cross-tenant query not permitted")),
        Ok(_) => Ok(()),
        Err(msg) => Err(bad_request(&msg)),
    }
}

fn parse_bulk_body(req: &str) -> Result<BulkAssignBody, (&'static str, String)> {
    let raw = super::body_from_request(req);
    serde_json::from_str(raw).map_err(|e| bad_request(&format!("parse JSON body: {e}")))
}

fn parse_worker_ids(raw: &[String]) -> Result<Vec<WorkerId>, (&'static str, String)> {
    let mut out = Vec::with_capacity(raw.len());
    for id in raw {
        let parsed = id
            .parse::<WorkerId>()
            .map_err(|e| bad_request(&format!("invalid worker_id '{id}': {e}")))?;
        out.push(parsed);
    }
    Ok(out)
}

/// Write one `HttpRequest+endpoint=skill_assigned` row per worker so
/// the cohort drawer's activity feed surfaces the bulk action as one
/// row per affected member (per the brief acceptance).
async fn emit_bulk_assign_audits(
    store: &Arc<SessionStore>,
    req: &str,
    tenant: TenantId,
    cohort_id: &str,
    body: &BulkAssignBody,
    worker_ids: &[WorkerId],
) {
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    for worker_id in worker_ids {
        let payload = json!({
            "endpoint": "skill_assigned",
            "actor": actor,
            "tenant_id": tenant.to_string(),
            "cohort_id": cohort_id,
            "worker_id": worker_id.to_string(),
            "skill_id": body.skill_id,
            "source": "cohort.bulk_assign",
        });
        if let Err(e) = sink
            .write(None, AuditKind::HttpRequest, payload, Some(actor.clone()))
            .await
        {
            tracing::warn!(
                target: "gateway::cohorts",
                error = %e,
                worker_id = %worker_id,
                "cohorts.bulk_assign audit write failed"
            );
        }
    }
}

#[cfg(test)]
#[path = "routes_cohorts_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "routes_cohorts_bulk_tests.rs"]
mod bulk_tests;
