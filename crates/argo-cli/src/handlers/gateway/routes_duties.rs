//! Per-worker duty endpoints (Stream J13): classify, commit, preview,
//! PATCH, DELETE under `/api/v1/workers/<id>/duties/`. Wired via
//! `dispatch_duties` in `routes_dispatch_post`. Sensible deviations
//! (hard delete, label→description, 5-field wire/6-field disk cron) are
//! documented in the PR body.

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_genesis::{classify_charter, Archetype};
use argo_identity::{IdentityStore, WorkerId};
use argo_orchestrator::{DutyRecord, DutyStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;

const MIN_LABEL_LEN: usize = 4;
const MAX_LABEL_LEN: usize = 100;
const NEXT_FIRES_LOOKAHEAD: usize = 10;
const CONFLICT_WINDOW_SECS: i64 = 60;
const PREVIEW_DEFAULT_SAMPLES: usize = 3;
const PREVIEW_MAX_SAMPLES: usize = 3;

pub(super) async fn classify_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, worker_id, _) = match resolve_worker(worker_id_raw, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let parsed = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let input = match parsed.get("input").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return err("400 Bad Request", "bad_request", "`input` (non-empty string) is required"),
    };
    let archetype = classify_charter(&input).unwrap_or(Archetype::OpsOnCall);
    let defaults = defaults_for(archetype);
    let proposed_5 = parsed
        .get("cron_hint").and_then(Value::as_str).map(str::trim)
        .filter(|s| !s.is_empty()).map_or(defaults.cron, |s| s).to_string();
    let stored_cron = match validate_5_field_cron(&proposed_5) {
        Ok(s) => s,
        Err(msg) => return err("400 Bad Request", "bad_request", &msg),
    };
    let duty_store = DutyStore::new(Arc::clone(store));
    let conflicts = match collect_conflicts(&duty_store, &worker_id, &stored_cron).await {
        Ok(v) => v,
        Err(e) => return err("500 Internal Server Error", "internal_error", &format!("conflicts probe: {e}")),
    };
    let samples = sample_outputs_for(archetype);
    let body = json!({
        "duty_proposal": {
            "label": defaults.label, "schedule": proposed_5, "cron_stored": stored_cron,
            "entry_action": defaults.entry_action, "archetype": archetype.slug(),
        },
        "sample_outputs": samples,
        "conflicts": conflicts,
    });
    emit_audit(store, req, "duty.classify", &worker_id, samples.len()).await;
    ("200 OK", body.to_string())
}

pub(super) async fn commit_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, worker_id, tenant) = match resolve_worker(worker_id_raw, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let parsed = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let proposal = match parsed.get("duty_proposal") {
        Some(v) if v.is_object() => v,
        _ => return err("400 Bad Request", "bad_request", "`duty_proposal` (object) is required"),
    };
    let label = match validate_label(proposal.get("label").and_then(Value::as_str).unwrap_or("")) {
        Ok(s) => s,
        Err(msg) => return err("400 Bad Request", "bad_request", &msg),
    };
    let schedule_raw = proposal.get("schedule").and_then(Value::as_str).unwrap_or("");
    let stored_cron = match validate_5_field_cron(schedule_raw) {
        Ok(s) => s,
        Err(msg) => return err("400 Bad Request", "bad_request", &msg),
    };
    let entry_action = proposal
        .get("entry_action").and_then(Value::as_str).map(str::trim)
        .filter(|s| !s.is_empty()).map_or_else(|| "ping".to_string(), str::to_string);
    let duty_store = DutyStore::new(Arc::clone(store));
    let resolution = parsed.get("accepted_conflict_resolution").and_then(Value::as_str).unwrap_or("");
    let existing_conflicts = match collect_conflicts(&duty_store, &worker_id, &stored_cron).await {
        Ok(v) => v,
        Err(e) => return err("500 Internal Server Error", "internal_error", &format!("conflicts probe: {e}")),
    };
    if !existing_conflicts.is_empty() {
        match resolution {
            "cancel" => return err("409 Conflict", "duty_conflict", "commit cancelled by conflict resolution"),
            "replace" => if let Err(e) = delete_conflicting(store, &existing_conflicts).await {
                return err("500 Internal Server Error", "internal_error", &format!("replace conflicts: {e}"));
            },
            "keep_both" => {}
            _ => return err("409 Conflict", "duty_conflict", "duty cron overlaps existing duties; supply `accepted_conflict_resolution` (replace|keep_both|cancel)"),
        }
    }
    let duty = DutyRecord::new(tenant, worker_id, stored_cron.clone(), entry_action, label);
    if let Err(e) = duty_store.insert(&duty).await {
        return err("500 Internal Server Error", "internal_error", &format!("insert duty: {e}"));
    }
    let first_fire = Schedule::from_str(&stored_cron).ok().and_then(|s| s.after(&Utc::now()).next());
    emit_audit(store, req, "duty.commit", &worker_id, 1).await;
    ("201 Created", json!({
        "duty_id": duty.id,
        "scheduled_first_fire": first_fire.map(|t| t.to_rfc3339()),
    }).to_string())
}

pub(super) async fn preview_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, _, _) = match resolve_worker(worker_id_raw, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let parsed = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let duty_id = match parsed.get("duty_id").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return err("400 Bad Request", "bad_request", "`duty_id` (string) is required"),
    };
    let sample_size = parsed.get("sample_size").and_then(Value::as_u64)
        .map_or(PREVIEW_DEFAULT_SAMPLES, |n| n as usize)
        .clamp(1, PREVIEW_MAX_SAMPLES);
    let duty_store = DutyStore::new(Arc::clone(store));
    let duty = match duty_store.get(&duty_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return err("404 Not Found", "not_found", &format!("duty '{duty_id}' not found")),
        Err(e) => return err("500 Internal Server Error", "internal_error", &format!("get duty: {e}")),
    };
    let arch = classify_charter(&duty.description).unwrap_or(Archetype::OpsOnCall);
    let outputs = sample_outputs_for(arch);
    let samples: Vec<Value> = outputs.iter().take(sample_size).enumerate()
        .map(|(idx, out)| json!({
            "inbound_context": format!("synthesized fixture #{}", idx + 1),
            "would_send": out,
        }))
        .collect();
    emit_audit(store, req, "duty.preview", &duty.worker_id, samples.len()).await;
    ("200 OK", json!({ "samples": samples }).to_string())
}

pub(super) async fn patch_response(
    worker_id_raw: &str,
    duty_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, worker_id, _) = match resolve_worker(worker_id_raw, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let duty_id = duty_id_raw.split('?').next().unwrap_or(duty_id_raw);
    if duty_id.is_empty() {
        return err("400 Bad Request", "bad_request", "missing duty_id");
    }
    let parsed = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let duty_store = DutyStore::new(Arc::clone(store));
    let existing = match duty_store.get(duty_id).await {
        Ok(Some(d)) if d.worker_id == worker_id => d,
        Ok(_) => return err("404 Not Found", "not_found", &format!("duty '{duty_id}' not found")),
        Err(e) => return err("500 Internal Server Error", "internal_error", &format!("get duty: {e}")),
    };
    let new_cron = match parsed.get("schedule").and_then(Value::as_str) {
        Some(s) => match validate_5_field_cron(s) {
            Ok(stored) => Some(stored),
            Err(msg) => return err("400 Bad Request", "bad_request", &msg),
        },
        None => None,
    };
    let new_label = match parsed.get("label").and_then(Value::as_str) {
        Some(s) => match validate_label(s) {
            Ok(v) => Some(v),
            Err(msg) => return err("400 Bad Request", "bad_request", &msg),
        },
        None => None,
    };
    let new_enabled = parsed.get("enabled").and_then(Value::as_bool);
    if new_cron.is_none() && new_label.is_none() && new_enabled.is_none() {
        return err("400 Bad Request", "bad_request", "at least one of `enabled`, `schedule`, `label` is required");
    }
    if let Err(e) = apply_patch(store, duty_id, new_cron.as_deref(), new_label.as_deref(), new_enabled).await {
        return err("500 Internal Server Error", "internal_error", &format!("update duty: {e}"));
    }
    emit_audit(store, req, "duty.patch", &worker_id, 1).await;
    let refreshed = duty_store.get(duty_id).await.ok().flatten().unwrap_or(existing);
    ("200 OK", json!({ "duty": duty_to_json(&refreshed) }).to_string())
}

pub(super) async fn delete_response(
    worker_id_raw: &str,
    duty_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let (store, worker_id, _) = match resolve_worker(worker_id_raw, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let duty_id = duty_id_raw.split('?').next().unwrap_or(duty_id_raw);
    if duty_id.is_empty() {
        return err("400 Bad Request", "bad_request", "missing duty_id");
    }
    let duty_store = DutyStore::new(Arc::clone(store));
    match duty_store.get(duty_id).await {
        Ok(Some(d)) if d.worker_id == worker_id => {}
        Ok(_) => return err("404 Not Found", "not_found", &format!("duty '{duty_id}' not found")),
        Err(e) => return err("500 Internal Server Error", "internal_error", &format!("get duty: {e}")),
    }
    if let Err(e) = sqlx::query("DELETE FROM role_duty WHERE id = ?")
        .bind(duty_id).execute(store.pool()).await
    {
        return err("500 Internal Server Error", "internal_error", &format!("delete duty: {e}"));
    }
    emit_audit(store, req, "duty.delete", &worker_id, 1).await;
    ("204 No Content", String::new())
}

fn parse_body(req: &str) -> Result<Value, (&'static str, String)> {
    let body = body_from_request(req);
    if body.trim().is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    serde_json::from_str(body).map_err(|e| err("400 Bad Request", "bad_request", &format!("parse body: {e}")))
}

async fn resolve_worker<'a>(
    worker_id_raw: &str,
    store: Option<&'a Arc<SessionStore>>,
) -> Result<(&'a Arc<SessionStore>, WorkerId, TenantId), (&'static str, String)> {
    let Some(store) = store else { return Err(super::store_unavailable()) };
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
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
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

fn validate_5_field_cron(input: &str) -> Result<String, String> {
    let fields: Vec<&str> = input.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(format!("cron must be 5 fields, got {}", fields.len()));
    }
    let six = format!("0 {}", fields.join(" "));
    Schedule::from_str(&six).map_err(|e| format!("invalid cron: {e}"))?;
    Ok(six)
}

fn validate_label(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if !(MIN_LABEL_LEN..=MAX_LABEL_LEN).contains(&trimmed.len()) {
        return Err(format!("`label` must be {MIN_LABEL_LEN}..={MAX_LABEL_LEN} chars"));
    }
    Ok(trimmed.to_string())
}

fn next_fires(stored_cron: &str, now: DateTime<Utc>, n: usize) -> Vec<DateTime<Utc>> {
    Schedule::from_str(stored_cron).ok().map(|s| s.after(&now).take(n).collect()).unwrap_or_default()
}

async fn collect_conflicts(
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

async fn delete_conflicting(store: &SessionStore, conflicts: &[Value]) -> Result<(), sqlx::Error> {
    for c in conflicts {
        if let Some(id) = c.get("duty_id").and_then(Value::as_str) {
            sqlx::query("DELETE FROM role_duty WHERE id = ?")
                .bind(id).execute(store.pool()).await?;
        }
    }
    Ok(())
}

async fn apply_patch(
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

fn duty_to_json(d: &DutyRecord) -> Value {
    json!({
        "id": d.id, "worker_id": d.worker_id.to_string(), "cron": d.cron,
        "entry_action": d.entry_action, "label": d.description, "enabled": d.enabled,
        "paused_until": d.paused_until.map(|t| t.to_rfc3339()),
        "last_fired_at": d.last_fired_at.map(|t| t.to_rfc3339()),
        "created_at": d.created_at.to_rfc3339(),
    })
}

struct DutyDefaults { label: &'static str, cron: &'static str, entry_action: &'static str }

fn defaults_for(a: Archetype) -> DutyDefaults {
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

fn sample_outputs_for(a: Archetype) -> Vec<String> {
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

async fn emit_audit(
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

fn err(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (status, json!({ "error": { "code": code, "message": message } }).to_string())
}

#[cfg(test)]
#[path = "routes_duties_tests.rs"]
mod tests;
