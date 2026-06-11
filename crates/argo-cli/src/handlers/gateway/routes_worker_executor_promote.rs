//! `POST /api/v1/workers/<id>/actions/<action>/executor` — graduate one
//! action off the draft-only human default onto a real executor
//! (UF Phase-B · B3, DEC-037).
//!
//! A hired teammate is **draft-only at birth** ([[DEC-025]] #6): every
//! generated action is `executor = "human"`. DEC-037 greenlit the
//! deliberate post-hire grant — a **proven** teammate may have one
//! action graduated onto a real executor (MCP today). This route records
//! that grant. It does NOT flip execution on by itself: honouring a
//! promotion at dispatch is dormant and default-OFF
//! (`ARGO_EXECUTION_GRADUATION`, see
//! [`crate::handlers::worker_actions_executor_promotion`]); the live flip
//! stays operator-only after the blast-radius review.
//!
//! The route:
//! 1. Parses the path worker id (UUID-validated) + action segment.
//! 2. Resolves the workspace tenant + loads the worker (404 / 403 if
//!    unknown or cross-tenant).
//! 3. Enforces [`Action::WorkerExecutorPromote`] (owner only). On deny:
//!    emits an [`AuditKind::RbacDeny`] row and returns 403.
//! 4. **Probation precondition** (DEC-037 / [[DEC-025]] #2): a worker
//!    still inside its 24h probation window cannot be promoted — returns
//!    409 with a `RbacDeny`-tagged audit row and writes NO grant.
//! 5. Parses the JSON body `{ kind, server, tool }`. `kind` must be
//!    `mcp` (the only promotable kind in v1); `server` + `tool` are
//!    required.
//! 6. Persists the grant via [`argo_store::executor_promotion::promote`].
//! 7. Emits an [`AuditKind::HttpRequest`] `executor.promoted` row and
//!    returns the grant JSON.
//!
//! **Approval STAYS.** Promotion changes *who* executes an action after
//! approval (a tool vs a human); it never removes the approval
//! requirement on sensitive verbs, and the hire-time
//! dangerous-autonomous-capability fence is untouched.

use super::role_gate::{enforce_role, resolve_role, Action};
use super::routes_auth_principal::principal_id_for_audit;
use super::{body_from_request, store_unavailable};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::user_tenant::UserTenantRole;
use argo_identity::{is_in_probation, IdentityStore, Worker, WorkerId};
use argo_store::executor_promotion::{
    self, PromotionGrant, PromotionRecord, PROMOTABLE_KIND_CU_MACOS, PROMOTABLE_KIND_MCP,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;

/// `POST /api/v1/workers/<id>/actions/<action>/executor`.
/// Body shapes: `{ "kind": "mcp", "server": "<name>", "tool": "<tool>" }`
/// or `{ "kind": "computer_use_native_macos", "target_app": "<bundle-id>" }`
/// (DEC-050/DEC-051 — the CU run loop's risk-tiered floor still holds
/// every committing operation for approval regardless of this grant).
pub(super) async fn post_executor_promote(
    worker_id_raw: &str,
    action_type: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    if action_type.trim().is_empty() {
        return bad_request("missing action segment in path");
    }
    let (tenant, worker) = match preflight(worker_id_raw, req, store).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    if let Err(resp) = enforce_promote(store, &worker, req).await {
        return resp;
    }
    // Probation precondition — a worker still inside its 24h window has
    // no track record yet; refuse to graduate it.
    if is_in_probation(&worker.persona.provenance_chain, chrono::Utc::now()) {
        emit_probation_deny(store, req, &worker, action_type).await;
        return probation_blocked(&worker);
    }
    let binding = match parse_binding(req) {
        Ok(b) => b,
        Err(resp) => return resp,
    };
    let actor = actor_label(req);
    let grant = PromotionGrant {
        tenant_id: tenant.to_string(),
        worker_id: worker.id.to_string(),
        action_type: action_type.to_string(),
        executor_kind: binding.kind,
        mcp_server: binding.server,
        mcp_tool: binding.tool,
        target_app: binding.target_app,
        promoted_by: actor,
    };
    let record = match executor_promotion::promote(store.pool(), &grant).await {
        Ok(r) => r,
        Err(ArgoError::Schema(msg)) => return bad_request(&msg),
        Err(e) => return server_error(&format!("promote: {e}")),
    };
    emit_success(store, req, &worker, &record).await;
    ("200 OK", promotion_response(&record))
}

struct ExecutorBinding {
    kind: String,
    server: Option<String>,
    tool: Option<String>,
    target_app: Option<String>,
}

async fn preflight(
    worker_id_raw: &str,
    req: &str,
    store: &Arc<SessionStore>,
) -> Result<(TenantId, Worker), (&'static str, String)> {
    let worker_id = parse_worker_id(worker_id_raw)?;
    let tenant = resolve_tenant(req, store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    let worker = load_worker(store, tenant, worker_id).await?;
    Ok((tenant, worker))
}

fn parse_worker_id(raw: &str) -> Result<WorkerId, (&'static str, String)> {
    let cleaned = raw.split('?').next().unwrap_or(raw);
    cleaned
        .parse::<WorkerId>()
        .map_err(|e| bad_request(&format!("invalid worker_id '{cleaned}': {e}")))
}

async fn resolve_tenant(req: &str, store: &SessionStore) -> ArgoResult<TenantId> {
    // Identity-first (WS1): the signed-in user's session-JWT tenant wins, then an
    // explicit override, then the cwd fallback for CLI. Previously cwd-only — a real
    // signed-in user's executor-promotion hit the default tenant's worker (real-mode
    // walk cross-tenant fix, pt2). Honouring a promotion at dispatch stays default-OFF
    // (`ARGO_EXECUTION_GRADUATION`); this only records the grant against the right tenant.
    super::resolve_tenant::resolve_tenant_for_request(req, store).await
}

async fn load_worker(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    worker_id: WorkerId,
) -> Result<Worker, (&'static str, String)> {
    match IdentityStore::new(store).get_worker(tenant, worker_id).await {
        Ok(Some(w)) => Ok(w),
        Ok(None) => Err(not_found(&format!("worker {worker_id} not found"))),
        Err(e) => Err(server_error(&format!("get worker: {e}"))),
    }
}

async fn enforce_promote(
    store: &Arc<SessionStore>,
    worker: &Worker,
    req: &str,
) -> Result<(), (&'static str, String)> {
    if let Some(resp) = enforce_role(req, Action::WorkerExecutorPromote) {
        let role_label = resolve_role(req)
            .unwrap_or(UserTenantRole::Guest)
            .as_str();
        emit_rbac_deny(store, req, worker, role_label).await;
        return Err(resp);
    }
    Ok(())
}

/// Parse + validate the executor-binding body. Promotable kinds:
/// `mcp` (requires non-empty `server` + `tool`) and
/// `computer_use_native_macos` (requires non-empty `target_app`).
fn parse_binding(req: &str) -> Result<ExecutorBinding, (&'static str, String)> {
    let body = body_from_request(req);
    let body_trim = body.trim();
    if body_trim.is_empty() {
        return Err(bad_request(
            "missing JSON body { kind, server, tool } or { kind, target_app }",
        ));
    }
    let parsed: Value = serde_json::from_str(body_trim)
        .map_err(|e| bad_request(&format!("invalid JSON body: {e}")))?;
    let kind = parsed
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or(PROMOTABLE_KIND_MCP)
        .trim()
        .to_string();
    if kind == PROMOTABLE_KIND_MCP {
        let server = required_field(&parsed, "server")?;
        let tool = required_field(&parsed, "tool")?;
        return Ok(ExecutorBinding {
            kind,
            server: Some(server),
            tool: Some(tool),
            target_app: None,
        });
    }
    if kind == PROMOTABLE_KIND_CU_MACOS {
        let target_app = required_field(&parsed, "target_app")?;
        return Ok(ExecutorBinding {
            kind,
            server: None,
            tool: None,
            target_app: Some(target_app),
        });
    }
    Err(bad_request(&format!(
        "only '{PROMOTABLE_KIND_MCP}' or '{PROMOTABLE_KIND_CU_MACOS}' are \
         promotable, got '{kind}'"
    )))
}

fn required_field(parsed: &Value, field: &str) -> Result<String, (&'static str, String)> {
    let raw = parsed
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request(&format!("missing required field '{field}'")))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(bad_request(&format!("'{field}' must not be empty")));
    }
    Ok(trimmed.to_string())
}

fn actor_label(req: &str) -> String {
    principal_id_for_audit(req).unwrap_or_else(|| "anonymous".to_string())
}

async fn emit_success(store: &Arc<SessionStore>, req: &str, worker: &Worker, record: &PromotionRecord) {
    let payload = json!({
        "endpoint": "worker.executor_promoted",
        "worker_id": worker.id.to_string(),
        "action_type": record.action_type,
        "executor_kind": record.executor_kind,
        "mcp_server": record.mcp_server,
        "mcp_tool": record.mcp_tool,
        "promoted_by": record.promoted_by,
        "promoted_at": record.promoted_at,
    });
    write_audit(store, req, worker, AuditKind::HttpRequest, payload).await;
}

async fn emit_rbac_deny(store: &Arc<SessionStore>, req: &str, worker: &Worker, role: &str) {
    let payload = json!({
        "endpoint": "worker.executor_promote",
        "worker_id": worker.id.to_string(),
        "role": role,
        "capability": "worker.executor_promote",
    });
    write_audit(store, req, worker, AuditKind::RbacDeny, payload).await;
}

async fn emit_probation_deny(store: &Arc<SessionStore>, req: &str, worker: &Worker, action_type: &str) {
    let payload = json!({
        "endpoint": "worker.executor_promote",
        "worker_id": worker.id.to_string(),
        "action_type": action_type,
        "reason": "worker is still in its probation window; cannot graduate",
    });
    write_audit(store, req, worker, AuditKind::RbacDeny, payload).await;
}

async fn write_audit(
    store: &Arc<SessionStore>,
    req: &str,
    worker: &Worker,
    kind: AuditKind,
    payload: Value,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), worker.tenant_id.to_string());
    if let Err(e) = sink
        .write(None, kind, payload, principal_id_for_audit(req))
        .await
    {
        tracing::warn!(
            target: "gateway::worker_executor_promote",
            error = %e,
            "worker.executor_promote audit write failed"
        );
    }
}

fn promotion_response(record: &PromotionRecord) -> String {
    json!({
        "worker_id": record.worker_id,
        "action_type": record.action_type,
        "executor": {
            "kind": record.executor_kind,
            "server": record.mcp_server,
            "tool": record.mcp_tool,
        },
        "promoted_by": record.promoted_by,
        "promoted_at": record.promoted_at,
    })
    .to_string()
}

fn probation_blocked(worker: &Worker) -> (&'static str, String) {
    (
        "409 Conflict",
        json!({
            "error": {
                "code": "probation_active",
                "message": format!(
                    "worker {} is still in its probation window; an action can only be \
                     graduated after probation completes",
                    worker.id
                ),
            }
        })
        .to_string(),
    )
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
#[path = "routes_worker_executor_promote_tests.rs"]
mod tests;
