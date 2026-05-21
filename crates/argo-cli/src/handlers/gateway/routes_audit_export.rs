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
//! - `GET /api/v1/audit/export/<bundle_id>?expires=…&sig=…` —
//!   download a previously-built bundle. Verifies the HMAC over
//!   `bundle_id|expires`, refuses expired or unsigned URLs.
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
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// Lifetime of a freshly-signed export URL, in seconds (5 minutes).
pub(super) const EXPORT_URL_TTL_SECS: i64 = 300;

const SECRET_ENV: &str = "ARGO_AUDIT_EXPORT_SECRET";
const DEV_FALLBACK_SECRET: &[u8] = b"argo-dev-export-secret-do-not-use-in-prod";
const EXPORT_DIR_ENV: &str = "ARGO_AUDIT_EXPORT_DIR";

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
    let rows = match fetch_rows_for_export(store, &window).await {
        Ok(r) => r,
        Err(e) => return server_error(&format!("fetch rows: {e}")),
    };
    let bytes = match build_bundle(&rows, format) {
        Ok(b) => b,
        Err(e) => return server_error(&format!("build bundle: {e}")),
    };
    let sha256 = sha256_hex(&bytes);
    let bundle_id = Uuid::now_v7().simple().to_string();
    let export_dir = export_root();
    if let Err(e) = std::fs::create_dir_all(&export_dir) {
        return server_error(&format!(
            "create export dir {}: {e}",
            export_dir.display()
        ));
    }
    let filename = format!("{bundle_id}.{}", format.extension());
    let bundle_path = export_dir.join(&filename);
    if let Err(e) = std::fs::write(&bundle_path, &bytes) {
        return server_error(&format!("write {}: {e}", bundle_path.display()));
    }
    let expires = Utc::now().timestamp() + EXPORT_URL_TTL_SECS;
    let download_url = sign_export_url(&bundle_id, format, expires);
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
    let preview: ExportPreview = match fetch_preview(store, &window).await {
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

/// `GET /api/v1/audit/export/<bundle_id>?format=<f>&expires=<unix>&sig=<hex>`.
/// Returns the bundle bytes (base64-wrapped — the v1 listener
/// hard-codes JSON content-type; mirrors the screenshot endpoint).
pub(super) async fn download_response(
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
    let Some(sig) = query_kv(query, "sig") else {
        return unauthorized("missing sig");
    };
    if expires <= Utc::now().timestamp() {
        return unauthorized("signature expired");
    }
    if !verify_signature(bundle_id, format, expires, &sig) {
        return unauthorized("signature mismatch");
    }
    if !is_safe_bundle_id(bundle_id) {
        return bad_request("invalid bundle id");
    }
    let path = export_root().join(format!("{bundle_id}.{}", format.extension()));
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

/// Build the public download URL for `(bundle_id, format)`. The
/// caller picks `expires` (typically `now + EXPORT_URL_TTL_SECS`).
#[must_use]
pub(super) fn sign_export_url(bundle_id: &str, format: BundleFormat, expires: i64) -> String {
    let sig = compute_signature(bundle_id, format, expires);
    format!(
        "/api/v1/audit/export/{bundle_id}?format={}&expires={expires}&sig={sig}",
        format_wire(format)
    )
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

fn export_root() -> PathBuf {
    if let Ok(dir) = std::env::var(EXPORT_DIR_ENV) {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    std::env::temp_dir().join("argo-audit-exports")
}

fn is_safe_bundle_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn format_wire(f: BundleFormat) -> &'static str {
    match f {
        BundleFormat::Json => "json",
        BundleFormat::Csv => "csv",
        BundleFormat::Tar => "tar",
    }
}

fn verify_signature(bundle_id: &str, format: BundleFormat, expires: i64, presented: &str) -> bool {
    let expected = compute_signature(bundle_id, format, expires);
    constant_time_eq(expected.as_bytes(), presented.as_bytes())
}

fn compute_signature(bundle_id: &str, format: BundleFormat, expires: i64) -> String {
    let secret = secret_bytes();
    let msg = format!("{bundle_id}|{}|{expires}", format_wire(format));
    hex::encode(hmac_sha256(&secret, msg.as_bytes()))
}

fn secret_bytes() -> Vec<u8> {
    std::env::var(SECRET_ENV)
        .ok()
        .filter(|s| !s.is_empty())
        .map_or_else(|| DEV_FALLBACK_SECRET.to_vec(), String::into_bytes)
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut k = if key.len() > BLOCK {
        let mut h = Sha256::new();
        h.update(key);
        h.finalize().to_vec()
    } else {
        key.to_vec()
    };
    k.resize(BLOCK, 0);
    let opad: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();
    let ipad: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(msg);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(inner_digest);
    let out = outer.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn query_kv(query: &str, key: &str) -> Option<String> {
    let trimmed = query.split_whitespace().next().unwrap_or(query);
    let qs = trimmed
        .split_once('?')
        .map(|(_, q)| q)
        .unwrap_or(trimmed);
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
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
    Some(download_response(bundle_id, path, store).await)
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
/// every file under [`export_root`]; tests use it to keep
/// per-process state from leaking between cases.
#[cfg(test)]
pub(super) fn _test_cleanup_export_dir() {
    let _ = std::fs::remove_dir_all(export_root());
}

/// Allow tests to walk the bundle directory without re-deriving the
/// path from env vars.
#[cfg(test)]
pub(super) fn _test_export_root() -> PathBuf {
    export_root()
}

#[cfg(test)]
#[path = "routes_audit_export_tests.rs"]
mod tests;
