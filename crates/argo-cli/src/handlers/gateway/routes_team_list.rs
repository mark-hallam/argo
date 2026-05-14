//! `GET /api/v1/team` — tenant-scoped Team-page roster (Stream J6).
//!
//! Returns workers in the workspace tenant enriched with the E3-c
//! headline (`argo_team_headline::generate_team_headline`), a derived
//! runtime status, and (when an org chart is loaded) the worker's
//! department + team binding via `argo_identity::OrgStore`. Bearer-auth
//! is gated upstream by `routes::dispatch_route`. v1 hardcoded RBAC,
//! pagination, and cache postures are tracked by their respective
//! `mission-enterprise-team-*` follow-up briefs.
//!
//! ## Sensible deviations from the brief (rationale in the PR body):
//! - `total_count` lives on the JSON body, not an `x-total-count`
//!   response header — the gateway response writer in `boot.rs` is
//!   fixed-shape and outside this brief's may-touch list.
//! - Audit rows use `AuditKind::HttpRequest` with `endpoint:
//!   "team.list.queried"` payload (matches the J5 `workers.hire`
//!   convention; adding an enum variant is workspace-wide).
//! - `status` is derived: circuit Open → `blocked`, HalfOpen →
//!   `paused`, pending-approval > 0 → `waiting`, fresh activity →
//!   `working`, else `idle` (no `worker.status` column exists).
//! - Cross-tenant scoping uses `?tenant_id=`: if supplied and not
//!   equal to the workspace tenant, the handler returns 403.

use super::routes_auth_principal::principal_id_for_audit;
use crate::handlers::worker_actions::ActionState;
use crate::handlers::worker_actions_query::collect_latest_actions;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{CircuitState, IdentityStore, OrgStore, Worker, WorkerId};
use argo_store::SessionStore;
use argo_team_headline::{generate_team_headline, TeamHeadline};
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Hard cap on workers returned in a single response. Beyond this the
/// `total_count` body field signals truncation (pagination follow-up).
pub(super) const MAX_WORKERS: usize = 200;
const WORKING_FRESHNESS_SECS: i64 = 5 * 60;

type OrgRefs = (Option<Value>, Option<Value>);

/// `GET /api/v1/team` entry point.
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
    let identities = IdentityStore::new(store);
    let workers = match identities.list_workers(workspace_tenant, false).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("list workers: {e}")),
    };
    let total = workers.len();
    let truncated: Vec<Worker> = workers.into_iter().take(MAX_WORKERS).collect();

    let pending_by_worker = pending_approvals_by_worker().await;
    let org_index = build_org_index(store, workspace_tenant).await.unwrap_or_else(|e| {
        tracing::warn!(target: "gateway::team_list", error = %e, "org index unavailable");
        HashMap::new()
    });
    let now = Utc::now();

    let mut rows: Vec<(u8, Option<DateTime<Utc>>, Value)> = Vec::with_capacity(truncated.len());
    for w in &truncated {
        let pending = pending_by_worker.get(&w.id).copied().unwrap_or(0);
        let (priority, status_label) = derive_status(w, pending, now);
        let headline = generate_team_headline(w.id, &w.vertical_id, store)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    target: "gateway::team_list",
                    worker_id = %w.id,
                    error = %e,
                    "headline generation failed; falling back to empty"
                );
                TeamHeadline { bullets: vec!["No activity yet.".to_string()], tomorrow: None }
            });
        let summary = build_summary(w, status_label, pending, &headline, &org_index);
        rows.push((priority, w.last_used_at, summary));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
    let summaries: Vec<Value> = rows.into_iter().map(|(_, _, v)| v).collect();

    emit_audit(store, req, summaries.len(), workspace_tenant).await;

    let body = json!({ "workers": summaries, "total_count": total }).to_string();
    ("200 OK", body)
}

fn build_summary(
    w: &Worker,
    status_label: &'static str,
    pending: u32,
    headline: &TeamHeadline,
    org: &HashMap<WorkerId, OrgRefs>,
) -> Value {
    let avatar = w.persona.avatar.clone().filter(|s| !s.is_empty());
    let tagline = w
        .persona
        .bio
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| Some(w.role.charter.clone()).filter(|s| !s.is_empty()))
        .unwrap_or_else(|| w.persona.name.clone());
    let (department, team) = org.get(&w.id).cloned().unwrap_or((None, None));
    json!({
        "id": w.id.to_string(),
        "name": w.persona.name,
        "avatar_url": avatar,
        "role": { "tagline": tagline, "vertical": w.vertical_id },
        "status": status_label,
        "department": department,
        "team": team,
        "headline": { "bullets": headline.bullets, "tomorrow": headline.tomorrow },
        "last_active_at": w.last_used_at.map(|ts| ts.to_rfc3339()),
        "pending_approval_count": pending,
    })
}

/// Returns `(sort_priority, wire_label)`. Lower priority sorts first:
/// waiting=0, working=1, idle=2, paused=3, blocked=4.
pub(super) fn derive_status(
    w: &Worker,
    pending: u32,
    now: DateTime<Utc>,
) -> (u8, &'static str) {
    match &w.circuit_state {
        CircuitState::Open { .. } => (4, "blocked"),
        CircuitState::HalfOpen => (3, "paused"),
        CircuitState::Closed => {
            if pending > 0 {
                (0, "waiting")
            } else if w
                .last_used_at
                .is_some_and(|ts| (now - ts).num_seconds() <= WORKING_FRESHNESS_SECS)
            {
                (1, "working")
            } else {
                (2, "idle")
            }
        }
    }
}

async fn pending_approvals_by_worker() -> HashMap<WorkerId, u32> {
    let mut out: HashMap<WorkerId, u32> = HashMap::new();
    let actions = match collect_latest_actions(None).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(target: "gateway::team_list", error = %e, "pending action lookup failed");
            return out;
        }
    };
    for a in actions {
        if a.requires_approval && a.state == ActionState::Pending.as_str() {
            let entry = out.entry(a.worker_id).or_insert(0);
            *entry = entry.saturating_add(1);
        }
    }
    out
}

async fn build_org_index(
    store: &Arc<SessionStore>,
    tenant: TenantId,
) -> ArgoResult<HashMap<WorkerId, OrgRefs>> {
    let org = OrgStore::new(Arc::clone(store));
    org.ensure_schema().await?;
    let units = org.list_units(tenant).await?;
    let members = org.list_members(tenant).await?;
    let by_unit_id: HashMap<&str, &argo_identity::OrgUnit> =
        units.iter().map(|u| (u.id.as_str(), u)).collect();
    let mut idx: HashMap<WorkerId, OrgRefs> = HashMap::new();
    for m in &members {
        let Some(worker_id_str) = m.worker_id.as_deref() else { continue };
        let Ok(wid) = worker_id_str.parse::<WorkerId>() else { continue };
        let unit = m.unit_id.as_deref().and_then(|id| by_unit_id.get(id).copied());
        let team = unit.map(|u| json!({ "id": u.id, "name": u.name }));
        let department = unit
            .and_then(|u| u.parent_id.as_deref())
            .and_then(|pid| by_unit_id.get(pid).copied())
            .map(|u| json!({ "id": u.id, "name": u.name }));
        idx.insert(wid, (department, team));
    }
    Ok(idx)
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

fn parse_tenant_query(req: &str) -> Result<Option<TenantId>, String> {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let head = tail.split_once(' ').map_or(tail, |(p, _)| p);
    let Some(qs) = head.split_once('?').map(|(_, q)| q) else { return Ok(None) };
    let raw = qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == "tenant_id").then(|| v.to_string())
    });
    let Some(raw) = raw else { return Ok(None) };
    Uuid::parse_str(&raw)
        .map(|u| Some(TenantId(u)))
        .map_err(|e| format!("invalid tenant_id '{raw}': {e}"))
}

async fn emit_audit(store: &Arc<SessionStore>, req: &str, count: usize, tenant: TenantId) {
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let payload = json!({
        "endpoint": "team.list.queried",
        "actor": actor,
        "tenant_id": tenant.to_string(),
        "count": count,
    });
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, Some(actor.clone()))
        .await
    {
        tracing::warn!(
            target: "gateway::team_list",
            error = %e,
            actor = %actor,
            "team.list.queried audit write failed"
        );
    }
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
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
#[path = "routes_team_list_tests.rs"]
mod tests;
