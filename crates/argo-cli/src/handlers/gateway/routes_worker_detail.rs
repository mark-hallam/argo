//! `GET /api/v1/workers/<id>` — worker profile detail (Stream J7).
//!
//! Read-only HTTP shim over `IdentityStore::get_worker` that powers
//! E4-a's worker profile page. Surfaces the full `WorkerDetail`
//! shape (persona, role, team, headline, KPI of the day) plus a
//! server-computed `permissions` block scoped to the caller's
//! `X-Argo-Role` claim.
//!
//! ## v1 posture vs target
//!
//! - Tenant scope: workspace-cwd-derived, matching `routes_team_list`
//!   and `routes_worker_trust_grade`. Operator multi-tenant context
//!   is `mission-enterprise-cross-tenant-access`.
//! - Principal roles: parsed from the `X-Argo-Role` header
//!   (`delegator` | `operator` | `auditor` | `compliance-officer`).
//!   No such role concept lives in the principal model today; the
//!   header is the documented v1 shim. JWT/RBAC-claim driven
//!   resolution is the architectural target (see
//!   `mission-enterprise-principal-roles`).
//! - Hire-author check: `worker.persona.builder_attribution ==
//!   caller's principal id`. Delegator perms key off this.
//! - Soft-deleted workers: `archived_at IS NOT NULL` → 410 Gone
//!   with `recoverable_until_at` in the body (the gateway
//!   response writer in `boot.rs` is fixed-shape, so the
//!   `Retry-After` hint travels in the JSON body, not a header).
//!
//! ## Sensible deviations from the brief
//!
//! - Audit row uses `AuditKind::HttpRequest` with `endpoint:
//!   "worker.detail.viewed"` (matches J5/J6 convention; adding an
//!   enum variant is workspace-wide and out of scope).
//! - JSON keys are snake_case (`can_edit_charter`, not
//!   `canEditCharter`) to match every other gateway response.
//! - Cross-tenant ids surface as 403 (per acceptance criteria),
//!   distinguished from 404 (unknown id) via one extra existence
//!   probe — the scope-table 404-or-403 ambiguity resolved in
//!   favour of the explicit acceptance-criterion contract.

use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use argo_vertical::{KpiDefinition, KpiKind};
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use std::sync::Arc;

const RECOVERABLE_DAYS: i64 = 30;
const ROLE_HEADER: &str = "X-Argo-Role";

/// Principal role parsed from the `X-Argo-Role` request header.
/// v1 shim; resolved from a JWT/RBAC claim once the auth layer
/// grows native role support (`mission-enterprise-principal-roles`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PrincipalRole {
    Delegator,
    Operator,
    Auditor,
    ComplianceOfficer,
}

impl PrincipalRole {
    fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "delegator" => Some(Self::Delegator),
            "operator" => Some(Self::Operator),
            "auditor" => Some(Self::Auditor),
            "compliance-officer" | "compliance_officer" => Some(Self::ComplianceOfficer),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Delegator => "delegator",
            Self::Operator => "operator",
            Self::Auditor => "auditor",
            Self::ComplianceOfficer => "compliance-officer",
        }
    }
}

/// Server-resolved per-action permissions for the caller. Single
/// shape regardless of role; the resolver toggles the booleans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Permissions {
    pub can_edit_charter: bool,
    pub can_edit_duties: bool,
    pub can_pause_resume: bool,
    pub can_archive: bool,
    pub can_delete: bool,
    pub can_view_audit: bool,
    pub can_view_settings: bool,
}

impl Permissions {
    fn to_json(self) -> Value {
        json!({
            "can_edit_charter": self.can_edit_charter,
            "can_edit_duties": self.can_edit_duties,
            "can_pause_resume": self.can_pause_resume,
            "can_archive": self.can_archive,
            "can_delete": self.can_delete,
            "can_view_audit": self.can_view_audit,
            "can_view_settings": self.can_view_settings,
        })
    }
}

/// Map `(role, is_hire_author)` to the seven booleans. Pure
/// function — tests drive it directly without a store.
pub(super) fn resolve_permissions(role: PrincipalRole, is_hire_author: bool) -> Permissions {
    match role {
        PrincipalRole::Operator => Permissions {
            can_edit_charter: true,
            can_edit_duties: true,
            can_pause_resume: true,
            can_archive: true,
            can_delete: true,
            can_view_audit: true,
            can_view_settings: true,
        },
        PrincipalRole::Auditor => Permissions {
            can_edit_charter: false,
            can_edit_duties: false,
            can_pause_resume: false,
            can_archive: false,
            can_delete: false,
            can_view_audit: true,
            can_view_settings: false,
        },
        PrincipalRole::ComplianceOfficer => Permissions {
            can_edit_charter: false,
            can_edit_duties: false,
            can_pause_resume: false,
            can_archive: true,
            can_delete: false,
            can_view_audit: true,
            can_view_settings: false,
        },
        PrincipalRole::Delegator => Permissions {
            can_edit_charter: is_hire_author,
            can_edit_duties: is_hire_author,
            can_pause_resume: is_hire_author,
            can_archive: is_hire_author,
            can_delete: false,
            can_view_audit: is_hire_author,
            can_view_settings: false,
        },
    }
}

/// `GET /api/v1/workers/<id>` entry point. `worker_id_raw` is the
/// path segment after `/workers/` (and may include the trailing
/// `?query`; we strip it before parsing).
pub(super) async fn show_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let raw_id = worker_id_raw.split('?').next().unwrap_or(worker_id_raw);
    let worker_id = match raw_id.parse::<WorkerId>() {
        Ok(id) => id,
        Err(e) => return bad_request(&format!("invalid worker_id '{raw_id}': {e}")),
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

    let actor =
        principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let role = parse_role_header(req).unwrap_or(PrincipalRole::Delegator);
    let is_hire_author = !worker.persona.builder_attribution.is_empty()
        && worker.persona.builder_attribution == actor;
    let permissions = resolve_permissions(role, is_hire_author);

    let pending = pending_approval_count(&worker.id).await;
    let kpi_today = derive_kpi_today(&worker.vertical_id);
    let now = Utc::now();
    let (department, team) = build_org_refs(store, tenant, worker.id).await;
    let headline = build_headline(store, &worker).await;
    let status_label = super::routes_team_list::derive_status(&worker, pending, now).1;

    let body = build_detail(
        &worker,
        status_label,
        department,
        team,
        kpi_today,
        permissions,
        headline,
        pending,
    );
    emit_audit(store, tenant, &worker, &actor, role).await;
    ("200 OK", body.to_string())
}

#[allow(clippy::too_many_arguments)]
fn build_detail(
    w: &Worker,
    status_label: &'static str,
    department: Option<Value>,
    team: Option<Value>,
    kpi_today: Option<Value>,
    permissions: Permissions,
    headline: Option<Value>,
    pending: u32,
) -> Value {
    let avatar = w.persona.avatar.clone().filter(|s| !s.is_empty());
    let tagline = w
        .persona
        .bio
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| Some(w.role.charter.clone()).filter(|s| !s.is_empty()))
        .unwrap_or_else(|| w.persona.name.clone());
    json!({
        "id": w.id.to_string(),
        "name": w.persona.name,
        "avatar_url": avatar,
        "role": { "tagline": tagline, "vertical": w.vertical_id },
        "status": status_label,
        "department": department,
        "team": team,
        "kpi_today": kpi_today,
        "permissions": permissions.to_json(),
        "headline": headline,
        "last_active_at": w.last_used_at.map(|ts| ts.to_rfc3339()),
        "pending_approval_count": pending,
    })
}

fn parse_role_header(req: &str) -> Option<PrincipalRole> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case(ROLE_HEADER)
            .then(|| PrincipalRole::parse(value.trim()))
            .flatten()
    })
}

async fn pending_approval_count(worker_id: &WorkerId) -> u32 {
    use crate::handlers::worker_actions::ActionState;
    use crate::handlers::worker_actions_query::collect_latest_actions;
    let actions = match collect_latest_actions(None).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(target: "gateway::worker_detail", error = %e, "pending action lookup failed");
            return 0;
        }
    };
    let pending = ActionState::Pending.as_str();
    let count = actions
        .into_iter()
        .filter(|a| a.requires_approval && a.state == pending && &a.worker_id == worker_id)
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn derive_kpi_today(vertical_id: &str) -> Option<Value> {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir).ok()?;
    let loaded = registry.get(vertical_id)?;
    let kpis = loaded.manifest.all_kpis();
    let first = kpis.into_iter().next()?;
    Some(format_kpi(&first))
}

fn format_kpi(k: &KpiDefinition) -> Value {
    let label = k.description.clone().unwrap_or_else(|| k.name.clone());
    let value = match k.kind {
        KpiKind::Percentage => format!("{}%", k.threshold),
        KpiKind::DurationMs => format!("{}ms", k.threshold),
        KpiKind::Count => format!("{}", k.threshold as i64),
        KpiKind::Ratio => format!("{:.2}", k.threshold),
        KpiKind::Score => format!("{}", k.threshold),
    };
    json!({ "label": label, "value": value })
}

async fn build_org_refs(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
) -> (Option<Value>, Option<Value>) {
    use argo_identity::OrgStore;
    let org = OrgStore::new(Arc::clone(store));
    if let Err(e) = org.ensure_schema().await {
        tracing::warn!(target: "gateway::worker_detail", error = %e, "org schema unavailable");
        return (None, None);
    }
    let units = org.list_units(tenant).await.unwrap_or_default();
    let members = org.list_members(tenant).await.unwrap_or_default();
    let by_id: std::collections::HashMap<&str, &argo_identity::OrgUnit> =
        units.iter().map(|u| (u.id.as_str(), u)).collect();
    let wid_str = worker_id.to_string();
    let me = members
        .iter()
        .find(|m| m.worker_id.as_deref() == Some(wid_str.as_str()));
    let Some(member) = me else { return (None, None) };
    let unit = member.unit_id.as_deref().and_then(|id| by_id.get(id).copied());
    let team = unit.map(|u| json!({ "id": u.id, "name": u.name }));
    let department = unit
        .and_then(|u| u.parent_id.as_deref())
        .and_then(|pid| by_id.get(pid).copied())
        .map(|u| json!({ "id": u.id, "name": u.name }));
    (department, team)
}

async fn build_headline(store: &Arc<SessionStore>, worker: &Worker) -> Option<Value> {
    match argo_team_headline::generate_team_headline(worker.id, &worker.vertical_id, store).await {
        Ok(h) => Some(json!({ "bullets": h.bullets, "tomorrow": h.tomorrow })),
        Err(e) => {
            tracing::warn!(
                target: "gateway::worker_detail",
                worker_id = %worker.id,
                error = %e,
                "headline generation failed"
            );
            None
        }
    }
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

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

async fn emit_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker: &Worker,
    actor: &str,
    role: PrincipalRole,
) {
    let payload = json!({
        "endpoint": "worker.detail.viewed",
        "actor": actor,
        "tenant_id": tenant.to_string(),
        "worker_id": worker.id.to_string(),
        "viewer_role": role.as_str(),
    });
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, Some(actor.to_string()))
        .await
    {
        tracing::warn!(
            target: "gateway::worker_detail",
            error = %e,
            actor = %actor,
            "worker.detail.viewed audit write failed"
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
#[path = "routes_worker_detail_tests.rs"]
mod tests;
