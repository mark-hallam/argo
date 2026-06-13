//! `/api/v1/workers/<id>/files` — tenant-scoped file ingress into a
//! teammate's `fs_root` (brief
//! `launch-cloud-file-ingress-upload-to-fs-root`).
//!
//! ## The gap this closes
//!
//! The only end-to-end-verified real-work loop is fenced FILE work inside
//! a teammate's `fs_root` (a folder on the gateway host). On a cloud host
//! the customer has NO way to put a file into that folder, so the proven
//! value loop is dead for every cloud customer. This route is the ingress:
//! an owner uploads a file, it lands ONLY inside the worker's resolved
//! `fs_root` (via [`confine_path`], the canonical jail check), and the next
//! teammate run can `file_read` it — because it is inside the fence.
//!
//! ## Why base64 JSON, not binary multipart
//!
//! The gateway represents the whole HTTP request as a UTF-8 `&str`, so a
//! binary multipart body would already be lossy / corrupt by the time it
//! reaches a handler. base64-in-JSON survives the `&str` round-trip intact,
//! so it is the correct ingress shape for this host. (Multipart is a
//! documented follow-up if/when the gateway grows a bytes-level body
//! channel.)
//!
//! ## Security
//! - **Owner-RBAC** ([`Action::WorkerFileUpload`]) on upload + delete —
//!   only the owner ingests / removes files.
//! - **Tenant-scoped** worker resolve → uniform `404` (existence is never
//!   leaked; a cross-tenant / unknown worker is indistinguishable).
//! - **[`confine_path`] + basename sanitization** → no path traversal: a
//!   `../x` name is refused twice (sanitizer + jail check).
//! - **Per-file size cap** ([`MAX_FILE_BYTES`]) + **per-folder storage
//!   quota** ([`FOLDER_QUOTA_BYTES`]).
//! - **Extension allowlist** ([`ALLOWED_EXTENSIONS`], no executables).
//! - **Audit row** on upload + delete (hash-chained via [`AuditSink`]).

use super::role_gate::{enforce_role, Action};
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Worker, WorkerId};
use argo_store::SessionStore;
use argo_tool::confine_path;
use argo_types::TenantId;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Max bytes for a single uploaded file (decoded). 10 MiB — enough for the
/// documents a teammate works on (invoices, spreadsheets, PDFs) without
/// letting one upload blow the per-folder quota in a single shot.
const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;

/// Per-worker-folder storage quota. Enforced against the live on-disk size
/// of the target `fs_root` plus the incoming file. The brief says
/// "per-tenant storage quota"; the concrete disk an upload consumes is the
/// teammate's own folder, so the quota is enforced at that boundary (a
/// documented, narrower-than-tenant choice — see the PR body).
const FOLDER_QUOTA_BYTES: u64 = 100 * 1024 * 1024;

/// Extension allowlist — documents + data + images a teammate reads. An
/// allowlist (not a denylist) is the safe default: anything not on this
/// list — every executable / script / archive — is refused. Lower-cased
/// before the membership check.
const ALLOWED_EXTENSIONS: &[&str] = &[
    "txt", "md", "markdown", "csv", "tsv", "json", "yaml", "yml", "xml", "html", "htm", "log",
    "pdf", "png", "jpg", "jpeg", "gif", "webp", "svg", "docx", "xlsx", "pptx", "rtf", "ics",
];

/// `POST /api/v1/workers/<id>/files` — ingest one file into the worker's
/// `fs_root`. Body: `{ "filename": "<name.ext>", "content_base64": "<b64>" }`.
pub(super) async fn upload_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(deny) = enforce_role(req, Action::WorkerFileUpload) {
        return deny;
    }
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let (tenant, worker) = match resolve_worker(store, req, worker_id).await {
        Ok(w) => w,
        Err(r) => return r,
    };
    let (filename, bytes) = match parse_upload_body(req) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let fs_root = resolve_fs_root(store, tenant, &worker).await;
    if let Err(e) = std::fs::create_dir_all(&fs_root) {
        return server_error(&format!("create fs_root: {e}"));
    }
    let dest = match confine_path(&fs_root, Path::new(&filename)) {
        Ok(p) => p,
        Err(e) => return bad_request(&format!("path refused: {e}")),
    };
    // Quota: live folder size (minus any file we're about to overwrite) +
    // the incoming bytes must stay under the cap.
    let existing = if dest.exists() {
        std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };
    let projected = folder_size(&fs_root).saturating_sub(existing) + bytes.len() as u64;
    if projected > FOLDER_QUOTA_BYTES {
        return payload_too_large(&format!(
            "folder quota exceeded: {projected} > {FOLDER_QUOTA_BYTES} bytes"
        ));
    }
    if let Err(e) = std::fs::write(&dest, &bytes) {
        return server_error(&format!("write file: {e}"));
    }
    emit_audit(
        store,
        req,
        tenant,
        worker_id,
        "worker.files.uploaded",
        json!({ "filename": filename, "size_bytes": bytes.len() }),
    )
    .await;
    (
        "201 Created",
        json!({ "filename": filename, "size_bytes": bytes.len() }).to_string(),
    )
}

/// `GET /api/v1/workers/<id>/files` — list the worker's `fs_root` (file
/// names + sizes only; never contents).
pub(super) async fn list_response(
    worker_id_raw: &str,
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
    let (tenant, worker) = match resolve_worker(store, req, worker_id).await {
        Ok(w) => w,
        Err(r) => return r,
    };
    let fs_root = resolve_fs_root(store, tenant, &worker).await;
    let mut files = list_files(&fs_root);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let total_bytes: u64 = files.iter().map(|(_, s)| *s).sum();
    let files_json: Vec<Value> = files
        .iter()
        .map(|(name, size)| json!({ "name": name, "size_bytes": size }))
        .collect();
    emit_audit(
        store,
        req,
        tenant,
        worker_id,
        "worker.files.listed",
        json!({ "count": files_json.len() }),
    )
    .await;
    (
        "200 OK",
        json!({
            "files": files_json,
            "total_bytes": total_bytes,
            "quota_bytes": FOLDER_QUOTA_BYTES,
            "max_file_bytes": MAX_FILE_BYTES,
        })
        .to_string(),
    )
}

/// `DELETE /api/v1/workers/<id>/files/<name>` — remove one file from the
/// worker's `fs_root`.
pub(super) async fn delete_response(
    worker_id_raw: &str,
    name_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(deny) = enforce_role(req, Action::WorkerFileUpload) {
        return deny;
    }
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    let (tenant, worker) = match resolve_worker(store, req, worker_id).await {
        Ok(w) => w,
        Err(r) => return r,
    };
    let name = match sanitize_basename(name_raw) {
        Ok(n) => n,
        Err(r) => return r,
    };
    let fs_root = resolve_fs_root(store, tenant, &worker).await;
    let dest = match confine_path(&fs_root, Path::new(&name)) {
        Ok(p) => p,
        Err(e) => return bad_request(&format!("path refused: {e}")),
    };
    if !dest.is_file() {
        return not_found("file not found");
    }
    if let Err(e) = std::fs::remove_file(&dest) {
        return server_error(&format!("delete file: {e}"));
    }
    emit_audit(
        store,
        req,
        tenant,
        worker_id,
        "worker.files.deleted",
        json!({ "filename": name }),
    )
    .await;
    ("204 No Content", String::new())
}

// ----- body parsing --------------------------------------------------------

/// Parse + validate the upload body. Returns the sanitized basename and the
/// decoded bytes, or a ready error envelope.
fn parse_upload_body(req: &str) -> Result<(String, Vec<u8>), (&'static str, String)> {
    let parsed: Value = serde_json::from_str(super::body_from_request(req))
        .map_err(|e| bad_request(&format!("parse JSON body: {e}")))?;
    let filename_raw = parsed
        .get("filename")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request("missing `filename` field"))?;
    let filename = sanitize_basename(filename_raw)?;
    let b64 = parsed
        .get("content_base64")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request("missing `content_base64` field"))?;
    let bytes = STANDARD
        .decode(b64.trim())
        .map_err(|e| bad_request(&format!("invalid base64: {e}")))?;
    if bytes.is_empty() {
        return Err(bad_request("empty file"));
    }
    if bytes.len() > MAX_FILE_BYTES {
        return Err(payload_too_large(&format!(
            "file exceeds {MAX_FILE_BYTES}-byte limit"
        )));
    }
    Ok((filename, bytes))
}

/// Reduce an arbitrary client-supplied name to a single safe basename:
/// trims, rejects empty, takes only the final path component (so
/// `../../etc/passwd` and `a/b.txt` both reduce to their leaf), rejects
/// `.` / `..` / any residual separator / leading-dot (hidden) names, and
/// enforces the extension allowlist. The canonical jail check
/// ([`confine_path`]) re-validates at write/delete time (defense in depth).
fn sanitize_basename(raw: &str) -> Result<String, (&'static str, String)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(bad_request("empty filename"));
    }
    let leaf = Path::new(trimmed)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| bad_request("filename has no basename"))?;
    if leaf == "." || leaf == ".." || leaf.contains('/') || leaf.contains('\\') {
        return Err(bad_request("filename is not a plain basename"));
    }
    if leaf.starts_with('.') {
        return Err(bad_request("hidden / dotfile names are not allowed"));
    }
    // Defense in depth: the sanitized leaf must equal what the client sent
    // (minus surrounding whitespace) — a name that changed under
    // basename-reduction was a traversal attempt.
    if leaf != trimmed {
        return Err(bad_request("filename contains a path; send a plain name"));
    }
    if !extension_allowed(leaf) {
        return Err(bad_request(
            "file type not allowed (allowlist: documents, data, images — no executables)",
        ));
    }
    Ok(leaf.to_string())
}

/// True when the name's lower-cased extension is on [`ALLOWED_EXTENSIONS`].
/// A name with no extension is refused (executables are commonly
/// extension-less).
fn extension_allowed(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|ext| ALLOWED_EXTENSIONS.contains(&ext.as_str()))
}

// ----- tenant-scoped worker + fs_root --------------------------------------

/// Resolve the JWT tenant + load the worker. Cross-tenant / unknown all
/// surface as a uniform `404` — existence is never leaked.
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

/// Resolve the worker's effective `fs_root`. Reads the tenant-scoped
/// `worker.fs_root` column; falls back to the SAME deterministic
/// `/tmp/argo-fs/<worker_id>` default the filesystem executor uses
/// (`worker_actions_executor_stubs_filesystem::resolve_fs_root`) so an
/// uploaded file lands exactly where the teammate's `file_read` will look
/// for it. Tenant ownership is already proven by [`resolve_worker`]; the
/// `tenant_id` clause is belt-and-suspenders.
async fn resolve_fs_root(store: &Arc<SessionStore>, tenant: TenantId, worker: &Worker) -> PathBuf {
    let row: Result<Option<Option<String>>, _> =
        sqlx::query_scalar("SELECT fs_root FROM worker WHERE id = ? AND tenant_id = ?")
            .bind(worker.id.to_string())
            .bind(tenant.to_string())
            .fetch_optional(store.pool())
            .await;
    match row {
        Ok(Some(Some(p))) if !p.trim().is_empty() => PathBuf::from(p),
        // `temp_dir()` not hardcoded `/tmp` (bogus on Windows) — must match the
        // executor's resolve_fs_root fallback. win-path-canonicalization-deep-audit.
        _ => std::env::temp_dir().join("argo-fs").join(worker.id.to_string()),
    }
}

// ----- on-disk helpers -----------------------------------------------------

/// Sum the byte size of the regular files directly inside `dir`. Missing
/// dir → `0`. Top-level only — uploads are flat by construction.
fn folder_size(dir: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    rd.flatten()
        .filter_map(|e| {
            let md = e.metadata().ok()?;
            md.is_file().then_some(md.len())
        })
        .sum()
}

/// List `(name, size_bytes)` for the regular files directly inside `dir`.
/// Missing dir → empty. Hidden (dot-prefixed) entries are skipped so the
/// listing matches what upload allows.
fn list_files(dir: &Path) -> Vec<(String, u64)> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.flatten()
        .filter_map(|e| {
            let md = e.metadata().ok()?;
            if !md.is_file() {
                return None;
            }
            let name = e.file_name().to_str()?.to_string();
            if name.starts_with('.') {
                return None;
            }
            Some((name, md.len()))
        })
        .collect()
}

// ----- id parsing ----------------------------------------------------------

fn parse_worker_id(raw: &str) -> Result<WorkerId, (&'static str, String)> {
    let t = raw.split('?').next().unwrap_or(raw);
    t.parse::<WorkerId>()
        .map_err(|e| bad_request(&format!("invalid worker_id '{t}': {e}")))
}

// ----- audit ---------------------------------------------------------------

async fn emit_audit(
    store: &Arc<SessionStore>,
    req: &str,
    tenant: TenantId,
    worker_id: WorkerId,
    endpoint: &str,
    extra: Value,
) {
    let mut payload = json!({
        "endpoint": endpoint,
        "tenant_id": tenant.to_string(),
        "worker_id": worker_id.to_string(),
    });
    if let (Some(map), Some(obj)) = (payload.as_object_mut(), extra.as_object()) {
        for (k, v) in obj {
            map.insert(k.clone(), v.clone());
        }
    }
    if let Err(e) = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string())
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            principal_id_for_audit(req),
        )
        .await
    {
        tracing::warn!(target: "gateway::worker_files", error = %e, "audit write failed");
    }
}

// ----- error envelopes -----------------------------------------------------

fn bad_request(message: &str) -> (&'static str, String) {
    envelope("400 Bad Request", "bad_request", message)
}
fn not_found(message: &str) -> (&'static str, String) {
    envelope("404 Not Found", "not_found", message)
}
fn payload_too_large(message: &str) -> (&'static str, String) {
    envelope("413 Payload Too Large", "payload_too_large", message)
}
fn server_error(message: &str) -> (&'static str, String) {
    envelope("500 Internal Server Error", "internal_error", message)
}
fn envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({ "error": { "code": code, "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_worker_files_tests.rs"]
mod tests;
