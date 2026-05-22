//! Audit evidence-bundle endpoints
//! (`web-gateway-endpoints-audit-mutations-batch`).
//!
//! Three routes:
//!
//! - `POST /api/v1/audit/export` — build an evidence bundle (NDJSON
//!   / CSV / tar.gz) for an `[from, to]` audit window. Returns a
//!   short-lived (5 min) HMAC-signed download URL. The bundle is
//!   written to disk under the configured export root.
//! - `POST /api/v1/audit/export/preview` — count + sample of what
//!   would land in the bundle. No file generated.
//! - `GET /api/v1/audit/export/<bundle_id>?format=…&expires=…&tenant=…&sig=…` —
//!   download a previously-built bundle. Verifies the HMAC over
//!   `bundle_id|format|expires|tenant_id`, matches the URL's `tenant`
//!   query against the caller's `X-Argo-Tenant-Id` header
//!   (existence-hide 404 on mismatch — see
//!   `mission-audit-export-tenant-scope-fix`), then reads bytes from
//!   a per-tenant subdirectory of the export root. Refuses expired
//!   or unsigned URLs.
//!
//! The on-disk file is the procurement-grade evidence artefact —
//! its SHA-256 + size are recorded on the manifest entry returned
//! by the POST so an external chain-of-custody system can verify
//! the bytes match what the gateway built.

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use super::store_unavailable;
use argo_audit::export_bundle::{
    build_bundle, fetch_preview, fetch_rows_for_export, BundleFormat, ExportPreview, ExportWindow,
};
use argo_audit::{AuditKind, AuditSink, DEFAULT_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use signing::{
    is_safe_bundle_id, query_kv, request_tenant_id, sha256_hex, sign_export_url, tenant_export_dir,
    url_decode, verify_signature,
};

#[path = "routes_audit_export_signing.rs"]
mod signing;

/// Lifetime of a freshly-signed export URL, in seconds (5 minutes).
pub(super) const EXPORT_URL_TTL_SECS: i64 = 300;

/// `POST /api/v1/audit/export` — body `{ from?, to?, format }`.
pub(super) async fn export_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let parsed: Value = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let format = match parse_format(&parsed) {
        Ok(f) => f,
        Err(msg) => return bad_request(&msg),
    };
    let window = match parse_window(&parsed) {
        Ok(w) => w,
        Err(msg) => return bad_request(&msg),
    };
    let tenant_id = request_tenant_id(req);
    let rows = match fetch_rows_for_export(store, &tenant_id, &window).await {
        Ok(r) => r,
        Err(e) => return server_error(&format!("fetch rows: {e}")),
    };
    let bytes = match build_bundle(&rows, format) {
        Ok(b) => b,
        Err(e) => return server_error(&format!("build bundle: {e}")),
    };
    let sha256 = sha256_hex(&bytes);
    let bundle_id = Uuid::now_v7().simple().to_string();
    // `mission-audit-export-tenant-scope-fix`: bundles live under a
    // per-tenant subdir so a forged signed URL minted with another
    // tenant's id cannot read these bytes — the lookup at GET time
    // is scoped to the caller's tenant directory.
    let Some(tenant_dir) = tenant_export_dir(&tenant_id) else {
        return bad_request("invalid tenant id");
    };
    if let Err(e) = std::fs::create_dir_all(&tenant_dir) {
        return server_error(&format!(
            "create export dir {}: {e}",
            tenant_dir.display()
        ));
    }
    let filename = format!("{bundle_id}.{}", format.extension());
    let bundle_path = tenant_dir.join(&filename);
    if let Err(e) = std::fs::write(&bundle_path, &bytes) {
        return server_error(&format!("write {}: {e}", bundle_path.display()));
    }
    let expires = Utc::now().timestamp() + EXPORT_URL_TTL_SECS;
    let download_url = sign_export_url(&bundle_id, format, expires, &tenant_id);
    emit_export_audit(store, req, &bundle_id, format, rows.len(), &sha256).await;
    let body = json!({
        "bundle_id": bundle_id,
        "format": format,
        "row_count": rows.len(),
        "byte_size": bytes.len(),
        "sha256": sha256,
        "download_url": download_url,
        "expires_at": DateTime::<Utc>::from_timestamp(expires, 0)
            .map(|t| t.to_rfc3339())
            .unwrap_or_default(),
        "ttl_seconds": EXPORT_URL_TTL_SECS,
    });
    ("201 Created", body.to_string())
}

/// `POST /api/v1/audit/export/preview` — body `{ from?, to?, format }`.
pub(super) async fn preview_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let parsed: Value = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    // Preview accepts the same body shape as export; `format` is
    // optional here because the row set is format-independent.
    let format = parsed
        .get("format")
        .and_then(Value::as_str)
        .map_or(BundleFormat::Json, |s| {
            BundleFormat::parse(s).unwrap_or(BundleFormat::Json)
        });
    let window = match parse_window(&parsed) {
        Ok(w) => w,
        Err(msg) => return bad_request(&msg),
    };
    let tenant_id = request_tenant_id(req);
    let preview: ExportPreview = match fetch_preview(store, &tenant_id, &window).await {
        Ok(p) => p,
        Err(e) => return server_error(&format!("fetch preview: {e}")),
    };
    let body = json!({
        "row_count": preview.row_count,
        "byte_estimate": preview.byte_estimate,
        "format": format,
        "sample": preview.sample,
    });
    ("200 OK", body.to_string())
}

/// `GET /api/v1/audit/export/<bundle_id>?format=<f>&expires=<unix>&tenant=<id>&sig=<hex>`.
/// Returns the bundle bytes (base64-wrapped — the v1 listener
/// hard-codes JSON content-type; mirrors the screenshot endpoint).
///
/// Cross-tenant defense (`mission-audit-export-tenant-scope-fix`): the
/// signed `tenant` query value is the tenant the bundle was issued
/// for; the caller's `X-Argo-Tenant-Id` request header must match it,
/// otherwise the handler returns 404 (existence-hide pattern, per
/// #2040). The HMAC also incorporates the tenant, so a cross-tenant
/// forgery without the export secret fails signature verification.
pub(super) async fn download_response(
    req: &str,
    bundle_id: &str,
    query: &str,
    _store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let format = match query_kv(query, "format").as_deref().and_then(BundleFormat::parse) {
        Some(f) => f,
        None => return bad_request("missing or invalid format query"),
    };
    let Some(expires) = query_kv(query, "expires").and_then(|v| v.parse::<i64>().ok()) else {
        return unauthorized("missing or invalid expires");
    };
    let Some(signed_tenant) = query_kv(query, "tenant").and_then(|s| url_decode(&s)) else {
        return not_found(bundle_id);
    };
    let Some(sig) = query_kv(query, "sig") else {
        return unauthorized("missing sig");
    };
    // Caller-tenant binding check (mission-audit-export-tenant-scope-fix).
    // Mismatch is 404, not 403 — same existence-hide pattern used by
    // routes_worker_skills per #2040.
    let caller_tenant = request_tenant_id(req);
    if caller_tenant != signed_tenant {
        return not_found(bundle_id);
    }
    if expires <= Utc::now().timestamp() {
        return unauthorized("signature expired");
    }
    if !verify_signature(bundle_id, format, expires, &signed_tenant, &sig) {
        return unauthorized("signature mismatch");
    }
    if !is_safe_bundle_id(bundle_id) {
        return bad_request("invalid bundle id");
    }
    // Scope the read to the caller's tenant directory so a forged
    // signed URL cannot read bytes another tenant wrote
    // (`mission-audit-export-tenant-scope-fix`).
    let Some(tenant_dir) = tenant_export_dir(&caller_tenant) else {
        return not_found(bundle_id);
    };
    let path = tenant_dir.join(format!("{bundle_id}.{}", format.extension()));
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return not_found(bundle_id),
        Err(e) => return server_error(&format!("read {}: {e}", path.display())),
    };
    let body = json!({
        "bundle_id": bundle_id,
        "format": format,
        "content_type": format.content_type(),
        "size_bytes": bytes.len(),
        "sha256": sha256_hex(&bytes),
        "b64": general_purpose::STANDARD.encode(&bytes),
    });
    ("200 OK", body.to_string())
}

fn parse_format(body: &Value) -> Result<BundleFormat, String> {
    let raw = body
        .get("format")
        .and_then(Value::as_str)
        .ok_or_else(|| "`format` (string) is required".to_string())?;
    BundleFormat::parse(raw)
        .ok_or_else(|| format!("unknown format `{raw}` (allowed: json | csv | tar)"))
}

fn parse_window(body: &Value) -> Result<ExportWindow, String> {
    let from = parse_optional_timestamp(body.get("from"), "from")?;
    let to = parse_optional_timestamp(body.get("to"), "to")?;
    if let (Some(lo), Some(hi)) = (from, to) {
        if lo > hi {
            return Err("`from` must be ≤ `to`".to_string());
        }
    }
    Ok(ExportWindow { from, to })
}

fn parse_optional_timestamp(v: Option<&Value>, label: &str) -> Result<Option<DateTime<Utc>>, String> {
    let Some(v) = v else {
        return Ok(None);
    };
    if v.is_null() {
        return Ok(None);
    }
    let s = v
        .as_str()
        .ok_or_else(|| format!("`{label}` must be an RFC-3339 string"))?;
    let parsed: DateTime<Utc> = s
        .parse()
        .map_err(|e| format!("`{label}` parse: {e}"))?;
    Ok(Some(parsed))
}

async fn emit_export_audit(
    store: &Arc<SessionStore>,
    req: &str,
    bundle_id: &str,
    format: BundleFormat,
    row_count: usize,
    sha256: &str,
) {
    let payload = json!({
        "endpoint": "audit.export",
        "bundle_id": bundle_id,
        "format": format,
        "row_count": row_count,
        "sha256": sha256,
    });
    let tenant = resolve_tenant(store)
        .await
        .unwrap_or_else(|| DEFAULT_AUDIT_TENANT_ID.to_string());
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant);
    if let Err(e) = sink
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            principal_id_for_audit(req),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::audit_export",
            error = %e,
            "audit.export write failed",
        );
    }
}

async fn resolve_tenant(store: &Arc<SessionStore>) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await.ok()?;
    Some(tenant.0.to_string())
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn unauthorized(message: &str) -> (&'static str, String) {
    error_envelope("401 Unauthorized", "unauthorized", message)
}

fn not_found(id: &str) -> (&'static str, String) {
    error_envelope(
        "404 Not Found",
        "not_found",
        &format!("bundle `{id}` not found"),
    )
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({
            "error": { "code": code, "message": message }
        })
        .to_string(),
    )
}

/// Dispatcher for the GET download path. Called from `routes.rs`
/// ahead of the per-method dispatchers. Returns `Some(resp)` when
/// the path matches; `None` lets the caller fall through.
pub(super) async fn try_dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let tail = req.strip_prefix("GET ")?;
    let path = tail.split_whitespace().next().unwrap_or("");
    let path_only = path.split_once('?').map_or(path, |(p, _)| p);
    let bundle_id = path_only.strip_prefix("/api/v1/audit/export/")?;
    if bundle_id.is_empty() || bundle_id.contains('/') {
        return None;
    }
    Some(download_response(req, bundle_id, path, store).await)
}

/// Dispatcher for the POST mutate paths. Called from
/// `routes_dispatch_post_audit::dispatch_audit`.
pub(super) async fn try_dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if is_route(req, "POST", "/api/v1/audit/export/preview") {
        return Some(preview_response(req, store).await);
    }
    if is_route(req, "POST", "/api/v1/audit/export") {
        return Some(export_response(req, store).await);
    }
    None
}

fn is_route(req: &str, method: &str, path: &str) -> bool {
    let prefix = format!("{method} {path}");
    req.starts_with(&format!("{prefix} ")) || req.starts_with(&format!("{prefix}?"))
}

/// Bundle dir cleanup helper exposed for test isolation. Removes
/// every file under the export root; tests use it to keep
/// per-process state from leaking between cases.
#[cfg(test)]
pub(super) fn _test_cleanup_export_dir() {
    let _ = std::fs::remove_dir_all(signing::export_root());
}

/// Allow tests to walk the bundle directory without re-deriving the
/// path from env vars.
#[cfg(test)]
pub(super) fn _test_export_root() -> std::path::PathBuf {
    signing::export_root()
}

#[cfg(test)]
#[path = "routes_audit_export_tests.rs"]
mod tests;
