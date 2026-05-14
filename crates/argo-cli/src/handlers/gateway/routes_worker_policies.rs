//! Per-worker approval-policy endpoints (Stream
//! `mission-approval-policy-ui`).
//!
//! - `GET   /api/v1/workers/<id>/policies` — manifest-declared
//!   action types joined with the worker's per-action overrides.
//! - `PATCH /api/v1/workers/<id>/policies/<action_type>` — body:
//!   `{ "policy": "<value>", "operator_note": "<why>" }`.
//!
//! Per-worker overrides live in
//! `.skycot/worker-policies/<worker_id>.toml`, distinct from the
//! workspace-level `.skycot/auto_approval.toml` (different schema;
//! see PR body for sensible-deviation notes). Vocabulary: see
//! `POLICY_VALUES`; `risk-gated` is the default. Read-only mode
//! keys off `CircuitState::HalfOpen` (the same signal
//! `routes_team_list` surfaces as `paused`).

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{CircuitState, IdentityStore, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use toml::value::{Table, Value as TomlValue};

/// Canonical policy vocabulary. Order is the canonical render order.
pub(super) const POLICY_VALUES: &[&str] = &[
    "auto-approve",
    "always-ask",
    "always-refuse",
    "risk-gated",
];

/// Default when an action_type has no explicit override.
const DEFAULT_POLICY: &str = "risk-gated";

/// Workspace-relative directory for per-worker policy files.
const POLICY_DIR: &str = ".skycot/worker-policies";

/// Bound on operator-note size so the audit row stays compact.
const MAX_OPERATOR_NOTE_BYTES: usize = 1024;

/// `GET /api/v1/workers/<id>/policies` — full per-action list.
pub(super) async fn list_response(
    worker_id_raw: &str,
    _req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let worker_id = match worker_id_raw.parse::<WorkerId>() {
        Ok(w) => w,
        Err(e) => return bad_request(&format!("invalid worker_id '{worker_id_raw}': {e}")),
    };
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let cwd = match std::env::current_dir() {
        Ok(c) => c,
        Err(e) => return server_error(&format!("cwd: {e}")),
    };
    let paused = match resolve_paused(store, worker_id).await {
        Ok(p) => p,
        Err(WorkerLookupError::NotFound) => return not_found(worker_id_raw),
        Err(WorkerLookupError::CrossTenant) => {
            return forbidden(&format!("worker {worker_id} not in this tenant"))
        }
        Err(WorkerLookupError::Backend(msg)) => return server_error(&msg),
    };
    let overrides = match read_policies(&cwd, worker_id) {
        Ok(map) => map,
        Err(e) => return server_error(&e),
    };
    let items: Vec<Value> = list_action_types()
        .into_iter()
        .map(|(name, risk)| {
            let policy = overrides
                .get(&name)
                .cloned()
                .unwrap_or_else(|| DEFAULT_POLICY.to_string());
            json!({
                "action_type": name,
                "current_policy": policy,
                "inferred_trust_grade": risk,
                "is_default": !overrides.contains_key(&name),
            })
        })
        .collect();
    let body = json!({
        "worker_id": worker_id.to_string(),
        "read_only": paused,
        "policies": items,
        "allowed_policies": POLICY_VALUES,
        "default_policy": DEFAULT_POLICY,
    });
    ("200 OK", body.to_string())
}

/// `PATCH /api/v1/workers/<id>/policies/<action_type>` — overwrite one row.
pub(super) async fn update_response(
    worker_id_raw: &str,
    action_type_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let worker_id = match worker_id_raw.parse::<WorkerId>() {
        Ok(w) => w,
        Err(e) => return bad_request(&format!("invalid worker_id '{worker_id_raw}': {e}")),
    };
    let action_type = action_type_raw.trim();
    if action_type.is_empty() {
        return bad_request("action_type is empty");
    }
    let parsed = match parse_patch_body(body_from_request(req)) {
        Ok(p) => p,
        Err(msg) => return bad_request(&msg),
    };
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let cwd = match std::env::current_dir() {
        Ok(c) => c,
        Err(e) => return server_error(&format!("cwd: {e}")),
    };
    let paused = match resolve_paused(store, worker_id).await {
        Ok(p) => p,
        Err(WorkerLookupError::NotFound) => return not_found(worker_id_raw),
        Err(WorkerLookupError::CrossTenant) => {
            return forbidden(&format!("worker {worker_id} not in this tenant"))
        }
        Err(WorkerLookupError::Backend(msg)) => return server_error(&msg),
    };
    if paused {
        return conflict("worker is paused; resume the worker to edit policies");
    }
    let mut overrides = match read_policies(&cwd, worker_id) {
        Ok(m) => m,
        Err(e) => return server_error(&e),
    };
    let old_policy = overrides
        .get(action_type)
        .cloned()
        .unwrap_or_else(|| DEFAULT_POLICY.to_string());
    overrides.insert(action_type.to_string(), parsed.policy.clone());
    if let Err(e) = write_policies(&cwd, worker_id, &overrides) {
        return server_error(&e);
    }
    let principal_id =
        principal_id_for_audit(req).unwrap_or_else(|| "system:policies".to_string());
    emit_policy_audit(
        store,
        &principal_id,
        worker_id,
        action_type,
        &old_policy,
        &parsed.policy,
        &parsed.operator_note,
    )
    .await;
    let body = json!({
        "worker_id": worker_id.to_string(),
        "action_type": action_type,
        "policy": parsed.policy,
        "previous_policy": old_policy,
        "operator_note": parsed.operator_note,
        "inferred_trust_grade": inferred_trust_grade_for(action_type),
    });
    ("200 OK", body.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PatchBody {
    pub policy: String,
    pub operator_note: String,
}

pub(super) fn parse_patch_body(body: &str) -> Result<PatchBody, String> {
    let parsed: Value = serde_json::from_str(body).map_err(|e| format!("parse body: {e}"))?;
    let obj = parsed
        .as_object()
        .ok_or_else(|| "body must be a JSON object".to_string())?;
    let policy = obj
        .get("policy")
        .and_then(Value::as_str)
        .ok_or_else(|| "`policy` is required".to_string())?
        .trim()
        .to_string();
    if !POLICY_VALUES.contains(&policy.as_str()) {
        return Err(format!(
            "unknown policy '{policy}'; allowed: {}",
            POLICY_VALUES.join(", ")
        ));
    }
    let note = obj
        .get("operator_note")
        .and_then(Value::as_str)
        .ok_or_else(|| "`operator_note` is required".to_string())?
        .trim();
    if note.is_empty() {
        return Err("`operator_note` must be non-empty".to_string());
    }
    if note.len() > MAX_OPERATOR_NOTE_BYTES {
        return Err(format!(
            "`operator_note` exceeds {MAX_OPERATOR_NOTE_BYTES} bytes"
        ));
    }
    Ok(PatchBody {
        policy,
        operator_note: note.to_string(),
    })
}

pub(super) fn worker_policy_path(cwd: &Path, worker_id: WorkerId) -> PathBuf {
    cwd.join(POLICY_DIR).join(format!("{worker_id}.toml"))
}

pub(super) fn read_policies(
    cwd: &Path,
    worker_id: WorkerId,
) -> Result<BTreeMap<String, String>, String> {
    let path = worker_policy_path(cwd, worker_id);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let body = std::fs::read_to_string(&path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    let table: Table = toml::from_str(&body)
        .map_err(|e| format!("parse {}: {e}", path.display()))?;
    let mut out = BTreeMap::new();
    if let Some(TomlValue::Table(policies)) = table.get("policies") {
        for (k, v) in policies {
            if let TomlValue::String(s) = v {
                if POLICY_VALUES.contains(&s.as_str()) {
                    out.insert(k.clone(), s.clone());
                }
            }
        }
    }
    Ok(out)
}

pub(super) fn write_policies(
    cwd: &Path,
    worker_id: WorkerId,
    overrides: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = worker_policy_path(cwd, worker_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create_dir_all {}: {e}", parent.display()))?;
    }
    let mut policies = Table::new();
    for (k, v) in overrides {
        policies.insert(k.clone(), TomlValue::String(v.clone()));
    }
    let mut root = Table::new();
    root.insert("policies".to_string(), TomlValue::Table(policies));
    let body = toml::to_string_pretty(&TomlValue::Table(root))
        .map_err(|e| format!("serialise policies: {e}"))?;
    std::fs::write(&path, body).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

fn list_action_types() -> Vec<(String, String)> {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let Ok((registry, _errors)) = argo_vertical::load_dir(&dir) else {
        return Vec::new();
    };
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for (_id, loaded) in registry.list() {
        let Some(actions) = loaded.manifest.actions.as_ref() else {
            continue;
        };
        for (name, decl) in &actions.actions {
            seen.entry(name.clone())
                .or_insert_with(|| risk_label(decl.risk).to_string());
        }
    }
    seen.into_iter().collect()
}

fn risk_label(risk: argo_vertical::ActionRiskLevel) -> &'static str {
    match risk {
        argo_vertical::ActionRiskLevel::Low => "low",
        argo_vertical::ActionRiskLevel::Medium => "medium",
        argo_vertical::ActionRiskLevel::High => "high",
    }
}

fn inferred_trust_grade_for(action_type: &str) -> &'static str {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let Ok((registry, _errors)) = argo_vertical::load_dir(&dir) else {
        return "unknown";
    };
    for (_id, loaded) in registry.list() {
        let Some(actions) = loaded.manifest.actions.as_ref() else {
            continue;
        };
        if let Some(decl) = actions.actions.get(action_type) {
            return risk_label(decl.risk);
        }
    }
    "unknown"
}

#[derive(Debug)]
enum WorkerLookupError {
    NotFound,
    CrossTenant,
    Backend(String),
}

async fn resolve_paused(
    store: &Arc<SessionStore>,
    worker_id: WorkerId,
) -> Result<bool, WorkerLookupError> {
    let tenant = resolve_workspace_tenant(store)
        .await
        .map_err(|e| WorkerLookupError::Backend(format!("resolve tenant: {e}")))?;
    let identities = IdentityStore::new(store);
    let worker = identities
        .get_worker(tenant, worker_id)
        .await
        .map_err(|e| WorkerLookupError::Backend(format!("get worker: {e}")))?;
    match worker {
        Some(w) => Ok(matches!(w.circuit_state, CircuitState::HalfOpen)),
        None => match exists_in_other_tenant(store, tenant, worker_id).await {
            Ok(true) => Err(WorkerLookupError::CrossTenant),
            Ok(false) => Err(WorkerLookupError::NotFound),
            Err(e) => Err(WorkerLookupError::Backend(format!("cross-tenant probe: {e}"))),
        },
    }
}

async fn exists_in_other_tenant(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<bool> {
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
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

async fn emit_policy_audit(
    store: &Arc<SessionStore>,
    principal_id: &str,
    worker_id: WorkerId,
    action_type: &str,
    old_policy: &str,
    new_policy: &str,
    operator_note: &str,
) {
    let payload = json!({
        "endpoint": "worker.policy_changed",
        "worker_id": worker_id.to_string(),
        "action_type": action_type,
        "old_policy": old_policy,
        "new_policy": new_policy,
        "operator_note": operator_note,
    });
    let sink = AuditSink::new(Arc::clone(store));
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, Some(principal_id.to_string()))
        .await
    {
        tracing::warn!(
            target: "gateway::worker_policies",
            error = %e,
            worker_id = %worker_id,
            action_type = %action_type,
            "worker.policy_changed audit write failed"
        );
    }
}

fn bad_request(msg: &str) -> (&'static str, String) {
    err("400 Bad Request", "bad_request", msg)
}
fn not_found(id: &str) -> (&'static str, String) {
    err("404 Not Found", "worker_not_found", &format!("no worker with id '{id}'"))
}
fn forbidden(msg: &str) -> (&'static str, String) {
    err("403 Forbidden", "forbidden", msg)
}
fn conflict(msg: &str) -> (&'static str, String) {
    err("409 Conflict", "worker_paused", msg)
}
fn server_error(msg: &str) -> (&'static str, String) {
    err("500 Internal Server Error", "internal_error", msg)
}
fn err(status: &'static str, code: &str, msg: &str) -> (&'static str, String) {
    (status, json!({ "error": { "code": code, "message": msg } }).to_string())
}

#[cfg(test)]
#[path = "routes_worker_policies_tests.rs"]
mod tests;
