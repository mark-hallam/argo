//! `GET /api/v1/workers/<id>/scorecard` — goal-vs-actual scorecard,
//! and `GET /api/v1/team/scorecard` — the team "goals hit this week"
//! rollup.
//!
//! Where the worker-overview KPI bullets count generic activity
//! (messages, tool-calls) and silently drop any metric with no audit
//! source, the scorecard joins a vertical's declarative KPI
//! **targets** (`VerticalManifest::all_kpis`) with **actuals**
//! measured from the tenant-scoped audit trail and reports
//! `target` + `attainment_pct` + `status` per goal.
//!
//! Tenant scoping mirrors `routes_worker_overview`: the tenant is
//! resolved via [`super::resolve_tenant::resolve_tenant_for_request`]
//! and a worker that belongs to another tenant is indistinguishable
//! from one that does not exist (`404`). So tenant A cannot read
//! tenant B's scorecard.
//!
//! **NO THEATER**: a target whose actual cannot be measured from a
//! countable audit source yet renders `measured: false` with the note
//! "target set — measurement not wired". A number is only ever emitted
//! for a genuinely observed value. The scoring + source-resolution
//! logic lives in [`score`].

#[path = "routes_worker_scorecard_score.rs"]
mod score;

use score::{classify_source, resolve_actual, score, status_label, ScoreSource};

use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
use argo_vertical::KpiDefinition;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use std::sync::Arc;

type Resp = (&'static str, String);

/// Worker scorecard window — 14 days, matching the overview KPI
/// window so the two surfaces agree on "recent".
const WINDOW_DAYS: i64 = 14;
/// Team rollup window — the trailing 7 days ("this week").
const TEAM_WINDOW_DAYS: i64 = 7;

pub(super) async fn show_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Resp {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match build_scorecard(store, worker_id_raw, req).await {
        Ok(ok) | Err(ok) => ok,
    }
}

async fn build_scorecard(
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
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .map_err(|e| ise(&format!("resolve tenant: {e}")))?;
    // Cross-tenant existence is hidden: a worker in another tenant is
    // a 404 here, identical to a worker that does not exist.
    let worker = IdentityStore::new(store)
        .get_worker(tenant, worker_id)
        .await
        .map_err(|e| ise(&format!("get worker: {e}")))?
        .ok_or_else(|| err("404 Not Found", "not_found", "worker not found"))?;

    let now = Utc::now();
    let since = (now - Duration::days(WINDOW_DAYS)).timestamp_micros();
    let targets = load_targets(&worker.vertical_id);

    let mut goals = Vec::with_capacity(targets.len());
    let (mut total, mut measured, mut met) = (0_u32, 0_u32, 0_u32);
    for kpi in &targets {
        let source = classify_source(kpi);
        let actual = resolve_actual(store, tenant, worker_id, source, since)
            .await
            .map_err(|e| ise(&format!("resolve actual: {e}")))?;
        let (entry, is_measured, is_met) = scorecard_entry(kpi, source, actual);
        total += 1;
        if is_measured {
            measured += 1;
            if is_met {
                met += 1;
            }
        }
        goals.push(entry);
    }

    emit_view_audit(store, tenant, worker_id).await;
    let body = json!({
        "worker_id": worker_id.to_string(),
        "worker_name": worker.persona.name,
        "vertical_id": worker.vertical_id,
        "window_days": WINDOW_DAYS,
        "goals": goals,
        "summary": {
            "goals_total": total,
            "goals_measured": measured,
            "goals_met": met,
        },
    });
    Ok(("200 OK", body.to_string()))
}

/// `GET /api/v1/team/scorecard` — count of goals hit this week across
/// every worker in the resolved tenant. Read-only rollup; same
/// tenant-scoping invariant (the worker list is filtered by tenant).
pub(super) async fn team_scorecard_response(req: &str, store: Option<&Arc<SessionStore>>) -> Resp {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
        Ok(t) => t,
        Err(e) => return ise(&format!("resolve tenant: {e}")),
    };
    let workers = match IdentityStore::new(store).list_workers(tenant, false).await {
        Ok(w) => w,
        Err(e) => return ise(&format!("list workers: {e}")),
    };
    let now = Utc::now();
    let since = (now - Duration::days(TEAM_WINDOW_DAYS)).timestamp_micros();

    let mut rows = Vec::with_capacity(workers.len());
    let (mut team_met, mut team_measured) = (0_u32, 0_u32);
    for worker in &workers {
        let (w_met, w_measured) =
            match worker_goal_tally(store, tenant, worker.id, &worker.vertical_id, since).await {
                Ok(t) => t,
                Err(e) => return ise(&format!("worker tally: {e}")),
            };
        team_met += w_met;
        team_measured += w_measured;
        rows.push(json!({
            "worker_id": worker.id.to_string(),
            "worker_name": worker.persona.name,
            "vertical_id": worker.vertical_id,
            "goals_met": w_met,
            "goals_measured": w_measured,
        }));
    }
    let body = json!({
        "window_days": TEAM_WINDOW_DAYS,
        "summary": {
            "workers": workers.len(),
            "goals_met": team_met,
            "goals_measured": team_measured,
        },
        "workers": rows,
    });
    ("200 OK", body.to_string())
}

/// Count `(met, measured)` goals for one worker over the window.
async fn worker_goal_tally(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
    vertical_id: &str,
    since: i64,
) -> argo_types::ArgoResult<(u32, u32)> {
    let (mut met, mut measured) = (0_u32, 0_u32);
    for kpi in load_targets(vertical_id) {
        let source = classify_source(&kpi);
        if let Some(actual) = resolve_actual(store, tenant, worker_id, source, since).await? {
            measured += 1;
            if score(actual, kpi.threshold, kpi.comparator).1 {
                met += 1;
            }
        }
    }
    Ok((met, measured))
}

/// Load the declarative KPI targets for a vertical. Returns an empty
/// list when the verticals dir cannot be read or the id is unknown —
/// the scorecard then renders an empty goal list (no fabrication).
fn load_targets(vertical_id: &str) -> Vec<KpiDefinition> {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let Ok((registry, _errs)) = argo_vertical::load_dir(&dir) else {
        return Vec::new();
    };
    registry
        .get(vertical_id)
        .map(|loaded| loaded.manifest.all_kpis())
        .unwrap_or_default()
}

/// Build one goal entry. Returns `(json, measured, met)` so the
/// caller can tally the summary without re-parsing the value.
fn scorecard_entry(kpi: &KpiDefinition, source: ScoreSource, actual: Option<f64>) -> (Value, bool, bool) {
    let label = kpi
        .description
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| humanize(&kpi.name));
    let target = json!({
        "value": kpi.threshold,
        "comparator": kpi.comparator,
        "display": format!(
            "{} {}",
            score::comparator_symbol(kpi.comparator),
            score::format_value(kpi.kind, kpi.threshold),
        ),
    });
    match actual {
        Some(v) => {
            let (pct, met) = score(v, kpi.threshold, kpi.comparator);
            let entry = json!({
                "name": kpi.name,
                "label": label,
                "kind": kpi.kind,
                "gate": kpi.gate,
                "target": target,
                "source": source.label(),
                "measured": true,
                "actual": { "value": v, "display": score::format_value(kpi.kind, v) },
                "attainment_pct": pct,
                "status": status_label(met),
            });
            (entry, true, met)
        }
        None => {
            let entry = json!({
                "name": kpi.name,
                "label": label,
                "kind": kpi.kind,
                "gate": kpi.gate,
                "target": target,
                "source": source.label(),
                "measured": false,
                "note": "target set — measurement not wired",
            });
            (entry, false, false)
        }
    }
}

/// Turn a snake_case KPI name into a Title-ish label when the manifest
/// gives no description.
fn humanize(name: &str) -> String {
    let spaced = name.replace(['_', '-'], " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
        None => spaced,
    }
}

async fn emit_view_audit(store: &Arc<SessionStore>, tenant: TenantId, worker_id: WorkerId) {
    let payload = json!({
        "endpoint": "worker.scorecard.viewed",
        "tenant_id": tenant.to_string(),
        "worker_id": worker_id.to_string(),
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink.write(None, AuditKind::HttpRequest, payload, None).await {
        tracing::warn!(target: "gateway::worker_scorecard", error = %e,
            "worker.scorecard.viewed audit write failed");
    }
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
#[path = "routes_worker_scorecard_tests.rs"]
mod tests;
