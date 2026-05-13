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

use super::routes_auth_principal::principal_id_for_audit;
use super::routes_worker_detail::{resolve_permissions, Permissions, PrincipalRole};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_store::SessionStore;
use argo_team_headline::{bullets_for_vertical, ActivityCounts, BulletKind};
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use cron::Schedule;
use serde_json::{json, Value};
use sqlx::Row;
use std::str::FromStr;
use std::sync::Arc;

const ROLE_HEADER: &str = "X-Argo-Role";
const RECOVERABLE_DAYS: i64 = 30;
const SPARKLINE_DAYS: usize = 7;
const KPI_WINDOW_DAYS: i64 = 14;
const RECENT_LIMIT: usize = 5;

type Resp = (&'static str, String);

pub(super) async fn show_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Resp {
    let Some(store) = store else { return super::store_unavailable() };
    match build_overview(store, worker_id_raw, req).await {
        Ok(ok) | Err(ok) => ok,
    }
}

async fn build_overview(store: &Arc<SessionStore>, worker_id_raw: &str, req: &str) -> Result<Resp, Resp> {
    let raw_id = worker_id_raw.split('?').next().unwrap_or(worker_id_raw);
    let worker_id = raw_id.parse::<WorkerId>().map_err(|e| {
        err("400 Bad Request", "bad_request", &format!("invalid worker_id '{raw_id}': {e}"))
    })?;
    let tenant = resolve_workspace_tenant(store)
        .await
        .map_err(|e| ise(&format!("resolve tenant: {e}")))?;
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
            return Err(if other {
                err("403 Forbidden", "forbidden", &format!("worker {worker_id} not in this tenant"))
            } else {
                err("404 Not Found", "not_found", &format!("worker {worker_id} not found"))
            });
        }
    };
    let (created_at, archived_at) = worker_metadata(store, tenant, worker_id)
        .await
        .map_err(|e| ise(&format!("worker metadata: {e}")))?
        .ok_or_else(|| ise("worker metadata row missing"))?;
    if let Some(ts) = archived_at {
        return Err(gone_response(worker_id, &ts));
    }
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let role = parse_role_header(req).unwrap_or(PrincipalRole::Delegator);
    let is_hire_author = !worker.persona.builder_attribution.is_empty()
        && worker.persona.builder_attribution == actor;
    let perms = resolve_permissions(role, is_hire_author);
    let now = Utc::now();
    let audit_rows = fetch_audit_rows(store, worker_id, now)
        .await
        .map_err(|e| ise(&format!("audit fetch: {e}")))?;
    let duties = duties_for_worker(store, tenant, worker_id, now)
        .await
        .map_err(|e| ise(&format!("duty fetch: {e}")))?;
    let body = json!({
        "charter": charter_json(&worker, created_at),
        "duties": duties,
        "kpis": compute_kpis(&worker.vertical_id, &audit_rows, now),
        "recent_activity": recent_activity_rows(&audit_rows),
        "permissions": permissions_json(perms),
    });
    emit_audit(store, tenant, &worker, &actor, role).await;
    Ok(("200 OK", body.to_string()))
}

fn charter_json(worker: &Worker, created_at: DateTime<Utc>) -> Value {
    let text = worker.charter.clone().filter(|s| !s.is_empty())
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
    let row = sqlx::query(
        "SELECT created_at, archived_at FROM worker WHERE id = ? AND tenant_id = ?",
    )
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
        DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
    });
    Ok(Some((created, archived)))
}

async fn exists_in_other_tenant(store: &SessionStore, tenant: TenantId, worker_id: WorkerId) -> ArgoResult<bool> {
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
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

fn micros_to_utc(micros: i64) -> DateTime<Utc> {
    Utc.timestamp_micros(micros).single().unwrap_or_else(Utc::now)
}

#[derive(Debug, Clone)]
pub(super) struct AuditRow {
    pub id: String,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub payload: Value,
}

async fn fetch_audit_rows(store: &SessionStore, worker_id: WorkerId, now: DateTime<Utc>) -> ArgoResult<Vec<AuditRow>> {
    let since = (now - Duration::days(KPI_WINDOW_DAYS)).timestamp_micros();
    let rows = sqlx::query(
        "SELECT a.id, a.kind, a.payload_json, a.created_at \
         FROM audit_record a JOIN session s ON s.id = a.session_id \
         WHERE s.worker_id = ? AND a.created_at >= ? \
         ORDER BY a.created_at DESC",
    )
    .bind(worker_id.0.as_bytes().to_vec())
    .bind(since)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("audit window: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let id_bytes: Vec<u8> = row.try_get("id").unwrap_or_default();
            let id = uuid::Uuid::from_slice(&id_bytes).map(|u| u.to_string()).unwrap_or_default();
            let kind: String = row.try_get("kind").unwrap_or_default();
            let payload_json: String = row.try_get("payload_json").unwrap_or_default();
            let created_micros: i64 = row.try_get("created_at").unwrap_or(0);
            AuditRow {
                id,
                kind,
                created_at: micros_to_utc(created_micros),
                payload: serde_json::from_str(&payload_json).unwrap_or(Value::Null),
            }
        })
        .collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceKind {
    Messages,
    ToolCalls,
    Pending,
    AnyAction,
    NotComputable,
}

/// Recover a `BulletKind`'s underlying `BulletSource` by probing its
/// public `format()` against synthetic `ActivityCounts`. The
/// `BulletSource` enum is `pub` inside `argo-team-headline` but not
/// re-exported; v1.5 closes via the manifest-driven KPI table
/// (`mission-enterprise-team-headline-manifest`).
pub(super) fn classify_source(kind: &BulletKind) -> SourceKind {
    let probe = |a, t, p| ActivityCounts {
        assistant_messages: a,
        tool_calls: t,
        pending_escalations: p,
    };
    let on_msg = kind.format(&probe(1, 0, 0)).is_some();
    let on_tool = kind.format(&probe(0, 1, 0)).is_some();
    let on_pend = kind.format(&probe(0, 0, 1)).is_some();
    match (on_msg, on_tool, on_pend) {
        (true, true, false) => SourceKind::AnyAction,
        (true, false, false) => SourceKind::Messages,
        (false, true, false) => SourceKind::ToolCalls,
        (false, false, true) => SourceKind::Pending,
        _ => SourceKind::NotComputable,
    }
}

pub(super) fn label_from_template(template: &str) -> String {
    let stripped = template.trim_start_matches("{n}").trim();
    if stripped.is_empty() {
        return template.to_string();
    }
    let mut chars = stripped.chars();
    match chars.next() {
        Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
        None => stripped.to_string(),
    }
}

pub(super) fn slugify(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut prev_us = false;
    for c in label.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_us = false;
        } else if !prev_us {
            out.push('_');
            prev_us = true;
        }
    }
    out.trim_matches('_').to_string()
}

fn row_matches(source: SourceKind, kind: &str) -> bool {
    match source {
        SourceKind::Messages => kind == "assistant_message",
        SourceKind::ToolCalls => kind == "tool_call",
        SourceKind::Pending => kind == "escalation.required",
        SourceKind::AnyAction => matches!(kind, "assistant_message" | "tool_call"),
        SourceKind::NotComputable => false,
    }
}

fn sparkline_for(audit: &[AuditRow], source: SourceKind, today: NaiveDate) -> [i64; SPARKLINE_DAYS] {
    let mut bucket = [0_i64; SPARKLINE_DAYS];
    for row in audit {
        if !row_matches(source, &row.kind) {
            continue;
        }
        let delta = today.num_days_from_ce() - row.created_at.date_naive().num_days_from_ce();
        if delta < 0 || delta >= SPARKLINE_DAYS as i32 {
            continue;
        }
        bucket[SPARKLINE_DAYS - 1 - delta as usize] += 1;
    }
    bucket
}

fn prev_window_count(audit: &[AuditRow], source: SourceKind, now: DateTime<Utc>) -> i64 {
    let prev_start = now - Duration::days(KPI_WINDOW_DAYS);
    let prev_end = now - Duration::days(SPARKLINE_DAYS as i64);
    audit
        .iter()
        .filter(|r| row_matches(source, &r.kind) && r.created_at >= prev_start && r.created_at < prev_end)
        .count() as i64
}

pub(super) fn trend_for(curr: f64, prev: f64) -> (&'static str, f64) {
    let pct = if prev > 0.0 {
        (curr - prev) / prev * 100.0
    } else if curr > 0.0 {
        100.0
    } else {
        0.0
    };
    let direction = if pct.abs() < 2.0 { "flat" } else if pct > 0.0 { "up" } else { "down" };
    (direction, (pct * 10.0).round() / 10.0)
}

fn compute_kpis(vertical_id: &str, audit: &[AuditRow], now: DateTime<Utc>) -> Vec<Value> {
    let today = now.date_naive();
    bullets_for_vertical(vertical_id)
        .iter()
        .filter_map(|kind| {
            let source = classify_source(kind);
            if source == SourceKind::NotComputable {
                return None;
            }
            let sparkline = sparkline_for(audit, source, today);
            let curr: i64 = sparkline.iter().sum();
            let prev = prev_window_count(audit, source, now);
            let (direction, delta_pct) = trend_for(curr as f64, prev as f64);
            let label = label_from_template(kind.plural);
            Some(json!({
                "key": slugify(&label),
                "label": label,
                "value": curr.to_string(),
                "trend": { "direction": direction, "delta_pct": delta_pct },
                "sparkline_7d": sparkline,
                "direction_means_good": !matches!(source, SourceKind::Pending),
            }))
        })
        .collect()
}

fn recent_activity_rows(audit: &[AuditRow]) -> Vec<Value> {
    audit
        .iter()
        .take(RECENT_LIMIT)
        .map(|row| {
            let verb = row.payload.get("verb").and_then(Value::as_str).unwrap_or(&row.kind).to_string();
            let status = row
                .payload
                .get("status")
                .and_then(Value::as_str)
                .or_else(|| row.payload.get("outcome_status").and_then(Value::as_str))
                .unwrap_or("succeeded")
                .to_string();
            let cost = row
                .payload
                .get("cost_usd")
                .and_then(Value::as_f64)
                .map_or(Value::Null, |usd| json!({ "usd": usd }));
            json!({
                "id": row.id,
                "timestamp": row.created_at.to_rfc3339(),
                "verb": verb,
                "status": status,
                "cost": cost,
            })
        })
        .collect()
}

async fn duties_for_worker(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
    now: DateTime<Utc>,
) -> ArgoResult<Vec<Value>> {
    let rows = match sqlx::query(
        "SELECT id, cron, entry_action, description, enabled, paused_until, last_fired_at \
         FROM role_duty WHERE tenant_id = ? AND worker_id = ? \
         ORDER BY created_at ASC, id ASC",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(worker_id.to_string())
    .fetch_all(store.pool())
    .await
    {
        Ok(r) => r,
        Err(sqlx::Error::Database(db)) if db.message().contains("no such table") => return Ok(Vec::new()),
        Err(e) => return Err(ArgoError::Provider(format!("role_duty list: {e}"))),
    };
    Ok(rows
        .into_iter()
        .map(|row| {
            let id: String = row.try_get("id").unwrap_or_default();
            let cron: String = row.try_get("cron").unwrap_or_default();
            let entry_action: String = row.try_get("entry_action").unwrap_or_default();
            let description: String = row.try_get("description").unwrap_or_default();
            let enabled: i64 = row.try_get("enabled").unwrap_or(0);
            let paused_until: Option<i64> = row.try_get("paused_until").ok().flatten();
            let last_fired: Option<i64> = row.try_get("last_fired_at").ok().flatten();
            let label = if description.trim().is_empty() {
                entry_action.clone()
            } else {
                description.clone()
            };
            let live = enabled == 1 && !matches!(paused_until, Some(u) if u > now.timestamp_micros());
            let next_fire_at = next_fire_for(&cron, last_fired, now).map(|t| t.to_rfc3339());
            json!({
                "id": id,
                "label": label,
                "schedule": { "kind": "cron", "spec": cron },
                "description": description,
                "enabled": live,
                "next_fire_at": next_fire_at,
            })
        })
        .collect())
}

pub(super) fn next_fire_for(cron_expr: &str, last_fired_micros: Option<i64>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let schedule = Schedule::from_str(cron_expr).ok()?;
    let origin = last_fired_micros
        .and_then(|m| Utc.timestamp_micros(m).single())
        .filter(|dt| *dt < now)
        .unwrap_or(now);
    schedule.after(&origin).next()
}

async fn emit_audit(store: &Arc<SessionStore>, tenant: TenantId, worker: &Worker, actor: &str, role: PrincipalRole) {
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
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink.write(None, AuditKind::HttpRequest, payload, Some(actor.to_string())).await {
        tracing::warn!(target: "gateway::worker_overview", error = %e, actor = %actor,
            "worker.overview.viewed audit write failed");
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
    (status, json!({ "error": { "code": code, "message": message } }).to_string())
}

#[cfg(test)]
#[path = "routes_worker_overview_tests.rs"]
mod tests;
