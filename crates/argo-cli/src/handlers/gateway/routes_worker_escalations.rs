//! Per-worker escalation-policy endpoints (Stream
//! `worker-detail-escalation-policy`).
//!
//! - `GET   /api/v1/workers/<id>/escalations` — the worker's
//!   persisted escalation rules (empty list + `is_default: true`
//!   when the operator has not authored any).
//! - `PATCH /api/v1/workers/<id>/escalations` — body:
//!   `{ "rules": [ { "action", "condition", "routes_to",
//!   "channel"? }, ... ], "operator_note": "<why>" }` — overwrites
//!   the whole rule set in one write and appends an audit row.
//!
//! Per-worker escalation rules live in
//! `.skycot/worker-escalations/<worker_id>.toml`, mirroring the
//! `.skycot/worker-policies/<worker_id>.toml` convention in
//! `routes_worker_policies.rs` (same on-disk home, same
//! cross-tenant-hidden 404, same paused-worker conflict guard). The
//! rule shape matches the SPA `EscalationRule` contract
//! (`web/components/skycot/workers/EscalationMatrix.tsx`):
//! `action` × `condition` → `routes_to` (+ optional `channel`).
//! Read-only mode keys off `CircuitState::HalfOpen` exactly as the
//! policies endpoint does.

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{CircuitState, IdentityStore, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use toml::value::{Table, Value as TomlValue};

/// Workspace-relative directory for per-worker escalation files.
const ESCALATION_DIR: &str = ".skycot/worker-escalations";

/// Bound on operator-note size so the audit row stays compact.
const MAX_OPERATOR_NOTE_BYTES: usize = 1024;

/// Bound on the number of rules so a single PATCH can't bloat the
/// audit payload or the on-disk file unboundedly.
const MAX_RULES: usize = 64;

/// Bound on any single rule field so the TOML + audit row stay
/// compact.
const MAX_FIELD_BYTES: usize = 512;

/// One escalation rule: the action the worker would otherwise
/// auto-take, the condition that should pause it, the human owner the
/// escalation routes to, and an optional channel hint. Local to this
/// module (not a workspace-wide canonical type) so it does not
/// collide with the unrelated `argo_orchestrator::EscalationPolicy`
/// (SLA breach actions) or `argo_capability::EscalationTarget`
/// (truth-boundary targets). Mirrors the SPA `EscalationRule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EscalationRule {
    pub action: String,
    pub condition: String,
    pub routes_to: String,
    pub channel: Option<String>,
}

/// `GET /api/v1/workers/<id>/escalations` — the persisted rule set.
pub(super) async fn list_response(
    worker_id_raw: &str,
    req: &str,
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
    // Cross-tenant existence is hidden from the HTTP response.
    // See routes_worker_policies.rs / routes_workflows.rs:13-21.
    let paused = match resolve_paused(req, store, worker_id).await {
        Ok(p) => p,
        Err(WorkerLookupError::NotFound) => return worker_not_found(),
        Err(WorkerLookupError::CrossTenant) => {
            tracing::debug!(target: "tenancy", %worker_id, "cross-tenant probe: exists in other tenant");
            return worker_not_found();
        }
        Err(WorkerLookupError::Backend(msg)) => return server_error(&msg),
    };
    let rules = match read_rules(&cwd, worker_id) {
        Ok(r) => r,
        Err(e) => return server_error(&e),
    };
    let is_default = rules.is_empty();
    let items: Vec<Value> = rules.iter().map(rule_to_json).collect();
    let body = json!({
        "worker_id": worker_id.to_string(),
        "read_only": paused,
        "rules": items,
        "is_default": is_default,
    });
    ("200 OK", body.to_string())
}

/// `PATCH /api/v1/workers/<id>/escalations` — overwrite the rule set.
pub(super) async fn update_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let worker_id = match worker_id_raw.parse::<WorkerId>() {
        Ok(w) => w,
        Err(e) => return bad_request(&format!("invalid worker_id '{worker_id_raw}': {e}")),
    };
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
    // Cross-tenant existence is hidden from the HTTP response.
    let paused = match resolve_paused(req, store, worker_id).await {
        Ok(p) => p,
        Err(WorkerLookupError::NotFound) => return worker_not_found(),
        Err(WorkerLookupError::CrossTenant) => {
            tracing::debug!(target: "tenancy", %worker_id, "cross-tenant probe: exists in other tenant");
            return worker_not_found();
        }
        Err(WorkerLookupError::Backend(msg)) => return server_error(&msg),
    };
    if paused {
        return conflict("worker is paused; resume the worker to edit escalations");
    }
    let previous = match read_rules(&cwd, worker_id) {
        Ok(r) => r,
        Err(e) => return server_error(&e),
    };
    if let Err(e) = write_rules(&cwd, worker_id, &parsed.rules) {
        return server_error(&e);
    }
    let principal_id =
        principal_id_for_audit(req).unwrap_or_else(|| "system:escalations".to_string());
    emit_escalation_audit(
        req,
        store,
        &principal_id,
        worker_id,
        previous.len(),
        &parsed.rules,
        &parsed.operator_note,
    )
    .await;
    let items: Vec<Value> = parsed.rules.iter().map(rule_to_json).collect();
    let body = json!({
        "worker_id": worker_id.to_string(),
        "rules": items,
        "previous_rule_count": previous.len(),
        "operator_note": parsed.operator_note,
    });
    ("200 OK", body.to_string())
}

fn rule_to_json(rule: &EscalationRule) -> Value {
    json!({
        "action": rule.action,
        "condition": rule.condition,
        "routes_to": rule.routes_to,
        "channel": rule.channel,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PatchBody {
    pub rules: Vec<EscalationRule>,
    pub operator_note: String,
}

pub(super) fn parse_patch_body(body: &str) -> Result<PatchBody, String> {
    let parsed: Value = serde_json::from_str(body).map_err(|e| format!("parse body: {e}"))?;
    let obj = parsed
        .as_object()
        .ok_or_else(|| "body must be a JSON object".to_string())?;
    let rules_val = obj
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| "`rules` is required and must be an array".to_string())?;
    if rules_val.len() > MAX_RULES {
        return Err(format!("`rules` exceeds the {MAX_RULES}-rule limit"));
    }
    let mut rules = Vec::with_capacity(rules_val.len());
    for (i, raw) in rules_val.iter().enumerate() {
        rules.push(parse_rule(raw, i)?);
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
        rules,
        operator_note: note.to_string(),
    })
}

fn parse_rule(raw: &Value, index: usize) -> Result<EscalationRule, String> {
    let obj = raw
        .as_object()
        .ok_or_else(|| format!("rule[{index}] must be a JSON object"))?;
    let action = required_field(obj, "action", index)?;
    let condition = required_field(obj, "condition", index)?;
    let routes_to = required_field(obj, "routes_to", index)?;
    let channel = match obj.get("channel") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                if trimmed.len() > MAX_FIELD_BYTES {
                    return Err(format!("rule[{index}].channel exceeds {MAX_FIELD_BYTES} bytes"));
                }
                Some(trimmed.to_string())
            }
        }
        Some(_) => return Err(format!("rule[{index}].channel must be a string")),
    };
    Ok(EscalationRule {
        action,
        condition,
        routes_to,
        channel,
    })
}

fn required_field(
    obj: &serde_json::Map<String, Value>,
    key: &str,
    index: usize,
) -> Result<String, String> {
    let value = obj
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("rule[{index}].{key} is required"))?
        .trim();
    if value.is_empty() {
        return Err(format!("rule[{index}].{key} must be non-empty"));
    }
    if value.len() > MAX_FIELD_BYTES {
        return Err(format!("rule[{index}].{key} exceeds {MAX_FIELD_BYTES} bytes"));
    }
    Ok(value.to_string())
}

pub(super) fn worker_escalation_path(cwd: &Path, worker_id: WorkerId) -> PathBuf {
    cwd.join(ESCALATION_DIR).join(format!("{worker_id}.toml"))
}

pub(super) fn read_rules(cwd: &Path, worker_id: WorkerId) -> Result<Vec<EscalationRule>, String> {
    let path = worker_escalation_path(cwd, worker_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let body =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let table: Table =
        toml::from_str(&body).map_err(|e| format!("parse {}: {e}", path.display()))?;
    let mut out = Vec::new();
    if let Some(TomlValue::Array(rows)) = table.get("rule") {
        for row in rows {
            if let TomlValue::Table(t) = row {
                if let Some(rule) = rule_from_toml(t) {
                    out.push(rule);
                }
            }
        }
    }
    Ok(out)
}

fn rule_from_toml(t: &Table) -> Option<EscalationRule> {
    let action = toml_string(t, "action")?;
    let condition = toml_string(t, "condition")?;
    let routes_to = toml_string(t, "routes_to")?;
    let channel = toml_string(t, "channel");
    Some(EscalationRule {
        action,
        condition,
        routes_to,
        channel,
    })
}

fn toml_string(t: &Table, key: &str) -> Option<String> {
    match t.get(key) {
        Some(TomlValue::String(s)) if !s.trim().is_empty() => Some(s.clone()),
        _ => None,
    }
}

pub(super) fn write_rules(
    cwd: &Path,
    worker_id: WorkerId,
    rules: &[EscalationRule],
) -> Result<(), String> {
    let path = worker_escalation_path(cwd, worker_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create_dir_all {}: {e}", parent.display()))?;
    }
    let rows: Vec<TomlValue> = rules
        .iter()
        .map(|r| {
            let mut t = Table::new();
            t.insert("action".to_string(), TomlValue::String(r.action.clone()));
            t.insert(
                "condition".to_string(),
                TomlValue::String(r.condition.clone()),
            );
            t.insert(
                "routes_to".to_string(),
                TomlValue::String(r.routes_to.clone()),
            );
            if let Some(channel) = &r.channel {
                t.insert("channel".to_string(), TomlValue::String(channel.clone()));
            }
            TomlValue::Table(t)
        })
        .collect();
    let mut root = Table::new();
    root.insert("rule".to_string(), TomlValue::Array(rows));
    let body = toml::to_string_pretty(&TomlValue::Table(root))
        .map_err(|e| format!("serialise escalations: {e}"))?;
    std::fs::write(&path, body).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

#[derive(Debug)]
enum WorkerLookupError {
    NotFound,
    CrossTenant,
    Backend(String),
}

async fn resolve_paused(
    req: &str,
    store: &Arc<SessionStore>,
    worker_id: WorkerId,
) -> Result<bool, WorkerLookupError> {
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store)
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
            Err(e) => Err(WorkerLookupError::Backend(format!(
                "cross-tenant probe: {e}"
            ))),
        },
    }
}

async fn exists_in_other_tenant(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: WorkerId,
) -> ArgoResult<bool> {
    let row: Option<String> =
        sqlx::query_scalar("SELECT id FROM worker WHERE id = ? AND tenant_id != ? LIMIT 1")
            .bind(worker_id.to_string())
            .bind(tenant.to_string())
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("cross-tenant probe: {e}")))?;
    Ok(row.is_some())
}

async fn emit_escalation_audit(
    req: &str,
    store: &Arc<SessionStore>,
    principal_id: &str,
    worker_id: WorkerId,
    previous_rule_count: usize,
    rules: &[EscalationRule],
    operator_note: &str,
) {
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(
                target: "gateway::worker_escalations",
                error = %e,
                worker_id = %worker_id,
                "resolve tenant for audit failed"
            );
            return;
        }
    };
    let payload = json!({
        "endpoint": "worker.escalation_changed",
        "worker_id": worker_id.to_string(),
        "previous_rule_count": previous_rule_count,
        "new_rule_count": rules.len(),
        "operator_note": operator_note,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            Some(principal_id.to_string()),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::worker_escalations",
            error = %e,
            worker_id = %worker_id,
            "worker.escalation_changed audit write failed"
        );
    }
}

fn bad_request(msg: &str) -> (&'static str, String) {
    err("400 Bad Request", "bad_request", msg)
}
/// Generic 404 used for both "worker not found in this tenant" and
/// "exists in another tenant" paths so the HTTP response cannot
/// disambiguate the two. Mirrors `routes_worker_policies.rs`.
fn worker_not_found() -> (&'static str, String) {
    err("404 Not Found", "not_found", "worker not found")
}
fn conflict(msg: &str) -> (&'static str, String) {
    err("409 Conflict", "worker_paused", msg)
}
fn server_error(msg: &str) -> (&'static str, String) {
    err("500 Internal Server Error", "internal_error", msg)
}
fn err(status: &'static str, code: &str, msg: &str) -> (&'static str, String) {
    (
        status,
        json!({ "error": { "code": code, "message": msg } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_worker_escalations_tests.rs"]
mod tests;
