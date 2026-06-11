//! `GET /api/v1/workers/<id>/runs/<job_id>/output` — surface a single
//! teammate run's real work product to the SPA (brief
//! `uf-spa-teammate-output-surface`).
//!
//! ## Problem this closes
//!
//! A teammate's actual output (`assistant_output.md`, the proposed
//! `actions.json`, the `result.json` pointers + preview) lands in an
//! on-disk artifact bundle under `.skycot/outputs/<job_id>/` on the
//! gateway host. Before this route the only thing a SPA user ever saw of
//! the work was an `/audit` row (a `chars` count). This route lets the
//! SPA render "here is what your teammate produced" without ever
//! touching the disk directly.
//!
//! ## Tenant scoping (the security crux)
//!
//! The bundle directory path is read from the `completed_jobs` memory
//! row the run wrote — and that lookup is scoped to the caller's tenant
//! by construction, mirroring `routes_worker_run_exec`:
//!
//! 1. Resolve the JWT tenant ([`resolve_tenant_for_request`]).
//! 2. Load the worker by `(tenant, worker_id)` — a cross-tenant or
//!    unknown worker surfaces a uniform `404` (existence is not leaked).
//! 3. Resolve the worker's tenant-owned workspace
//!    ([`SessionStore::first_workspace_for_tenant`], cwd fallback for the
//!    anonymous/demo path).
//! 4. Find the `completed_jobs` row for `job_id` within
//!    `(workspace, worker.vertical_id)`. A `job_id` belonging to another
//!    tenant's bundle is not in this scope, so it `404`s.
//!
//! Because the `output_dir` is read from trusted memory (never from the
//! request) and `job_id` is validated as a UUID, there is no path
//! traversal: the route only reads three fixed filenames joined to the
//! recorded bundle dir, and never executes anything.

use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

/// Cap the assistant text we echo into the JSON response. A run's full
/// reasoning can be large; the SPA's "Output" tab shows a preview and
/// the bundle stays the source of truth for the full text. Bytes, not
/// chars — a defensive bound on response size.
const MAX_ASSISTANT_BYTES: usize = 200_000;

/// `GET /api/v1/workers/<id>/runs/<job_id>/output` entry point.
/// `worker_id_raw` / `job_id_raw` are the already-split path segments.
pub(super) async fn output_response(
    worker_id_raw: &str,
    job_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let job_id = match parse_job_id(job_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let (tenant, worker) = match resolve_worker(store, req, worker_id).await {
        Ok(w) => w,
        Err(r) => return r,
    };
    let bundle = match find_job_bundle(store, &worker, &job_id).await {
        Ok(Some(b)) => b,
        // Unknown / cross-tenant / still-pending job → uniform 404.
        Ok(None) => return not_found("run output not found"),
        Err(e) => return server_error(&format!("read run output: {e}")),
    };
    let body = build_output_json(&worker, &job_id, &bundle);
    emit_view_audit(store, req, tenant, worker_id, &job_id).await;
    ("200 OK", body.to_string())
}

// ----- id parsing ----------------------------------------------------------

fn parse_worker_id(raw: &str) -> Result<WorkerId, (&'static str, String)> {
    let t = raw.split('?').next().unwrap_or(raw);
    t.parse::<WorkerId>()
        .map_err(|e| bad_request(&format!("invalid worker_id '{t}': {e}")))
}

fn parse_job_id(raw: &str) -> Result<Uuid, (&'static str, String)> {
    let t = raw.split('?').next().unwrap_or(raw);
    Uuid::parse_str(t).map_err(|e| bad_request(&format!("invalid job_id '{t}': {e}")))
}

// ----- tenant-scoped worker resolution -------------------------------------

/// Resolve the JWT tenant + load the worker. Cross-tenant / unknown /
/// archived all surface as a uniform `404` — existence is never leaked,
/// matching the worker-detail + worker-activity routes.
async fn resolve_worker(
    store: &Arc<SessionStore>,
    req: &str,
    worker_id: WorkerId,
) -> Result<(TenantId, Worker), (&'static str, String)> {
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    match IdentityStore::new(store).get_worker(tenant, worker_id).await {
        Ok(Some(worker)) => Ok((tenant, worker)),
        Ok(None) => Err(not_found("worker not found")),
        Err(e) => Err(server_error(&format!("get worker: {e}"))),
    }
}

// ----- bundle lookup + read ------------------------------------------------

/// The parsed pieces of one run's artifact bundle, plus the metadata
/// from its `completed_jobs` memory row (which survives even if the disk
/// files were pruned).
struct RunBundle {
    output_dir: Option<PathBuf>,
    completed_at: String,
    stop_reason: String,
    stop_detail: String,
    steps_completed: Option<u64>,
    actions_emitted: Option<u64>,
    session_id: String,
}

/// Find the `completed_jobs` memory row for `job_id` within the worker's
/// tenant-owned `(workspace, vertical_id)` scope, then read the on-disk
/// bundle it points at. `Ok(None)` when no such completed run exists in
/// this tenant's scope (the uniform-404 path).
async fn find_job_bundle(
    store: &Arc<SessionStore>,
    worker: &Worker,
    job_id: &Uuid,
) -> ArgoResult<Option<RunBundle>> {
    let workspace = resolve_workspace(store, worker.tenant_id).await?;
    let memory = MemoryStore::new(Arc::clone(store));
    let entries = memory
        .list_for_vertical(workspace, MemoryFile::Memory, &worker.vertical_id)
        .await?;
    let target = job_id.to_string();
    // Newest-first: the most recent completion row for this job wins
    // (mirrors `routes_worker_run_exec::read_completion`).
    for e in entries.iter().rev() {
        let Ok(body) = serde_json::from_str::<Value>(&e.body) else {
            continue;
        };
        if body.get("schema_key").and_then(Value::as_str) != Some("completed_jobs") {
            continue;
        }
        if body.get("pending_job_id").and_then(Value::as_str) != Some(target.as_str()) {
            continue;
        }
        let output_dir = body
            .get("output_dir")
            .and_then(Value::as_str)
            .map(PathBuf::from);
        return Ok(Some(RunBundle {
            output_dir,
            completed_at: str_field(&body, "completed_at"),
            stop_reason: str_field(&body, "stop_reason"),
            stop_detail: str_field(&body, "stop_detail"),
            steps_completed: body.get("steps_completed").and_then(Value::as_u64),
            actions_emitted: body.get("actions_emitted").and_then(Value::as_u64),
            session_id: str_field(&body, "session_id"),
        }));
    }
    Ok(None)
}

/// Resolve the workspace that owns `tenant`. Prefers the tenant-scoped
/// workspace (a signed-in stranger's run anchors to THEIR workspace),
/// falling back to the cwd workspace for the demo / anonymous path —
/// the same resolution `routes_worker_run_exec::resolve_workspace` uses
/// when writing the completion row, so reads land in the same scope.
async fn resolve_workspace(
    store: &Arc<SessionStore>,
    tenant: TenantId,
) -> ArgoResult<WorkspaceId> {
    if let Some(ws) = store.first_workspace_for_tenant(tenant).await? {
        return Ok(ws);
    }
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let (_tenant, ws) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .map_err(|e| ArgoError::Provider(format!("ensure_workspace: {e}")))?;
    Ok(ws)
}

/// Read one named file from the bundle dir, capping returned bytes.
/// Missing / unreadable → `None` (the SPA renders the field absent
/// rather than failing the whole response).
fn read_bundle_text(dir: &Path, filename: &str, cap: usize) -> Option<String> {
    let body = std::fs::read_to_string(dir.join(filename)).ok()?;
    if body.len() > cap {
        let mut truncated: String = body.chars().take(cap).collect();
        truncated.push_str("\n\n…(truncated — open the bundle for the full text)");
        Some(truncated)
    } else {
        Some(body)
    }
}

/// Parse one JSON file from the bundle dir. Missing / malformed →
/// `None`.
fn read_bundle_json(dir: &Path, filename: &str) -> Option<Value> {
    let body = std::fs::read_to_string(dir.join(filename)).ok()?;
    serde_json::from_str(&body).ok()
}

// ----- response shaping ----------------------------------------------------

fn build_output_json(worker: &Worker, job_id: &Uuid, bundle: &RunBundle) -> Value {
    let (assistant_output, result, actions) = match bundle.output_dir.as_deref() {
        Some(dir) => (
            read_bundle_text(dir, "assistant_output.md", MAX_ASSISTANT_BYTES),
            read_bundle_json(dir, "result.json"),
            read_bundle_json(dir, "actions.json"),
        ),
        None => (None, None, None),
    };
    json!({
        "job_id": job_id.to_string(),
        "worker_id": worker.id.to_string(),
        "worker_name": worker.persona.name,
        "session_id": opt_str(&bundle.session_id),
        "completed_at": opt_str(&bundle.completed_at),
        "stop_reason": opt_str(&bundle.stop_reason),
        "stop_detail": opt_str(&bundle.stop_detail),
        "steps_completed": bundle.steps_completed,
        "actions_emitted": bundle.actions_emitted,
        // The teammate's real work product, read from the bundle.
        // `null` for any artifact not present on disk (e.g. an older run
        // whose bundle was pruned) — the metadata above still renders.
        "assistant_output": assistant_output,
        "result": result,
        "actions": actions,
    })
}

fn str_field(body: &Value, key: &str) -> String {
    body.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// `""` → JSON `null`, otherwise the string.
fn opt_str(s: &str) -> Value {
    if s.is_empty() {
        Value::Null
    } else {
        Value::String(s.to_string())
    }
}

// ----- audit ---------------------------------------------------------------

async fn emit_view_audit(
    store: &Arc<SessionStore>,
    req: &str,
    tenant: TenantId,
    worker_id: WorkerId,
    job_id: &Uuid,
) {
    let payload = json!({
        "endpoint": "worker.output.viewed",
        "worker_id": worker_id.to_string(),
        "job_id": job_id.to_string(),
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(None, AuditKind::HttpRequest, payload, principal_id_for_audit(req))
        .await
    {
        tracing::warn!(
            target: "gateway::worker_outputs",
            error = %e,
            "worker.output.viewed audit write failed"
        );
    }
}

// ----- error envelopes -----------------------------------------------------

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
#[path = "routes_worker_outputs_tests.rs"]
mod tests;
