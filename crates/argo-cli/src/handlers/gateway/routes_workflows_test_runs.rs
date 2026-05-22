//! Workflow test-run gateway routes (Stream
//! `claude-followup-pr-2021-test-run-gateway-persist`, re-author of
//! closed PR #2021 at slot 130/v78).
//!
//! POST `/api/v1/verticals/{vertical_id}/workflows/{name}/test_runs`
//! persists one completed test run from the builder's "Test run"
//! overlay; GET on the same path returns the last N runs
//! newest-first. Backed by [`argo_store::workflow_test_run`].
//!
//! ## Authorization model
//!
//! Mirrors `routes_workflows`: anonymous callers (no
//! `X-Argo-Principal` header) receive `401 Unauthorized` before the
//! DB layer is touched. Reads pass under the
//! `workflow_read_capability`; writes pass under
//! `workflow_edit_capability`. Cross-tenant lookups return an empty
//! `items` list (not 404) so callers cannot probe existence.

use super::role_gate::{enforce_role, resolve_role, Action};
use argo_audit::{AuditKind, AuditSink};
use argo_capability::{workflow_edit_capability, workflow_read_capability, CapabilityEngine};
use argo_identity::user_tenant::UserTenantRole;
use argo_store::workflow_test_run::{self as wtr, WorkflowTestRun, WorkflowTestRunRecord};
use argo_store::SessionStore;
use argo_types::{Capability, Principal, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;

const PREFIX: &str = "/api/v1/verticals/";
const SUFFIX: &str = "test_runs";
const PRINCIPAL_HEADER: &str = "X-Argo-Principal";
const DEFAULT_LIMIT: i64 = 10;
const MAX_LIMIT: i64 = 100;

/// GET dispatcher. Returns `Some` for any path matching
/// `/api/v1/verticals/{vid}/workflows/{name}/test_runs`.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let (vertical, name, query) = parse_path(req, "GET ")?;
    let store = match store {
        Some(s) => s,
        None => return Some(super::store_unavailable()),
    };
    Some(handle_get(req, store, vertical, name, query).await)
}

/// POST dispatcher. Returns `Some` for any POST hitting the test-runs
/// collection path.
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let (vertical, name, _query) = parse_path(req, "POST ")?;
    let store = match store {
        Some(s) => s,
        None => return Some(super::store_unavailable()),
    };
    Some(handle_post(req, store, vertical, name).await)
}

fn parse_path<'a>(req: &'a str, verb: &str) -> Option<(&'a str, &'a str, &'a str)> {
    let after_verb = req.strip_prefix(verb)?;
    let path_with_query = after_verb.split_whitespace().next()?;
    let (path, query) = path_with_query
        .split_once('?')
        .unwrap_or((path_with_query, ""));
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
    Some((vertical, name, query))
}

async fn handle_get(
    req: &str,
    store: &Arc<SessionStore>,
    vertical: &str,
    name: &str,
    query: &str,
) -> (&'static str, String) {
    let (principal, role) = match resolve_principal(req) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    if let Err(resp) = check_capability(&principal, role, &workflow_read_capability()) {
        return resp;
    }
    let tenant = match resolve_tenant(store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let limit = parse_limit(query);
    match wtr::list_recent(store.pool(), tenant, vertical, name, limit).await {
        Ok(rows) => {
            let items: Vec<Value> = rows.iter().map(run_json).collect();
            let body = json!({
                "vertical_id": vertical,
                "workflow_name": name,
                "count": items.len(),
                "items": items,
            });
            ("200 OK", body.to_string())
        }
        Err(e) => internal(&format!("list_recent: {e}")),
    }
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
    let actor = principal_to_actor(&principal);
    let body = super::body_from_request(req);
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse json body: {e}")),
    };
    let record = match decode_record(&parsed) {
        Ok(r) => r,
        Err(msg) => return bad_request(&msg),
    };
    match wtr::record(store.pool(), tenant, vertical, name, &record).await {
        Ok(id) => {
            emit_persist_audit(store, tenant, &actor, vertical, name, &record, &id.0).await;
            let body = json!({
                "id": id.0,
                "vertical_id": vertical,
                "workflow_name": name,
            });
            ("201 Created", body.to_string())
        }
        Err(argo_types::ArgoError::Schema(msg)) => bad_request(&msg),
        Err(e) => internal(&format!("record test-run: {e}")),
    }
}

async fn emit_persist_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    actor: &str,
    vertical: &str,
    name: &str,
    record: &WorkflowTestRunRecord,
    test_run_id: &str,
) {
    // Payload shape per the brief: `{ worker_id, vertical_id,
    // workflow_name, status }`. `worker_id` carries the resolved
    // gateway principal (closest stand-in when the run was triggered
    // by a human). Adds `tenant_id` + `test_run_id` for forensic
    // traceability — matches the pattern in `routes_workflows`.
    let payload = json!({
        "worker_id": actor,
        "tenant_id": tenant.to_string(),
        "vertical_id": vertical,
        "workflow_name": name,
        "status": record.status,
        "test_run_id": test_run_id,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(
            None,
            AuditKind::WorkflowTestRunPersisted,
            payload,
            Some(actor.to_string()),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::routes_workflows_test_runs",
            error = %e,
            kind = %AuditKind::WorkflowTestRunPersisted.label(),
            vertical_id = %vertical,
            workflow_name = %name,
            "workflow test-run audit write failed",
        );
    }
}

fn decode_record(body: &Value) -> Result<WorkflowTestRunRecord, String> {
    let status = body
        .get("status")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "missing or empty `status`".to_string())?;
    if !matches!(status, "complete" | "failed") {
        return Err(format!(
            "status must be one of complete|failed, got '{status}'"
        ));
    }
    let started_at = body
        .get("started_at")
        .and_then(Value::as_i64)
        .ok_or_else(|| "missing or non-integer `started_at` (Unix ms)".to_string())?;
    let ended_at = body
        .get("ended_at")
        .and_then(Value::as_i64)
        .ok_or_else(|| "missing or non-integer `ended_at` (Unix ms)".to_string())?;
    if started_at <= 0 || ended_at <= 0 {
        return Err("started_at / ended_at must be positive Unix-millis".to_string());
    }
    let summary_json = encode_optional_object(body.get("summary"), "null");
    let step_states_json = encode_optional_object(body.get("step_states"), "{}");
    let transport_error = match body.get("transport_error") {
        Some(Value::Null) | None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(other) => return Err(format!("transport_error must be string|null, got {other}")),
    };
    Ok(WorkflowTestRunRecord {
        status: status.to_string(),
        started_at,
        ended_at,
        summary_json,
        step_states_json,
        transport_error,
    })
}

fn encode_optional_object(value: Option<&Value>, default: &str) -> String {
    match value {
        Some(Value::Null) | None => default.to_string(),
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(other) => other.to_string(),
    }
}

fn parse_limit(query: &str) -> i64 {
    if query.is_empty() {
        return DEFAULT_LIMIT;
    }
    for pair in query.split('&') {
        if let Some(raw) = pair.strip_prefix("limit=") {
            if let Ok(parsed) = raw.parse::<i64>() {
                return parsed.clamp(1, MAX_LIMIT);
            }
            break;
        }
    }
    DEFAULT_LIMIT
}

fn run_json(run: &WorkflowTestRun) -> Value {
    json!({
        "id": run.id.0,
        "vertical_id": run.vertical_id,
        "workflow_name": run.workflow_name,
        "started_at": run.started_at,
        "ended_at": run.ended_at,
        "status": run.status,
        "summary_json": run.summary_json,
        "step_states_json": run.step_states_json,
        "transport_error": run.transport_error,
    })
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

fn unauthorized() -> (&'static str, String) {
    (
        "401 Unauthorized",
        json!({"error": {
            "code": "unauthorized",
            "message": "X-Argo-Principal header is required"
        }})
        .to_string(),
    )
}

fn forbidden() -> (&'static str, String) {
    (
        "403 Forbidden",
        json!({"error": {
            "code": "forbidden",
            "message": "principal lacks the required workflow capability"
        }})
        .to_string(),
    )
}

fn bad_request(message: &str) -> (&'static str, String) {
    (
        "400 Bad Request",
        json!({"error": {"code": "bad_request", "message": message}}).to_string(),
    )
}

fn internal(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({"error": {"code": "internal", "message": message}}).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_workflows_test_runs_tests.rs"]
mod tests;
