//! `POST /api/v1/orchestration/team-runs` — kick off a Supervisor
//! team run (parallel fan-out + dependency layers over the existing
//! handoff/orchestration-run rows). Supervisor model, DEC-022; design
//! plan 02 phases 1+2. The engine lives in `argo-orchestrator`
//! ([`argo_orchestrator::Supervisor`]); this file is the thin gateway
//! adapter: parse body → build [`TeamPlan`] → run → JSON result.
//!
//! Each child delegation + the lead's synthesis persist as
//! `provider='orchestrator'` session rows (the existing table the
//! `/orchestration` dashboard reads), so a team run's children appear
//! in `GET /api/v1/orchestration/runs` immediately. No new
//! migration / table.
use argo_orchestrator::{
    DelegationStep, OrchestratorStore, PersistingExecutor, StepId, Supervisor, TeamPlan,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;

use super::body_from_request;

/// Handle a `team-runs` POST. Returns `202 Accepted` with the
/// per-step outcomes + the lead synthesis run id on success.
pub(super) async fn create_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match resolve_tenant(req, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let body = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let plan = match build_plan(&body) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let handoff_id = body
        .get("current_handoff_id")
        .and_then(Value::as_str)
        .map(str::to_owned);

    let orch = Arc::new(OrchestratorStore::new(Arc::clone(store)));
    let persist = Arc::new(PersistingExecutor::new(orch));
    // DEC-023 P3: when `ARGO_ORCH_LLM_EXECUTOR=1` (+ a resolvable
    // provider manifest) each delegated step's `detail` is REAL model
    // output via `RuntimeStepExecutor`; default OFF keeps the row writes
    // deterministic. Both executors persist the SAME tenant-scoped
    // `[team:<id>]` row tree — only the `detail` text differs — so
    // topology + audit stay identical across the flag.
    let run = if super::routes_orchestration_team_llm::llm_executor_enabled() {
        let provider = super::routes_orchestration_team_llm::resolve_provider_id();
        let executor = Arc::new(
            super::routes_orchestration_team_llm::RuntimeStepExecutor::new(
                persist,
                provider,
                Some(Arc::clone(store)),
            ),
        );
        Supervisor::new(executor).run_team(&plan, tenant, handoff_id).await
    } else {
        Supervisor::new(persist).run_team(&plan, tenant, handoff_id).await
    };
    match run {
        Ok(result) => ("202 Accepted", render_result(&plan, &result)),
        Err(e) => server(&format!("team run failed: {e}")),
    }
}

/// Build a [`TeamPlan`] from the request body, validating shape.
fn build_plan(body: &Value) -> Result<TeamPlan, (&'static str, String)> {
    let lead_raw = body
        .get("lead")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("`lead` (worker id string) is required"))?;
    let lead = parse_worker(lead_raw)?;
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("team run")
        .to_string();
    let raw_steps = body
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("`steps` (non-empty array) is required"))?;
    if raw_steps.is_empty() {
        return Err(bad("`steps` must not be empty"));
    }
    let mut steps = Vec::with_capacity(raw_steps.len());
    for (i, raw) in raw_steps.iter().enumerate() {
        steps.push(build_step(raw, i)?);
    }
    let plan = TeamPlan::new(lead, title, steps);
    // Validate the dependency graph up front so a bad plan returns a
    // 400 rather than surfacing mid-run.
    plan.layers().map_err(|e| bad(&format!("invalid plan: {e}")))?;
    Ok(plan)
}

/// Build one [`DelegationStep`] from a `steps[]` entry.
fn build_step(raw: &Value, idx: usize) -> Result<DelegationStep, (&'static str, String)> {
    let id = raw
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(StepId::new)
        .ok_or_else(|| bad(&format!("step[{idx}].id (non-empty string) is required")))?;
    let to_raw = raw
        .get("to")
        .and_then(Value::as_str)
        .ok_or_else(|| bad(&format!("step[{idx}].to (worker id) is required")))?;
    let to = parse_worker(to_raw)?;
    let task = raw
        .get("task")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| bad(&format!("step[{idx}].task (non-empty string) is required")))?
        .to_string();
    let depends_on = raw
        .get("depends_on")
        .and_then(Value::as_array)
        .map(|deps| {
            deps.iter()
                .filter_map(Value::as_str)
                .map(StepId::new)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(DelegationStep::new(id, to, task).with_depends_on(depends_on))
}

/// Render the [`TeamResult`](argo_orchestrator::TeamResult) as the
/// wire envelope.
fn render_result(plan: &TeamPlan, result: &argo_orchestrator::TeamResult) -> String {
    let steps: Vec<Value> = result
        .outcomes
        .iter()
        .map(|o| {
            json!({
                "stepId": o.step_id.as_str(),
                "status": status_label(o.status),
                "detail": o.detail,
                "runId": o.run_id,
            })
        })
        .collect();
    json!({
        "ok": true,
        "lead": plan.lead.to_string(),
        "title": plan.title,
        "synthesisRunId": result.synthesis_run_id,
        "summary": {
            "total": result.outcomes.len(),
            "completed": result.completed_count(),
            "allCompleted": result.all_completed(),
        },
        "steps": steps,
    })
    .to_string()
}

fn status_label(s: argo_orchestrator::StepStatus) -> &'static str {
    match s {
        argo_orchestrator::StepStatus::Completed => "completed",
        argo_orchestrator::StepStatus::Failed => "failed",
        argo_orchestrator::StepStatus::Skipped => "skipped",
    }
}

fn parse_worker(raw: &str) -> Result<argo_identity::WorkerId, (&'static str, String)> {
    argo_identity::WorkerId::from_str(raw.trim())
        .map_err(|e| bad(&format!("invalid worker id '{raw}': {e}")))
}

fn parse_body(req: &str) -> Result<Value, (&'static str, String)> {
    let body = body_from_request(req);
    if body.trim().is_empty() {
        return Err(bad("request body is required"));
    }
    serde_json::from_str(body).map_err(|e| bad(&format!("parse body: {e}")))
}

/// Resolve the tenant from an explicit `?tenant_id=` query param (so
/// callers / tests can target a tenant directly) or fall back to the
/// cwd workspace — mirrors `routes_orchestration::resolve_tenant`.
async fn resolve_tenant(
    req: &str,
    store: &SessionStore,
) -> Result<TenantId, (&'static str, String)> {
    if let Some(raw) = query_value(req, "tenant_id") {
        return uuid::Uuid::parse_str(&raw)
            .map(TenantId)
            .map_err(|e| bad(&format!("invalid tenant_id: {e}")));
    }
    // Identity-first (WS1): signed-in user's session-JWT tenant, cwd fallback
    // for CLI / anonymous (real-mode walk 2026-06-07 cross-tenant fix).
    super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .map_err(|e| server_msg(format!("resolve tenant: {e}")))
}

fn query_value(req: &str, key: &str) -> Option<String> {
    let head = req.strip_prefix("POST ").unwrap_or(req).split_once(' ')?.0;
    let raw = head.split_once('?')?.1;
    raw.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

fn bad(msg: &str) -> (&'static str, String) {
    ("400 Bad Request", json!({ "error": msg }).to_string())
}

fn server(msg: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": msg }).to_string(),
    )
}

fn server_msg(msg: String) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": msg }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_orchestration_team_tests.rs"]
mod tests;
