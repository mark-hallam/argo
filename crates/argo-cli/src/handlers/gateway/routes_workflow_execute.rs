//! `mission-workflow-execute-route` — wires the SPA's "Run workflow"
//! button to a real server-side execution path. Promotes
//! mission-claim-3 from `LIVE-PENDING-HTTP-ROUTE` to LIVE (audit
//! 2026-05-23T03-39 §2 P1 #4).
//!
//! ## Route
//!
//! ```text
//! POST /api/v1/verticals/{vertical_id}/workflows/{name}/execute
//! ```
//!
//! Body (`application/json`):
//!
//! ```json
//! { "inputs": { /* optional caller-supplied inputs */ },
//!   "idempotency_key": "optional-string" }
//! ```
//!
//! ## Sensible deviations from the brief
//!
//! 1. **Path shape.** The brief named `POST /api/v1/workflows/:id/
//!    execute`. The codebase already keys workflow surfaces by
//!    `(vertical_id, name)` (see `routes_workflows`, `routes_workflow_
//!    runs`, `routes_workflows_test_runs`), so the route mirrors the
//!    sibling pattern instead of inventing a parallel `id`-keyed
//!    surface. The SPA's workflow detail page is already on
//!    `/workflows/{verticalId}/{name}`.
//! 2. **Crate location.** The brief named `crates/argo-gateway/`; no
//!    such crate exists. Gateway code lives under
//!    `crates/argo-cli/src/handlers/gateway/`.
//! 3. **Runner choice (v1).** The brief calls for `WorkerSessionRunner`.
//!    That runner needs production trait impls of `ToolDispatcher` /
//!    `WorkflowAuditEmitter` / `WorkflowCapabilityChecker` adapting
//!    `argo_mcp` / `argo_audit` / `argo_capability`. Those adapter
//!    impls aren't on `origin/main` yet. v1 ships `MockRunner::echo`
//!    (same runner the CLI handler at `argo-cli/handlers/workflow.rs`
//!    uses) so the audit chain + workflow_run persistence land
//!    correctly for the demo. A follow-up brief swaps in
//!    `WorkerSessionRunner` once the adapters land.
//!
//! ## Authorization model
//!
//! Mirrors `routes_workflows`: anonymous callers (no
//! `X-Argo-Principal`) get `401 Unauthorized`. Authenticated callers
//! pass through `Action::WorkflowEdit` (Admin+) + the
//! `workflow_edit_capability()` engine check. Cross-tenant lookups
//! return `404 Not Found` with the same shape as a genuine miss so
//! existence cannot be probed (per #1992 / `routes_workflows::
//! fetch_version`).
//!
//! ## Audit
//!
//! One `WorkflowExecuteDispatched` row per accepted POST. Refused
//! requests (401 / 403 / 404 / 409) do not emit. Payload:
//! `{ actor, tenant_id, vertical_id, workflow_name, run_id, ok,
//! idempotency_key, dispatched_at }`.

use super::role_gate::{enforce_role, resolve_role, Action};
use super::routes_workflows::{bad_request, forbidden, internal, not_found, unauthorized};
use argo_audit::{AuditKind, AuditSink};
use argo_capability::{workflow_edit_capability, CapabilityEngine};
use argo_identity::user_tenant::UserTenantRole;
use argo_store::workflow_definition::{self as wd, WorkflowDefinition};
use argo_store::SessionStore;
use argo_types::{Capability, Principal, TenantId};
use argo_workflow::{ExecutionRequest, Executor, MockRunner, TaskNode, Workflow};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

const PREFIX: &str = "/api/v1/verticals/";
const SUFFIX: &str = "execute";
const PRINCIPAL_HEADER: &str = "X-Argo-Principal";

/// POST dispatcher. Returns `Some(resp)` when the request matches
/// `POST /api/v1/verticals/{vid}/workflows/{name}/execute`; `None`
/// otherwise so the parent dispatcher can fall through to its other
/// arms.
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let (vertical, name) = parse_execute_path(req)?;
    let store = match store {
        Some(s) => s,
        None => return Some(super::store_unavailable()),
    };
    Some(handle_post(req, store, vertical, name).await)
}

fn parse_execute_path(req: &str) -> Option<(&str, &str)> {
    let after_verb = req.strip_prefix("POST ")?;
    let path_with_query = after_verb.split_whitespace().next()?;
    let path = path_with_query.split('?').next()?;
    let rest = path.strip_prefix(PREFIX)?;
    let mut iter = rest.splitn(2, '/');
    let vertical = iter.next().filter(|s| !s.is_empty())?;
    let tail = iter.next()?;
    let tail = tail.strip_prefix("workflows/")?;
    let mut tail_iter = tail.splitn(2, '/');
    let name = tail_iter.next().filter(|s| !s.is_empty())?;
    let suffix = tail_iter.next()?;
    if suffix != SUFFIX {
        return None;
    }
    Some((vertical, name))
}

async fn handle_post(
    req: &str,
    store: &Arc<SessionStore>,
    vertical: &str,
    name: &str,
) -> (&'static str, String) {
    if let Some(resp) = enforce_role(req, Action::WorkflowEdit) {
        return resp;
    }
    let (principal, role) = match resolve_principal(req) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    if let Err(resp) = check_capability(&principal, role, &workflow_edit_capability()) {
        return resp;
    }
    let tenant = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };

    // Fetch the live head of the workflow definition. Cross-tenant
    // and genuine-miss collapse to the same 404 shape so existence
    // cannot be probed (per `routes_workflows::fetch_head`).
    let head = match wd::get_head(store.pool(), tenant, vertical, name).await {
        Ok(Some(def)) => def,
        Ok(None) => {
            return not_found(&format!(
                "no head workflow definition for ({vertical}, {name})"
            ))
        }
        Err(e) => return internal(&format!("get_head: {e}")),
    };

    // Caller body is optional — the SPA `Run workflow` button POSTs
    // with no body in v1. An empty body decodes as
    // `inputs={}, idempotency_key=None`.
    let body = super::body_from_request(req);
    let (_inputs, idempotency_key) = match parse_request_body(body) {
        Ok(p) => p,
        Err(msg) => return bad_request(&msg),
    };

    // Parse the stored TOML through the canonical vertical parser
    // so the route gets the same shape validation everywhere else
    // in the codebase. A parse failure here means the row was
    // written out-of-band (validate-on-write skipped) — surface 500
    // because the workflow_definition layer guarantees parse-clean
    // rows.
    let parsed = match argo_vertical::parse_workflow(&head.definition_toml) {
        Ok(w) => w,
        Err(e) => return internal(&format!("parse workflow toml: {e}")),
    };

    let actor = principal_to_actor(&principal);
    let dispatched_at = Utc::now();
    let report =
        match run_workflow_against_definition(store, &head, &parsed, dispatched_at).await {
            Ok(r) => r,
            Err(msg) => return internal(&msg),
        };

    emit_dispatch_audit(
        store,
        tenant,
        &actor,
        vertical,
        name,
        &report.run_id,
        report.ok,
        idempotency_key.as_deref(),
        dispatched_at,
    )
    .await;

    let body = json!({
        "run_id": report.run_id,
        "vertical_id": vertical,
        "workflow_name": name,
        "ok": report.ok,
        "task_verdicts": report.verdicts,
        "dispatched_at": dispatched_at.to_rfc3339(),
    });
    ("200 OK", body.to_string())
}

struct RunReport {
    run_id: String,
    ok: bool,
    verdicts: Value,
}

async fn run_workflow_against_definition(
    store: &Arc<SessionStore>,
    head: &WorkflowDefinition,
    parsed: &argo_vertical::Workflow,
    started_at: chrono::DateTime<Utc>,
) -> Result<RunReport, String> {
    let dag = vertical_workflow_to_dag(parsed);
    let exec = Executor::new(MockRunner::echo());
    let request = ExecutionRequest::new(dag);
    let exec_report = exec
        .execute(request)
        .await
        .map_err(|e| format!("execute: {e}"))?;
    let finished_at = Utc::now();
    persist_workflow_run(store, head, &exec_report, started_at, finished_at).await?;
    let verdicts = serde_json::to_value(&exec_report.task_verdicts)
        .map_err(|e| format!("verdicts json: {e}"))?;
    Ok(RunReport {
        run_id: exec_report.workflow_id.as_str().to_string(),
        ok: exec_report.ok,
        verdicts,
    })
}

/// Build a minimal DAG from the stored vertical-style workflow so
/// the executor can walk it with `MockRunner`. Each `[[step]]`
/// becomes one `TaskNode`; `depends_on` carries over verbatim.
/// `tool_call` steps set `inputs.tool` so future `WorkerSessionRunner`
/// dispatch resolves cleanly; `skill_invoke` + `prompt` steps stay
/// shape-compatible without a tool name (the v1 MockRunner doesn't
/// care).
fn vertical_workflow_to_dag(parsed: &argo_vertical::Workflow) -> Workflow {
    let tasks: Vec<TaskNode> = parsed
        .steps
        .iter()
        .map(|step| {
            let mut inputs: BTreeMap<String, Value> = BTreeMap::new();
            if let Some(tool) = step.tool.as_deref() {
                inputs.insert("tool".into(), Value::String(tool.to_string()));
            }
            if let Some(args) = step.args.clone() {
                inputs.insert("args".into(), args);
            }
            TaskNode {
                id: step.name.clone(),
                worker: step.skill.clone().unwrap_or_default(),
                persona: None,
                inputs,
                outputs: BTreeMap::new(),
                depends_on: step.depends_on.clone(),
                cache_key: None,
            }
        })
        .collect();
    Workflow {
        id: parsed.workflow.name.clone(),
        description: parsed.workflow.description.clone(),
        tasks,
        edges: Vec::new(),
    }
}

async fn persist_workflow_run(
    store: &Arc<SessionStore>,
    head: &WorkflowDefinition,
    report: &argo_workflow::ExecutionReport,
    started_at: chrono::DateTime<Utc>,
    finished_at: chrono::DateTime<Utc>,
) -> Result<(), String> {
    // Schema 18 (`workflow_run`) predates the tenant-scoped
    // `workflow_definition` table; the row is keyed by the
    // executor's `workflow_id` which the executor synthesises from
    // the parsed DAG. We tag the row with the compound key
    // `{vertical_id}/{name}` so `routes_workflow_runs::dispatch_get`
    // (which walks both the compound + bare keys) resolves it back.
    let compound_key = format!("{}/{}", head.vertical_id, head.name);
    let verdicts_json = serde_json::to_string(&report.task_verdicts)
        .map_err(|e| format!("verdicts json: {e}"))?;
    crate::handlers::workflow::ensure_workflow_schema(store.as_ref())
        .await
        .map_err(|e| format!("workflow_run schema: {e}"))?;
    sqlx::query(
        "INSERT INTO workflow_run \
         (id, workflow_id, workflow_yaml, started_at, finished_at, ok, task_verdicts_json) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(report.workflow_id.as_str())
    .bind(&compound_key)
    .bind(&head.definition_toml)
    .bind(started_at.timestamp_micros())
    .bind(finished_at.timestamp_micros())
    .bind(if report.ok { 1_i64 } else { 0 })
    .bind(verdicts_json)
    .execute(store.pool())
    .await
    .map_err(|e| format!("workflow_run insert: {e}"))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn emit_dispatch_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    actor: &str,
    vertical: &str,
    name: &str,
    run_id: &str,
    ok: bool,
    idempotency_key: Option<&str>,
    dispatched_at: chrono::DateTime<Utc>,
) {
    let payload = json!({
        "actor": actor,
        "tenant_id": tenant.to_string(),
        "vertical_id": vertical,
        "workflow_name": name,
        "run_id": run_id,
        "ok": ok,
        "idempotency_key": idempotency_key,
        "dispatched_at": dispatched_at.to_rfc3339(),
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(
            None,
            AuditKind::WorkflowExecuteDispatched,
            payload,
            Some(actor.to_string()),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::routes_workflow_execute",
            error = %e,
            vertical_id = %vertical,
            workflow_name = %name,
            run_id = %run_id,
            "workflow execute-dispatched audit write failed",
        );
    }
}

fn parse_request_body(body: &str) -> Result<(Value, Option<String>), String> {
    if body.trim().is_empty() {
        return Ok((Value::Object(serde_json::Map::new()), None));
    }
    let parsed: Value = serde_json::from_str(body).map_err(|e| format!("parse json body: {e}"))?;
    let inputs = parsed
        .get("inputs")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let idempotency_key = match parsed.get("idempotency_key") {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        Some(Value::Null) | None => None,
        Some(other) => {
            return Err(format!(
                "idempotency_key must be a string when present, got {other}"
            ))
        }
    };
    Ok((inputs, idempotency_key))
}

fn resolve_principal(req: &str) -> Result<(Principal, &'static str), (&'static str, String)> {
    let raw = header_value(req, PRINCIPAL_HEADER).ok_or_else(unauthorized)?;
    let (kind, identity) = match raw.split_once(':') {
        Some((k, v)) if !v.is_empty() => (k.to_string(), v.to_string()),
        _ => return Err(unauthorized()),
    };
    let principal = Principal::Gateway {
        adapter: kind,
        identity,
    };
    let role = match resolve_role(req).unwrap_or(UserTenantRole::Guest) {
        UserTenantRole::Owner => "owner",
        UserTenantRole::Admin => "admin",
        UserTenantRole::User => "user",
        UserTenantRole::Guest => "guest",
    };
    Ok((principal, role))
}

fn check_capability(
    principal: &Principal,
    role: &str,
    requested: &Capability,
) -> Result<(), (&'static str, String)> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let engine = CapabilityEngine::with_workspace_defaults(&cwd);
    if engine.bind(principal.clone(), role.to_string()).is_err() {
        return Err(forbidden());
    }
    engine.check(principal, requested).map_err(|_| forbidden())
}

fn header_value(req: &str, name: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (k, v) = line.split_once(':')?;
        k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
    })
}

fn principal_to_actor(principal: &Principal) -> String {
    match principal {
        Principal::Gateway { adapter, identity } => format!("{adapter}:{identity}"),
        Principal::Local { os_user } => format!("local:{os_user}"),
        Principal::Acp { harness, identity } => format!("{harness}:{identity}"),
        Principal::Forge => "forge".to_string(),
    }
}

async fn resolve_tenant(store: &SessionStore) -> Result<TenantId, (&'static str, String)> {
    let cwd = std::env::current_dir().map_err(|e| internal(&format!("cwd: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    store
        .ensure_workspace(name, &root)
        .await
        .map(|(t, _)| t)
        .map_err(|e| internal(&format!("resolve tenant: {e}")))
}

#[cfg(test)]
#[path = "routes_workflow_execute_tests.rs"]
mod tests;
