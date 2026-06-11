//! `POST|DELETE /api/v1/workers/<id>/capability-grants[/<tool>]` — the
//! **durable** per-worker capability grant + revoke (tg3, operator-directed
//! tiered-gating).
//!
//! ## Why this exists
//!
//! Granting today is ONE-SHOT only: `POST .../run` with `approve_tools` mints
//! session grants via `filter_to_worker_tool_set`
//! ([`super::routes_worker_run_exec`]) that die with the session. The locked
//! G2-Elevated grant card promises a DURABLE decision ("Nora can now read
//! Contacts — revocable any time"). Re-approving every run is the
//! assistant-not-teammate failure mode. This route makes the card's
//! **Connect** durable, and adds revoke.
//!
//! ## How the grant becomes durable (zero new runtime wiring)
//!
//! The grant rewrites the worker's tenant-scoped Genesis manifest on disk,
//! appending the granted tool to `[tools].allowed`. The runtime already reads
//! that manifest each run via `argo_vertical::load_for_worker` →
//! `argo_capability::WorkerCapabilityProfile::derive` → `permits_tool`, so a
//! previously-fenced tool simply passes on the NEXT run. We REUSE that exact
//! derivation rather than inventing a parallel grant store.
//!
//! ## Safety invariants
//!
//! - **Owner only** ([`Action::WorkerCapabilityGrant`]) — it loosens the
//!   least-privilege fence, so it is as sensitive as executor-promotion.
//! - **Tenant-scoped, fail-closed** — the worker loads via the uniform-404
//!   `IdentityStore::get_worker`, and the manifest resolves via
//!   `load_for_worker` (which never crosses tenants). Only the per-tenant
//!   Genesis pack (`verticals/.genesis/<tenant>/<vid>/`) is writable; a flat
//!   SHARED vertical is refused (rewriting it would leak the grant to every
//!   tenant using that role).
//! - **Deny-wins, never bypassable** — an [`argo_genesis::ALWAYS_DENIED_RUNTIME_TOOLS`]
//!   tool (`code_execute` / `delegate_task` / `file_patch` / `process` /
//!   `send_message` / `terminal`) is refused with an explicit error even when
//!   requested, and after the append the always-denied strip is re-applied so
//!   a colliding entry can never resurrect one. Tier-3 (computer-use) stays
//!   the `executor_promotion` path (DEC-051) — this route never touches it.
//! - **Audited both ways** — grant + revoke each emit an audit row carrying
//!   the grantor principal + scope (tool + access), never a secret.

use super::role_gate::{enforce_role, resolve_role, Action};
use super::routes_auth_principal::session_principal_id_for_audit;
use super::{body_from_request, store_unavailable};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::user_tenant::UserTenantRole;
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use argo_vertical::{LoadedVertical, ToolsSection};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

/// `POST /api/v1/workers/<id>/capability-grants`.
/// Body: `{ "tool_or_connector": "<name>", "access": "read"|"write",
/// "request_id"?: "<tg1 capability_request id>" }`.
pub(super) async fn post_capability_grant(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let (tenant, worker) = match preflight(worker_id_raw, req, store).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let grant = match parse_grant(req) {
        Ok(g) => g,
        Err(resp) => return resp,
    };
    let tool = resolve_tool(&grant.resource, grant.access);
    // Security pin: an always-denied tool is refused with an explicit error,
    // never silently appended.
    if argo_genesis::ALWAYS_DENIED_RUNTIME_TOOLS.contains(&tool.to_ascii_lowercase().as_str()) {
        emit_grant_audit(store, req, &worker, AuditKind::RbacDeny, "denied", &tool, grant.access, grant.request_id.as_deref()).await;
        return always_denied(&tool);
    }
    let mut loaded = match load_genesis_pack(tenant, &worker).await {
        Ok(l) => l,
        Err(resp) => return resp,
    };
    // Deny-wins: a tool the manifest explicitly denies (always-denied set OR a
    // structural sensitive verb) can never be granted via this route.
    if manifest_denies(&loaded, &tool) {
        emit_grant_audit(store, req, &worker, AuditKind::RbacDeny, "denied", &tool, grant.access, grant.request_id.as_deref()).await;
        return always_denied(&tool);
    }
    if let Err(e) = append_and_persist(&mut loaded, &tool) {
        return server_error(&format!("persist grant: {e}"));
    }
    emit_grant_audit(store, req, &worker, AuditKind::HttpRequest, "granted", &tool, grant.access, grant.request_id.as_deref()).await;
    ("200 OK", grant_response(&worker, &tool, grant.access, "granted"))
}

/// `DELETE /api/v1/workers/<id>/capability-grants/<tool>` — remove a durable
/// grant, restoring the deny-by-default fence for that tool on the next run.
pub(super) async fn delete_capability_grant(
    worker_id_raw: &str,
    tool_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let tool = tool_raw.trim();
    if tool.is_empty() {
        return bad_request("missing tool segment in path");
    }
    let (tenant, worker) = match preflight(worker_id_raw, req, store).await {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let mut loaded = match load_genesis_pack(tenant, &worker).await {
        Ok(l) => l,
        Err(resp) => return resp,
    };
    let removed = match remove_and_persist(&mut loaded, tool) {
        Ok(r) => r,
        Err(e) => return server_error(&format!("persist revoke: {e}")),
    };
    let outcome = if removed { "revoked" } else { "not_granted" };
    emit_revoke_audit(store, req, &worker, tool, outcome).await;
    ("200 OK", revoke_response(&worker, tool, outcome))
}

/// `read` → read-side capability; `write` → write-side capability. Mapped onto
/// the runtime's EXISTING read/write tool split (`file_read` / `file_write`),
/// never a new access taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Access {
    Read,
    Write,
}

impl Access {
    fn as_str(self) -> &'static str {
        match self {
            Access::Read => "read",
            Access::Write => "write",
        }
    }
}

struct GrantBody {
    resource: String,
    access: Access,
    request_id: Option<String>,
}

/// Map `(resource, access)` onto the concrete runtime tool name appended to
/// `[tools].allowed`. The file family resolves from `access` via the existing
/// `WorkerCapabilityProfile` split (`file_read` / `file_write`); every other
/// resource is a literal tool / connector name and `access` is recorded for
/// audit only.
fn resolve_tool(resource: &str, access: Access) -> String {
    match resource.to_ascii_lowercase().as_str() {
        "file" | "files" | "filesystem" | "file_read" | "file_write" => match access {
            Access::Read => "file_read".to_string(),
            Access::Write => "file_write".to_string(),
        },
        _ => resource.to_string(),
    }
}

fn parse_grant(req: &str) -> Result<GrantBody, (&'static str, String)> {
    let body = body_from_request(req);
    let body_trim = body.trim();
    if body_trim.is_empty() {
        return Err(bad_request(
            "missing JSON body { tool_or_connector, access }",
        ));
    }
    let parsed: Value = serde_json::from_str(body_trim)
        .map_err(|e| bad_request(&format!("invalid JSON body: {e}")))?;
    let resource = parsed
        .get("tool_or_connector")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| bad_request("missing required field 'tool_or_connector'"))?
        .to_string();
    let access = match parsed.get("access").and_then(Value::as_str).map(str::trim) {
        Some("read") => Access::Read,
        Some("write") => Access::Write,
        _ => return Err(bad_request("'access' must be \"read\" or \"write\"")),
    };
    let request_id = parsed
        .get("request_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Ok(GrantBody {
        resource,
        access,
        request_id,
    })
}

/// Parse id + tenant + load worker (uniform 404 / owner RBAC). Shared by grant
/// + revoke so both gate identically.
async fn preflight(
    worker_id_raw: &str,
    req: &str,
    store: &Arc<SessionStore>,
) -> Result<(TenantId, Worker), (&'static str, String)> {
    let worker_id = parse_worker_id(worker_id_raw)?;
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    let worker = load_worker(store, tenant, worker_id).await?;
    enforce_grant_role(store, &worker, req).await?;
    Ok((tenant, worker))
}

fn parse_worker_id(raw: &str) -> Result<WorkerId, (&'static str, String)> {
    let cleaned = raw.split('?').next().unwrap_or(raw);
    cleaned
        .parse::<WorkerId>()
        .map_err(|e| bad_request(&format!("invalid worker_id '{cleaned}': {e}")))
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

async fn enforce_grant_role(
    store: &Arc<SessionStore>,
    worker: &Worker,
    req: &str,
) -> Result<(), (&'static str, String)> {
    if let Some(resp) = enforce_role(req, Action::WorkerCapabilityGrant) {
        let role = resolve_role(req).unwrap_or(UserTenantRole::Guest).as_str();
        let payload = json!({
            "endpoint": "worker.capability_grant",
            "worker_id": worker.id.to_string(),
            "role": role,
            "capability": "worker.capability_grant",
        });
        write_audit(store, req, worker, AuditKind::RbacDeny, payload).await;
        return Err(resp);
    }
    Ok(())
}

/// Resolve the worker's manifest, tenant-scoped, and require it to be the
/// per-tenant Genesis pack. A flat SHARED vertical is refused — rewriting its
/// `[tools].allowed` would leak the grant to every tenant using that role.
async fn load_genesis_pack(
    tenant: TenantId,
    worker: &Worker,
) -> Result<LoadedVertical, (&'static str, String)> {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let Some(loaded) =
        argo_vertical::load_for_worker(&dir, &tenant.to_string(), worker.vertical_id.as_str())
    else {
        return Err(not_found(&format!(
            "worker {} has no resolvable role",
            worker.id
        )));
    };
    if !is_genesis_pack(&loaded.root) {
        return Err((
            "409 Conflict",
            json!({ "error": {
                "code": "shared_role_not_grantable",
                "message": "this teammate runs a shared catalog role; durable \
                            per-worker grants apply only to a hired (Genesis) teammate",
            }})
            .to_string(),
        ));
    }
    Ok(loaded)
}

/// `true` when `root` is a tenant-scoped Genesis pack
/// (`verticals/.genesis/<tenant>/<vid>/`) rather than a flat shared vertical.
fn is_genesis_pack(root: &Path) -> bool {
    root.components()
        .any(|c| c.as_os_str() == std::ffi::OsStr::new(".genesis"))
}

/// `true` when the manifest's `[tools].denied` already lists `tool` — deny
/// wins, so it can never be granted via this route.
fn manifest_denies(loaded: &LoadedVertical, tool: &str) -> bool {
    loaded
        .manifest
        .tools
        .as_ref()
        .is_some_and(|t| t.denied.iter().any(|d| d == tool))
}

/// Append `tool` to `[tools].allowed` (de-duped, always-denied stripped) and
/// persist the manifest back to disk so the runtime honours it next run.
fn append_and_persist(loaded: &mut LoadedVertical, tool: &str) -> ArgoResult<()> {
    let tools = loaded.manifest.tools.get_or_insert_with(ToolsSection::default);
    if !tools.allowed.iter().any(|t| t == tool) {
        tools.allowed.push(tool.to_string());
    }
    strip_always_denied(tools);
    persist(loaded)
}

/// Remove `tool` from `[tools].allowed` and persist. Returns `true` if a grant
/// was actually present (idempotent revoke otherwise).
fn remove_and_persist(loaded: &mut LoadedVertical, tool: &str) -> ArgoResult<bool> {
    let Some(tools) = loaded.manifest.tools.as_mut() else {
        return Ok(false);
    };
    let before = tools.allowed.len();
    tools.allowed.retain(|t| t != tool);
    let removed = tools.allowed.len() != before;
    if removed {
        persist(loaded)?;
    }
    Ok(removed)
}

/// Re-apply the deny-wins invariant at the source: no always-denied tool may
/// survive in `allowed` regardless of what was appended.
fn strip_always_denied(tools: &mut ToolsSection) {
    tools
        .allowed
        .retain(|t| !argo_genesis::ALWAYS_DENIED_RUNTIME_TOOLS.contains(&t.to_ascii_lowercase().as_str()));
}

fn persist(loaded: &LoadedVertical) -> ArgoResult<()> {
    let body = toml::to_string_pretty(&loaded.manifest)
        .map_err(|e| ArgoError::Schema(format!("serialize manifest: {e}")))?;
    std::fs::write(loaded.root.join("vertical.toml"), body)
        .map_err(|e| ArgoError::Io(format!("write vertical.toml: {e}")))
}

#[allow(clippy::too_many_arguments)]
async fn emit_grant_audit(
    store: &Arc<SessionStore>,
    req: &str,
    worker: &Worker,
    kind: AuditKind,
    outcome: &str,
    tool: &str,
    access: Access,
    request_id: Option<&str>,
) {
    let payload = json!({
        "endpoint": "worker.capability_granted",
        "worker_id": worker.id.to_string(),
        "tool": tool,
        "access": access.as_str(),
        "outcome": outcome,
        "request_id": request_id,
        "granted_by": session_principal_id_for_audit(req),
    });
    write_audit(store, req, worker, kind, payload).await;
}

async fn emit_revoke_audit(
    store: &Arc<SessionStore>,
    req: &str,
    worker: &Worker,
    tool: &str,
    outcome: &str,
) {
    let payload = json!({
        "endpoint": "worker.capability_revoked",
        "worker_id": worker.id.to_string(),
        "tool": tool,
        "outcome": outcome,
        "revoked_by": session_principal_id_for_audit(req),
    });
    write_audit(store, req, worker, AuditKind::HttpRequest, payload).await;
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
        .write(None, kind, payload, session_principal_id_for_audit(req))
        .await
    {
        tracing::warn!(
            target: "gateway::worker_capability_grant",
            error = %e,
            "worker.capability_grant audit write failed"
        );
    }
}

fn grant_response(worker: &Worker, tool: &str, access: Access, outcome: &str) -> String {
    json!({
        "worker_id": worker.id.to_string(),
        "tool": tool,
        "access": access.as_str(),
        "outcome": outcome,
    })
    .to_string()
}

fn revoke_response(worker: &Worker, tool: &str, outcome: &str) -> String {
    json!({
        "worker_id": worker.id.to_string(),
        "tool": tool,
        "outcome": outcome,
    })
    .to_string()
}

fn always_denied(tool: &str) -> (&'static str, String) {
    (
        "403 Forbidden",
        json!({ "error": {
            "code": "always_denied_tool",
            "message": format!(
                "'{tool}' is an always-denied runtime tool (shell/exec/spawn/\
                 patch/transmit) and can never be granted",
            ),
        }})
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
#[path = "routes_worker_capability_grant_tests.rs"]
mod tests;
