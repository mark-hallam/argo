//! Shared helpers for the J13 per-worker duty handlers.
//!
//! Body parsing, worker resolution, cron/label validation, conflict
//! detection, audit emission, and error-envelope formatting live here so
//! the handler module stays well under the 500-LOC ceiling.

use super::super::body_from_request;
use super::super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_genesis::Archetype;
use argo_identity::{IdentityStore, WorkerId};
use argo_orchestrator::{DutyRecord, DutyStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;

pub(super) const MIN_LABEL_LEN: usize = 4;
pub(super) const MAX_LABEL_LEN: usize = 100;
pub(super) const NEXT_FIRES_LOOKAHEAD: usize = 10;
pub(super) const CONFLICT_WINDOW_SECS: i64 = 60;

pub(super) fn parse_body(req: &str) -> Result<Value, (&'static str, String)> {
    let body = body_from_request(req);
    if body.trim().is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    serde_json::from_str(body).map_err(|e| err("400 Bad Request", "bad_request", &format!("parse body: {e}")))
}

pub(super) async fn resolve_worker<'a>(
    worker_id_raw: &str,
    store: Option<&'a Arc<SessionStore>>,
) -> Result<(&'a Arc<SessionStore>, WorkerId, TenantId), (&'static str, String)> {
    let Some(store) = store else { return Err(super::super::store_unavailable()) };
    let raw = worker_id_raw.split('?').next().unwrap_or(worker_id_raw);
    let worker_id = raw.parse::<WorkerId>()
        .map_err(|e| err("400 Bad Request", "bad_request", &format!("invalid worker_id '{raw}': {e}")))?;
    let tenant = resolve_workspace_tenant(store).await
        .map_err(|e| err("500 Internal Server Error", "internal_error", &format!("resolve tenant: {e}")))?;
    let identities = IdentityStore::new(store);
    match identities.get_worker(tenant, worker_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return Err(match exists_in_other_tenant(store, tenant, worker_id).await {
            Ok(true) => err("403 Forbidden", "forbidden", &format!("worker {worker_id} not in this tenant")),
            Ok(false) => err("404 Not Found", "not_found", &format!("worker {worker_id} not found")),
            Err(e) => err("500 Internal Server Error", "internal_error", &format!("cross-tenant probe: {e}")),
        }),
        Err(e) => return Err(err("500 Internal Server Error", "internal_error", &format!("get worker: {e}"))),
    }
    if let Some(ts) = fetch_archived_at(store, tenant, worker_id).await
        .map_err(|e| err("500 Internal Server Error", "internal_error", &format!("archived lookup: {e}")))?
    {
        return Err(gone_response(worker_id, &ts));
    }
    Ok((store, worker_id, tenant))
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let (tenant, _ws) = store.ensure_workspace(name, &cwd.to_string_lossy()).await?;
    Ok(tenant)
}

async fn exists_in_other_tenant(store: &SessionStore, tenant: TenantId, worker_id: WorkerId) -> ArgoResult<bool> {
    let row: Option<String> =
        sqlx::query_scalar("SELECT id FROM worker WHERE id = ? AND tenant_id != ? LIMIT 1")
            .bind(worker_id.to_string()).bind(tenant.to_string())
            .fetch_optional(store.pool()).await
            .map_err(|e| ArgoError::Provider(format!("cross-tenant probe: {e}")))?;
    Ok(row.is_some())
}

async fn fetch_archived_at(
    store: &SessionStore, tenant: TenantId, worker_id: WorkerId,
) -> ArgoResult<Option<DateTime<Utc>>> {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT archived_at FROM worker WHERE id = ? AND tenant_id = ? AND archived_at IS NOT NULL",
    )
    .bind(worker_id.to_string()).bind(tenant.to_string())
    .fetch_optional(store.pool()).await
    .map_err(|e| ArgoError::Provider(format!("archived_at lookup: {e}")))?;
    Ok(raw.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))))
}

pub(super) fn validate_5_field_cron(input: &str) -> Result<String, String> {
    let fields: Vec<&str> = input.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(format!("cron must be 5 fields, got {}", fields.len()));
    }
    let six = format!("0 {}", fields.join(" "));
    Schedule::from_str(&six).map_err(|e| format!("invalid cron: {e}"))?;
    Ok(six)
}

pub(super) fn validate_label(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if !(MIN_LABEL_LEN..=MAX_LABEL_LEN).contains(&trimmed.len()) {
        return Err(format!("`label` must be {MIN_LABEL_LEN}..={MAX_LABEL_LEN} chars"));
    }
    Ok(trimmed.to_string())
}

fn next_fires(stored_cron: &str, now: DateTime<Utc>, n: usize) -> Vec<DateTime<Utc>> {
    Schedule::from_str(stored_cron).ok().map(|s| s.after(&now).take(n).collect()).unwrap_or_default()
}

pub(super) async fn collect_conflicts(
    duty_store: &DutyStore, worker_id: &WorkerId, proposed_stored_cron: &str,
) -> ArgoResult<Vec<Value>> {
    let now = Utc::now();
    let proposed = next_fires(proposed_stored_cron, now, NEXT_FIRES_LOOKAHEAD);
    // v1 worker-local scope; the store has no `list_for_worker` so we post-filter.
    let active = duty_store.list_active(now).await?;
    let mut conflicts = Vec::new();
    for existing in active.into_iter().filter(|d| &d.worker_id == worker_id) {
        let existing_fires = next_fires(&existing.cron, now, NEXT_FIRES_LOOKAHEAD);
        if let Some((a, b)) = find_overlap(&proposed, &existing_fires) {
            conflicts.push(json!({
                "duty_id": existing.id, "label": existing.description, "cron": existing.cron,
                "overlap_at": a.to_rfc3339(), "existing_fires_at": b.to_rfc3339(),
            }));
        }
    }
    Ok(conflicts)
}

fn find_overlap(
    proposed: &[DateTime<Utc>], existing: &[DateTime<Utc>],
) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    proposed.iter().find_map(|p| {
        existing.iter()
            .find(|e| (p.timestamp() - e.timestamp()).abs() <= CONFLICT_WINDOW_SECS)
            .map(|e| (*p, *e))
    })
}

pub(super) async fn delete_conflicting(store: &SessionStore, conflicts: &[Value]) -> Result<(), sqlx::Error> {
    for c in conflicts {
        if let Some(id) = c.get("duty_id").and_then(Value::as_str) {
            sqlx::query("DELETE FROM role_duty WHERE id = ?")
                .bind(id).execute(store.pool()).await?;
        }
    }
    Ok(())
}

pub(super) async fn apply_patch(
    store: &SessionStore, duty_id: &str,
    new_cron: Option<&str>, new_label: Option<&str>, new_enabled: Option<bool>,
) -> Result<(), sqlx::Error> {
    if let Some(c) = new_cron {
        sqlx::query("UPDATE role_duty SET cron = ? WHERE id = ?")
            .bind(c).bind(duty_id).execute(store.pool()).await?;
    }
    if let Some(l) = new_label {
        sqlx::query("UPDATE role_duty SET description = ? WHERE id = ?")
            .bind(l).bind(duty_id).execute(store.pool()).await?;
    }
    if let Some(e) = new_enabled {
        sqlx::query("UPDATE role_duty SET enabled = ? WHERE id = ?")
            .bind(i64::from(e)).bind(duty_id).execute(store.pool()).await?;
    }
    Ok(())
}

pub(super) fn duty_to_json(d: &DutyRecord) -> Value {
    json!({
        "id": d.id, "worker_id": d.worker_id.to_string(), "cron": d.cron,
        "entry_action": d.entry_action, "label": d.description, "enabled": d.enabled,
        "paused_until": d.paused_until.map(|t| t.to_rfc3339()),
        "last_fired_at": d.last_fired_at.map(|t| t.to_rfc3339()),
        "created_at": d.created_at.to_rfc3339(),
    })
}

pub(super) struct DutyDefaults {
    pub(super) label: &'static str,
    pub(super) cron: &'static str,
    pub(super) entry_action: &'static str,
}

pub(super) fn defaults_for(a: Archetype) -> DutyDefaults {
    use Archetype as A;
    let (label, cron, entry_action) = match a {
        A::SalesSdr => ("Send outbound batch", "0 9 * * 1-5", "send_outbound_batch"),
        A::CustomerSupportAgent => ("Triage inbox", "*/30 * * * *", "triage_inbox"),
        A::MarketingIntern | A::SocialMediaDrafter => ("Draft post", "0 6 * * *", "draft_post"),
        A::RecruiterScreener => ("Screen candidates", "0 9 * * 1-5", "screen_candidates"),
        A::OpsOnCall => ("Check alerts", "*/30 * * * *", "check_alerts"),
        A::ResearchAnalyst => ("Summarize news", "0 6 * * *", "summarize_news"),
        A::CalendarCoordinator => ("Review calendar", "0 8 * * 1-5", "review_calendar"),
        A::LeadResearcher => ("Research leads", "0 9 * * 1-5", "research_leads"),
        A::BookkeepingAssistant => ("Reconcile ledger", "0 17 * * 5", "reconcile_ledger"),
        A::ProjectPm => ("Send status summary", "0 17 * * 5", "send_status_summary"),
        A::GtmEngineer => ("Run GTM check", "0 9 * * *", "run_gtm_check"),
    };
    DutyDefaults { label, cron, entry_action }
}

pub(super) fn sample_outputs_for(a: Archetype) -> Vec<String> {
    use Archetype as A;
    let t: [&str; 3] = match a {
        A::SalesSdr | A::LeadResearcher => [
            "Sent 8 prospecting emails to qualified leads (3 replies).",
            "Logged 5 follow-ups; 2 booked discovery calls for tomorrow.",
            "Refreshed cadence for stalled deals (4 nudged).",
        ],
        A::CustomerSupportAgent | A::OpsOnCall => [
            "Triaged 6 tickets; resolved 3, escalated 1 to on-call.",
            "Replied to 9 questions with kb-cited answers.",
            "Flagged 1 churn-risk thread for the AM.",
        ],
        A::BookkeepingAssistant => [
            "Categorized 22 transactions; 3 awaiting confirmation.",
            "Reconciled bank feed against ledger (delta: $0.00).",
            "Drafted invoice for vendor X due Friday.",
        ],
        A::ProjectPm => [
            "Posted status update for Project Atlas (on track).",
            "Flagged 1 dependency at risk; owner pinged.",
            "Captured 3 action items from yesterday's standup.",
        ],
        _ => [
            "Completed the scheduled task with no exceptions.",
            "Processed daily queue; 2 items needed manual review.",
            "Drafted summary; ready for operator approval.",
        ],
    };
    t.iter().map(|s| (*s).to_string()).collect()
}

pub(super) async fn emit_audit(
    store: &Arc<SessionStore>, req: &str, endpoint: &'static str,
    worker_id: &WorkerId, item_count: usize,
) {
    let payload = json!({
        "endpoint": endpoint, "worker_id": worker_id.to_string(), "items": item_count,
    });
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink.write(None, AuditKind::HttpRequest, payload, principal_id_for_audit(req)).await {
        tracing::warn!(target: "gateway::duties", endpoint = %endpoint, error = %e, "duty audit write failed");
    }
}

fn gone_response(worker_id: WorkerId, archived_at: &DateTime<Utc>) -> (&'static str, String) {
    ("410 Gone", json!({
        "error": { "code": "gone", "message": format!("worker {worker_id} is archived") },
        "archived_at": archived_at.to_rfc3339(),
        "recoverable_until_at": (*archived_at + chrono::Duration::days(30)).to_rfc3339(),
    }).to_string())
}

pub(super) fn err(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (status, json!({ "error": { "code": code, "message": message } }).to_string())
}
