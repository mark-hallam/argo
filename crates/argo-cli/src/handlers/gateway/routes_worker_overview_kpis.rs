//! KPI bullets, sparklines, recent-activity rows, and duty schedule
//! helpers for the J8 worker-overview bundle. Split from the parent
//! `routes_worker_overview` module so each file stays under the
//! 500-LOC ceiling; routing, auth, and response wrappers remain in the
//! parent.

use super::micros_to_utc;
use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_team_headline::{bullets_for_vertical, ActivityCounts, BulletKind};
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use cron::Schedule;
use serde_json::{json, Value};
use sqlx::Row;
use std::str::FromStr;

const SPARKLINE_DAYS: usize = 7;
const KPI_WINDOW_DAYS: i64 = 14;
const RECENT_LIMIT: usize = 5;

#[derive(Debug, Clone)]
pub(super) struct AuditRow {
    pub id: String,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub payload: Value,
}

pub(super) async fn fetch_audit_rows(store: &SessionStore, worker_id: WorkerId, now: DateTime<Utc>) -> ArgoResult<Vec<AuditRow>> {
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

pub(super) fn compute_kpis(vertical_id: &str, audit: &[AuditRow], now: DateTime<Utc>) -> Vec<Value> {
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

pub(super) fn recent_activity_rows(audit: &[AuditRow]) -> Vec<Value> {
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

pub(super) async fn duties_for_worker(
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
