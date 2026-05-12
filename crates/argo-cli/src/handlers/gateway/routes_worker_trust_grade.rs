//! `GET /api/v1/workers/<id>/trust-grade` — informational trust-grade
//! surface (mission-trust-grade-ui).
//!
//! Read-only HTTP shim over `IdentityStore::get_worker`. Returns the
//! current trust grade (mapped from `worker.trust_grade` u8 to the
//! canonical [`argo_skill::TrustGrade`] enum names), the signals
//! contributing to it, and the criteria gap to the next rung. The UI
//! is purely informational — there is intentionally no edit
//! affordance.
//!
//! ## Mapping (`u8` → `TrustGrade`)
//!
//! `worker.trust_grade` is a `u8` (default `0` = fresh / unknown).
//! Higher values are more trusted. Mapping to the
//! `argo_skill::trust::TrustGrade` enum is inverse to the enum's
//! `Ord` order because TrustGrade's variant index goes from MOST
//! trusted (Builtin=0) to LEAST (LearnedUnreviewed=4), while the
//! worker `u8` field is the opposite direction. We surface enum
//! names rather than raw integers so the SPA never has to interpret
//! the underlying scale.
//!
//! | `u8` | TrustGrade           | UI tone     |
//! |------|----------------------|-------------|
//! | 0    | LearnedUnreviewed    | destructive |
//! | 1    | PublishedCommunity   | warning     |
//! | 2    | Tenant               | warning     |
//! | 3    | PublishedVerified    | success     |
//! | 4+   | Builtin              | success     |
//!
//! ## Sensible deviations from the brief
//!
//! - The brief proposed display labels Bootstrap / Reviewed / Tenant
//!   / Verified / Published-Verified, but the canonical enum names
//!   are `argo_skill::TrustGrade` variants. We surface those names
//!   so the API matches the underlying type. Frontend can re-label
//!   for human-friendly display.
//! - The brief listed signals `action_count`, `success_rate`,
//!   `refused_count`, `time_since_first_action`. The current core
//!   has no per-worker action tally surfaced through the identity
//!   store — building one would require touching the
//!   intentionally-out-of-scope evaluation logic. We surface the
//!   signals that ARE genuinely observable from `worker`:
//!   `trust_grade_value`, `age_days`, `last_active_days_ago`,
//!   `has_been_used`. The criteria array is shaped to convey the
//!   gap honestly (current_value/target_value/met).
//! - Bearer auth + principal-header parsing are gated upstream by
//!   `routes_dispatch_get`; this handler trusts both, matching
//!   `routes_workers_hire` and `routes_team_list`.

use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_skill::TrustGrade;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;

/// `GET /api/v1/workers/<id>/trust-grade` entry point. `worker_id_raw`
/// is the path segment between `/workers/` and `/trust-grade`; it
/// must parse as a [`WorkerId`].
pub(super) async fn show_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let worker_id = match worker_id_raw.parse::<WorkerId>() {
        Ok(id) => id,
        Err(e) => return bad_request(&format!("invalid worker_id '{worker_id_raw}': {e}")),
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let identities = IdentityStore::new(store);
    let worker = match identities.get_worker(tenant, worker_id).await {
        Ok(Some(w)) => w,
        Ok(None) => return not_found(&format!("worker {worker_id} not found")),
        Err(e) => return server_error(&format!("get worker: {e}")),
    };

    let now = Utc::now();
    let body = build_trust_grade_response(&worker, now);
    emit_audit(store, req, tenant, worker_id).await;
    ("200 OK", body.to_string())
}

/// Pure assembler — given a worker snapshot and a clock, produce the
/// JSON envelope. Kept separate so tests can drive it without a
/// store at all.
pub(super) fn build_trust_grade_response(worker: &Worker, now: DateTime<Utc>) -> Value {
    let current = grade_for_u8(worker.trust_grade);
    let next = next_grade(worker.trust_grade);
    let signals = build_signals(worker, now);
    let criteria = build_criteria(worker, next);
    json!({
        "worker_id": worker.id.to_string(),
        "current_grade": serde_json::to_value(current).unwrap_or(Value::Null),
        "current_grade_label": grade_label(current),
        "current_grade_tone": grade_tone(current),
        "current_grade_index": worker.trust_grade,
        "next_grade": next.map(|g| serde_json::to_value(g).unwrap_or(Value::Null)),
        "next_grade_label": next.map(grade_label),
        "at_max_grade": next.is_none(),
        "signals": signals,
        "next_grade_criteria": criteria,
    })
}

/// Map worker.trust_grade (u8 — higher = more trusted) to the
/// canonical [`TrustGrade`] enum. Values above the top rung saturate
/// at `Builtin`.
pub(super) fn grade_for_u8(g: u8) -> TrustGrade {
    match g {
        0 => TrustGrade::LearnedUnreviewed,
        1 => TrustGrade::PublishedCommunity,
        2 => TrustGrade::Tenant,
        3 => TrustGrade::PublishedVerified,
        _ => TrustGrade::Builtin,
    }
}

/// The next rung above the current `u8`, or `None` if already at the
/// max (`Builtin`).
pub(super) fn next_grade(g: u8) -> Option<TrustGrade> {
    if g >= 4 {
        return None;
    }
    Some(grade_for_u8(g + 1))
}

/// Human-friendly display label. The SPA uses the raw `current_grade`
/// snake_case string for logic and this label for the badge.
pub(super) fn grade_label(g: TrustGrade) -> &'static str {
    match g {
        TrustGrade::Builtin => "Builtin",
        TrustGrade::PublishedVerified => "Published verified",
        TrustGrade::Tenant => "Tenant",
        TrustGrade::PublishedCommunity => "Published community",
        TrustGrade::LearnedUnreviewed => "Learned, unreviewed",
    }
}

/// Tone token — drives the badge colour family. Matches the
/// approval-card severity vocabulary (success / warning /
/// destructive). The frontend resolves these against the active
/// theme's CSS variables; no hard-coded hex here.
pub(super) fn grade_tone(g: TrustGrade) -> &'static str {
    match g {
        TrustGrade::Builtin | TrustGrade::PublishedVerified => "success",
        TrustGrade::Tenant | TrustGrade::PublishedCommunity => "warning",
        TrustGrade::LearnedUnreviewed => "destructive",
    }
}

fn build_signals(worker: &Worker, now: DateTime<Utc>) -> Value {
    let last_active_days = worker
        .last_used_at
        .map(|ts| ((now - ts).num_seconds().max(0)) / 86_400);
    json!({
        "trust_grade_value": worker.trust_grade,
        "has_been_used": worker.last_used_at.is_some(),
        "last_active_days_ago": last_active_days,
    })
}

fn build_criteria(worker: &Worker, next: Option<TrustGrade>) -> Value {
    let Some(next) = next else {
        return Value::Array(Vec::new());
    };
    let used = worker.last_used_at.is_some();
    let recent = worker
        .last_used_at
        .is_some_and(|ts| (Utc::now() - ts).num_days() <= 30);
    // Generic, honest criteria. The criteria-met markers are
    // computed from what the IdentityStore exposes — anything richer
    // would require touching the evaluation logic this brief is not
    // permitted to amend.
    let rows = vec![
        criterion(
            "Worker active in the last 30 days",
            if recent { "yes" } else { "no" },
            "yes",
            recent,
        ),
        criterion(
            "Has executed at least one action",
            if used { "yes" } else { "no" },
            "yes",
            used,
        ),
        criterion(
            &format!("Operator-promoted to {}", grade_label(next)),
            if worker.trust_grade as u32 >= grade_for_u8_index(next) {
                "yes"
            } else {
                "pending"
            },
            "yes",
            worker.trust_grade as u32 >= grade_for_u8_index(next),
        ),
    ];
    Value::Array(rows)
}

/// Inverse of [`grade_for_u8`] — returns the `u8` ladder index for a
/// given grade, used by the criteria builder to check "have we
/// reached the next rung".
fn grade_for_u8_index(g: TrustGrade) -> u32 {
    match g {
        TrustGrade::LearnedUnreviewed => 0,
        TrustGrade::PublishedCommunity => 1,
        TrustGrade::Tenant => 2,
        TrustGrade::PublishedVerified => 3,
        TrustGrade::Builtin => 4,
    }
}

fn criterion(name: &str, current: &str, target: &str, met: bool) -> Value {
    json!({
        "name": name,
        "current_value": current,
        "target_value": target,
        "met": met,
    })
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
    req: &str,
    tenant: TenantId,
    worker_id: WorkerId,
) {
    let actor = principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string());
    let payload = json!({
        "endpoint": "worker.trust_grade.queried",
        "actor": actor,
        "tenant_id": tenant.to_string(),
        "worker_id": worker_id.to_string(),
    });
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, Some(actor.clone()))
        .await
    {
        tracing::warn!(
            target: "gateway::worker_trust_grade",
            error = %e,
            actor = %actor,
            "worker.trust_grade.queried audit write failed"
        );
    }
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
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

#[cfg(test)]
#[path = "routes_worker_trust_grade_tests.rs"]
mod tests;
