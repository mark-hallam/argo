//! `GET /api/v1/workers/<id>/overview` — Overview tab bundle (J8).
//!
//! One round-trip yields { charter, duties, kpis, recent_activity,
//! permissions }. Re-uses J7's permission resolution + tenant
//! scoping. A single audit-window query feeds both the KPI aggregates
//! and the 5 recent-activity rows (N+1 prevention). v1 posture:
//! `charter.version=1` + `last_edited_at = worker.created_at` (no
//! edit-history table yet); KPIs derived from
//! `argo_team_headline::bullets_for_vertical` via a public-API
//! `format()` probe because `BulletSource` isn't re-exported.
//! Hardening targets are named in the brief's v1-vs-target table.
//!
//! Split for the 500-LOC ceiling: KPI bullets, sparkline / trend
//! math, recent-activity formatting, and duty-schedule helpers live
//! in `routes_worker_overview_kpis`. Routing, auth, response wrappers,
//! and the audit emission stay here.

#[path = "routes_worker_overview_kpis.rs"]
mod kpis;

use kpis::{compute_kpis, duties_for_worker, fetch_audit_rows};

#[cfg(test)]
use kpis::{classify_source, label_from_template, next_fire_for, slugify, trend_for, SourceKind};

use super::routes_auth_principal::principal_id_for_audit;
use super::routes_worker_detail::{resolve_permissions, Permissions, PrincipalRole};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Duration, TimeZone, Utc};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

const ROLE_HEADER: &str = "X-Argo-Role";
const RECOVERABLE_DAYS: i64 = 30;
const WORKER_OVERVIEW_PERMISSION_NAMES: [&str; 7] = [
    "can_edit_charter",
    "can_edit_duties",
    "can_pause_resume",
    "can_archive",
    "can_delete",
    "can_view_audit",
    "can_view_settings",
];

type Resp = (&'static str, String);

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemoPrincipal {
    principal_id: String,
}

#[derive(Debug, Deserialize)]
struct DemoSessionClaims {
    #[serde(default)]
    email: String,
    #[serde(default)]
    demo: bool,
    #[serde(rename = "iat")]
    _iat: i64,
    #[serde(rename = "exp")]
    _exp: i64,
}

pub(super) async fn show_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Resp {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match build_overview(store, worker_id_raw, req).await {
        Ok(ok) | Err(ok) => ok,
    }
}

async fn build_overview(
    store: &Arc<SessionStore>,
    worker_id_raw: &str,
    req: &str,
) -> Result<Resp, Resp> {
    let raw_id = worker_id_raw.split('?').next().unwrap_or(worker_id_raw);
    let worker_id = raw_id.parse::<WorkerId>().map_err(|e| {
        err(
            "400 Bad Request",
            "bad_request",
            &format!("invalid worker_id '{raw_id}': {e}"),
        )
    })?;
    let tenant = resolve_workspace_tenant(store)
        .await
        .map_err(|e| ise(&format!("resolve tenant: {e}")))?;
    // Cross-tenant existence is hidden from the HTTP response.
    // See routes_workflows.rs:13-21 for the canonical invariant.
    // Probe runs internally for telemetry; response is identical for both branches.
    let worker = match IdentityStore::new(store)
        .get_worker(tenant, worker_id)
        .await
        .map_err(|e| ise(&format!("get worker: {e}")))?
    {
        Some(w) => w,
        None => {
            let other = exists_in_other_tenant(store, tenant, worker_id)
                .await
                .map_err(|e| ise(&format!("cross-tenant probe: {e}")))?;
            if other {
                tracing::debug!(target: "tenancy", %worker_id, "cross-tenant probe: exists in other tenant");
            }
            return Err(err("404 Not Found", "not_found", "worker not found"));
        }
    };
    let (created_at, archived_at) = worker_metadata(store, tenant, worker_id)
        .await
        .map_err(|e| ise(&format!("worker metadata: {e}")))?
        .ok_or_else(|| ise("worker metadata row missing"))?;
    if let Some(ts) = archived_at {
        return Err(gone_response(worker_id, &ts));
    }
    let header_actor = principal_id_for_audit(req);
    let demo_principal = demo_principal_for_request(req);
    let actor = header_actor
        .clone()
        .or_else(|| demo_principal.as_ref().map(|p| p.principal_id.clone()))
        .unwrap_or_else(|| "anonymous".to_string());
    let role = parse_role_header(req).unwrap_or(PrincipalRole::Delegator);
    let is_hire_author = !worker.persona.builder_attribution.is_empty()
        && worker.persona.builder_attribution == actor;
    let demo_override_allowed = demo_permission_override_allowed(
        demo_mode_enabled_for_permissions(),
        header_actor.as_deref(),
        demo_principal.as_ref(),
    );
    let (perms, demo_override_granted) =
        compute_permissions_demo(role, is_hire_author, demo_override_allowed);
    let now = Utc::now();
    let audit_rows = fetch_audit_rows(store, tenant, worker_id, now)
        .await
        .map_err(|e| ise(&format!("audit fetch: {e}")))?;
    let duties = duties_for_worker(store, tenant, worker_id, now)
        .await
        .map_err(|e| ise(&format!("duty fetch: {e}")))?;
    let body = json!({
        "charter": charter_json(&worker, created_at),
        "duties": duties,
        "kpis": compute_kpis(&worker.vertical_id, &audit_rows, now),
        "recent_activity": kpis::recent_activity_rows(&audit_rows),
        "permissions": permissions_json(perms),
    });
    emit_audit(store, tenant, &worker, &actor, role).await;
    if demo_override_granted {
        emit_demo_override_audit(store, tenant, &worker, &actor).await;
    }
    Ok(("200 OK", body.to_string()))
}

fn charter_json(worker: &Worker, created_at: DateTime<Utc>) -> Value {
    let text = worker
        .charter
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| worker.role.charter.clone());
    json!({
        "text": text, "version": 1,
        "last_edited_at": created_at.to_rfc3339(),
        "last_edited_by": worker.persona.builder_attribution,
    })
}

fn permissions_json(p: Permissions) -> Value {
    json!({
        "can_edit_charter": p.can_edit_charter, "can_edit_duties": p.can_edit_duties,
        "can_pause_resume": p.can_pause_resume, "can_archive": p.can_archive,
        "can_delete": p.can_delete, "can_view_audit": p.can_view_audit,
        "can_view_settings": p.can_view_settings,
    })
}

fn compute_permissions_demo(
    role: PrincipalRole,
    is_hire_author: bool,
    demo_override_allowed: bool,
) -> (Permissions, bool) {
    if demo_override_allowed {
        return (
            Permissions {
                can_edit_charter: true,
                can_edit_duties: true,
                can_pause_resume: true,
                can_archive: true,
                can_delete: true,
                can_view_audit: true,
                can_view_settings: true,
            },
            true,
        );
    }
    (resolve_permissions(role, is_hire_author), false)
}

fn demo_permission_override_allowed(
    demo_mode_enabled: bool,
    header_principal: Option<&str>,
    demo_principal: Option<&DemoPrincipal>,
) -> bool {
    if !demo_mode_enabled {
        return false;
    }
    let Some(demo_principal) = demo_principal else {
        return false;
    };
    header_principal
        .map(|p| p == demo_principal.principal_id)
        .unwrap_or(true)
}

fn demo_principal_for_request(req: &str) -> Option<DemoPrincipal> {
    if let Some(token) = super::session_jwt::bearer_from_request(req) {
        if let Some(principal) = demo_principal_from_token(&token) {
            return Some(principal);
        }
    }
    super::session_jwt::session_cookie_from_request(req)
        .and_then(|token| demo_principal_from_token(&token))
}

fn demo_principal_from_token(token: &str) -> Option<DemoPrincipal> {
    if token.split('.').count() != 3 {
        return None;
    }
    let key = DecodingKey::from_secret(&super::auth_magic_link::jwt_secret());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.required_spec_claims.clear();
    validation.required_spec_claims.insert("exp".into());
    validation.required_spec_claims.insert("iat".into());
    validation.leeway = 60;
    let claims = decode::<DemoSessionClaims>(token, &key, &validation)
        .ok()?
        .claims;
    if !claims.demo {
        return None;
    }
    let email = claims.email.trim().to_ascii_lowercase();
    if !super::auth_magic_link::looks_like_email(&email) {
        return None;
    }
    Some(DemoPrincipal {
        principal_id: format!("human:{email}"),
    })
}

fn demo_mode_enabled_for_permissions() -> bool {
    #[cfg(test)]
    {
        std::env::var(super::demo_mode::DEMO_MODE_ENV)
            .map(|v| v == "1")
            .unwrap_or(false)
    }
    #[cfg(not(test))]
    {
        super::demo_mode::is_enabled()
    }
}

/// Parse `X-Argo-Role`. Duplicated from `routes_worker_detail`
/// because that fn is module-private and routes_worker_detail.rs is
/// outside this brief's may-touch list.
pub(super) fn parse_role_header(req: &str) -> Option<PrincipalRole> {
    let headers = req.split_once("\r\n\r\n").map_or(req, |(h, _)| h);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.eq_ignore_ascii_case(ROLE_HEADER) {
            return None;
        }
        match value.trim().to_ascii_lowercase().as_str() {
            "delegator" => Some(PrincipalRole::Delegator),
            "operator" => Some(PrincipalRole::Operator),
            "auditor" => Some(PrincipalRole::Auditor),
            "compliance-officer" | "compliance_officer" => Some(PrincipalRole::ComplianceOfficer),
            _ => None,
        }
    })
}

async fn worker_metadata(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<Option<(DateTime<Utc>, Option<DateTime<Utc>>)>> {
    let row =
        sqlx::query("SELECT created_at, archived_at FROM worker WHERE id = ? AND tenant_id = ?")
            .bind(worker_id.to_string())
            .bind(tenant.to_string())
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("worker meta: {e}")))?;
    let Some(row) = row else { return Ok(None) };
    let created_micros: i64 = row.try_get("created_at").unwrap_or(0);
    let archived_raw: Option<String> = row.try_get("archived_at").ok();
    let created = micros_to_utc(created_micros);
    let archived = archived_raw.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    });
    Ok(Some((created, archived)))
}

async fn exists_in_other_tenant(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<bool> {
    let row: Option<String> =
        sqlx::query_scalar("SELECT id FROM worker WHERE id = ? AND tenant_id != ? LIMIT 1")
            .bind(worker_id.to_string())
            .bind(tenant.to_string())
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("cross-tenant probe: {e}")))?;
    Ok(row.is_some())
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

pub(super) fn micros_to_utc(micros: i64) -> DateTime<Utc> {
    Utc.timestamp_micros(micros)
        .single()
        .unwrap_or_else(Utc::now)
}

async fn emit_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker: &Worker,
    actor: &str,
    role: PrincipalRole,
) {
    let viewer = match role {
        PrincipalRole::Delegator => "delegator",
        PrincipalRole::Operator => "operator",
        PrincipalRole::Auditor => "auditor",
        PrincipalRole::ComplianceOfficer => "compliance-officer",
    };
    let payload = json!({
        "endpoint": "worker.overview.viewed", "actor": actor,
        "tenant_id": tenant.to_string(), "worker_id": worker.id.to_string(),
        "viewer_role": viewer,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            Some(actor.to_string()),
        )
        .await
    {
        tracing::warn!(target: "gateway::worker_overview", error = %e, actor = %actor,
            "worker.overview.viewed audit write failed");
    }
}

async fn emit_demo_override_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker: &Worker,
    principal_id: &str,
) {
    let payload = json!({
        "principal_id": principal_id,
        "tenant_id": tenant.to_string(),
        "worker_id": worker.id.to_string(),
        "permissions": WORKER_OVERVIEW_PERMISSION_NAMES,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(
            None,
            AuditKind::PermissionDemoOverrideGranted,
            payload,
            Some(principal_id.to_string()),
        )
        .await
    {
        tracing::warn!(target: "gateway::worker_overview", error = %e, principal_id = %principal_id,
            "worker overview demo permission override audit write failed");
    }
}

fn gone_response(worker_id: WorkerId, archived_at: &DateTime<Utc>) -> Resp {
    let recoverable = *archived_at + Duration::days(RECOVERABLE_DAYS);
    let body = json!({
        "error": { "code": "gone", "message": format!("worker {worker_id} is archived") },
        "archived_at": archived_at.to_rfc3339(),
        "recoverable_until_at": recoverable.to_rfc3339(),
        "retry_after_seconds": (recoverable - Utc::now()).num_seconds().max(0),
    });
    ("410 Gone", body.to_string())
}

fn ise(message: &str) -> Resp {
    err("500 Internal Server Error", "internal_error", message)
}

fn err(status: &'static str, code: &str, message: &str) -> Resp {
    (
        status,
        json!({ "error": { "code": code, "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_worker_overview_tests.rs"]
mod tests;
