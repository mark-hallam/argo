//! Customer-facing GDPR/CCPA Article 15 data export
//! (Stream `mission-data-export-customer-flow`).
//!
//! Three routes hang off `/api/v1/me/export`:
//!
//! 1. `POST /api/v1/me/export` — accept the request, enforce the
//!    1-per-24h rate limit, insert a `data_export_job` row, spawn the
//!    background tar assembly, emit `AuditKind::DataExportRequested`,
//!    return `{ "job_id": ... }`. Synchronous for the v1 in-memory
//!    tarball — multi-GB exports are out of scope for v1 (the design
//!    keeps a background-task seam via `tokio::spawn` so a later
//!    redis-backed pattern can swap in without an API change).
//! 2. `GET /api/v1/me/export/<job_id>` — poll the job status. Returns
//!    `queued`/`running`/`ready`/`failed`/`expired` + the
//!    `download_url` (token-suffixed) when `ready`.
//! 3. `GET /api/v1/me/export/download/<token>` — resolve a `Ready`
//!    bundle by its hashed token, stream the raw `tar.gz` bytes. 404
//!    when expired or unknown.
//!
//! Authentication: the magic-link verify route stamps every
//! authenticated request with `X-Argo-Principal: human:<email>` (the
//! same contract `onboarding_provision` relies on). Without that
//! header the user is anonymous and every route here 401s.

use argo_audit::{AuditKind, AuditSink, SYSTEM_AUDIT_TENANT_ID};
use argo_identity::data_export::{gather_for_user, ExportBundle};
use argo_identity::user::UserId;
use argo_identity::user_store::get_user_by_email;
use argo_identity::user_tenant_store::list_tenants_for_user;
use argo_store::data_export_job_query::{
    self, ExportJobRow, ExportJobStatus, JobInsert, DOWNLOAD_TTL_SECS, RATE_LIMIT_WINDOW_SECS,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use base64::Engine;
use chrono::Utc;
use flate2::write::GzEncoder;
use flate2::Compression;
use rand_core::{OsRng, RngCore};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::sync::Arc;
use uuid::Uuid;

const PRINCIPAL_HEADER: &str = "X-Argo-Principal";
const HUMAN_PRINCIPAL_PREFIX: &str = "human:";

/// Try to dispatch `req` to one of the three `/me/export` routes.
/// Returns `None` when the path doesn't match — the caller falls
/// through to the next handler.
pub(super) async fn dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("POST /api/v1/me/export ") || req.starts_with("POST /api/v1/me/export?") {
        return Some(request_export_response(req, store).await);
    }
    if let Some(rest) = req.strip_prefix("GET /api/v1/me/export/download/") {
        let token = rest.split_whitespace().next().unwrap_or("");
        let token = token.split('?').next().unwrap_or(token);
        return Some(download_response(token, store).await);
    }
    if let Some(rest) = req.strip_prefix("GET /api/v1/me/export/") {
        let job_id = rest.split_whitespace().next().unwrap_or("");
        let job_id = job_id.split('?').next().unwrap_or(job_id);
        return Some(status_response(req, job_id, store).await);
    }
    None
}

async fn request_export_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(deny) = super::role_gate::enforce_role(req, super::role_gate::Action::DataExport)
    {
        return deny;
    }
    let store = match store {
        Some(s) => s,
        None => return store_unavailable(),
    };
    let email = match principal_email(req) {
        Some(e) => e,
        None => return unauthorized(),
    };
    let user = match get_user_by_email(store, &email).await {
        Ok(Some(u)) => u,
        Ok(None) => return unauthorized(),
        Err(e) => return server_error(&format!("resolve user: {e}")),
    };
    let now = Utc::now();
    let now_secs = now.timestamp();
    let window_start = now_secs - RATE_LIMIT_WINDOW_SECS;
    let recent =
        match data_export_job_query::count_jobs_since(store.pool(), user.id.0, window_start).await
        {
            Ok(n) => n,
            Err(e) => return server_error(&format!("rate-limit check: {e}")),
        };
    if recent >= 1 {
        return (
            "429 Too Many Requests",
            json!({
                "error": {
                    "code": "rate_limited",
                    "message": "one data export per 24 hours",
                    "retry_after_secs": RATE_LIMIT_WINDOW_SECS,
                }
            })
            .to_string(),
        );
    }

    let job_id = Uuid::now_v7().to_string();
    let primary_tenant: Option<TenantId> = list_tenants_for_user(store, user.id)
        .await
        .ok()
        .and_then(|mut t| t.pop())
        .map(|t| t.tenant_id);
    if let Err(e) = data_export_job_query::insert_job(
        store.pool(),
        &JobInsert {
            job_id: &job_id,
            user_id: user.id.0,
            tenant_id: primary_tenant.map(|t| t.0),
            requested_at: now_secs,
        },
    )
    .await
    {
        return server_error(&format!("insert job: {e}"));
    }

    emit_audit(store, &user.email, user.id, primary_tenant, &job_id, now_secs).await;

    // v1: assemble inline. The store-backed job lets future
    // implementations move this to a background task; the response
    // already shapes around the polling contract so the wire surface
    // doesn't change when assembly goes async.
    let bundle = match gather_for_user(store, user.id, now).await {
        Ok(b) => b,
        Err(e) => {
            let _ = data_export_job_query::mark_failed(
                store.pool(),
                &job_id,
                &format!("gather: {e}"),
                Utc::now().timestamp(),
            )
            .await;
            return server_error(&format!("gather: {e}"));
        }
    };
    let bytes = match tar_gz_bundle(&bundle) {
        Ok(b) => b,
        Err(msg) => {
            let _ = data_export_job_query::mark_failed(
                store.pool(),
                &job_id,
                &msg,
                Utc::now().timestamp(),
            )
            .await;
            return server_error(&msg);
        }
    };
    let raw_token = mint_token();
    let token_hash = sha256_hex(&raw_token);
    let completed_at = Utc::now().timestamp();
    if let Err(e) = data_export_job_query::mark_ready(
        store.pool(),
        &job_id,
        &bytes,
        &token_hash,
        completed_at,
        completed_at + DOWNLOAD_TTL_SECS,
    )
    .await
    {
        return server_error(&format!("mark ready: {e}"));
    }
    (
        "202 Accepted",
        json!({
            "job_id": job_id,
            "status": ExportJobStatus::Ready.as_str(),
            "download_url": format!("/api/v1/me/export/download/{raw_token}"),
            "size_bytes": bytes.len(),
            "expires_at": completed_at + DOWNLOAD_TTL_SECS,
        })
        .to_string(),
    )
}

async fn status_response(
    req: &str,
    job_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return store_unavailable(),
    };
    let email = match principal_email(req) {
        Some(e) => e,
        None => return unauthorized(),
    };
    let user = match get_user_by_email(store, &email).await {
        Ok(Some(u)) => u,
        Ok(None) => return unauthorized(),
        Err(e) => return server_error(&format!("resolve user: {e}")),
    };
    let row = match data_export_job_query::get_job_for_user(store.pool(), job_id, user.id.0).await
    {
        Ok(Some(r)) => r,
        Ok(None) => return not_found(),
        Err(e) => return server_error(&format!("read job: {e}")),
    };
    ("200 OK", render_status(&row))
}

async fn download_response(
    raw_token: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return store_unavailable(),
    };
    if raw_token.is_empty() {
        return not_found();
    }
    let token_hash = sha256_hex(raw_token);
    let now = Utc::now().timestamp();
    let pair = match data_export_job_query::fetch_ready_bundle(store.pool(), &token_hash, now).await
    {
        Ok(Some(p)) => p,
        Ok(None) => return not_found(),
        Err(e) => return server_error(&format!("download: {e}")),
    };
    let (_job, bytes) = pair;
    // We can't carry `Content-Type: application/gzip` through the
    // dispatcher's fixed JSON response tuple — surface the raw bytes
    // as a base64 envelope so the SPA can `atob` + reconstruct a
    // browser download. The brief budget covers the JSON form; once
    // the dispatcher returns a streamable tuple (`mission-gateway-
    // dispatcher-headers` follow-up) this switches to an octet-stream.
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    (
        "200 OK",
        json!({
            "content_type": "application/gzip",
            "filename": format!("my-skycot-export-{}.tar.gz", chrono::Utc::now().format("%Y-%m-%d")),
            "size_bytes": bytes.len(),
            "body_b64": encoded,
        })
        .to_string(),
    )
}

fn render_status(row: &ExportJobRow) -> String {
    let mut payload = json!({
        "job_id": row.job_id,
        "status": row.status.as_str(),
        "requested_at": row.requested_at,
        "completed_at": row.completed_at,
        "expires_at": row.expires_at,
    });
    if matches!(row.status, ExportJobStatus::Failed) {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "error".to_string(),
                Value::String(row.error_message.clone().unwrap_or_default()),
            );
        }
    }
    payload.to_string()
}

fn tar_gz_bundle(bundle: &ExportBundle) -> Result<Vec<u8>, String> {
    let mut gz = GzEncoder::new(Vec::<u8>::new(), Compression::default());
    {
        let mut builder = tar::Builder::new(&mut gz);
        for file in &bundle.files {
            let mut header = tar::Header::new_gnu();
            header
                .set_path(&file.path)
                .map_err(|e| format!("tar set_path {}: {e}", file.path))?;
            header.set_size(file.contents.len() as u64);
            header.set_mode(0o600);
            header.set_cksum();
            builder
                .append(&header, file.contents.as_bytes())
                .map_err(|e| format!("tar append {}: {e}", file.path))?;
        }
        builder
            .finish()
            .map_err(|e| format!("tar finish: {e}"))?;
    }
    let mut bytes = gz.finish().map_err(|e| format!("gz finish: {e}"))?;
    bytes.flush().map_err(|e| format!("gz flush: {e}"))?;
    Ok(bytes)
}

async fn emit_audit(
    store: &Arc<SessionStore>,
    email: &str,
    user_id: UserId,
    tenant_id: Option<TenantId>,
    job_id: &str,
    requested_at: i64,
) {
    let tenant_label = tenant_id
        .map(|t| t.to_string())
        .unwrap_or_else(|| SYSTEM_AUDIT_TENANT_ID.to_string());
    let payload = json!({
        "user_id": user_id.0.to_string(),
        "tenant_id": tenant_id.map(|t| t.to_string()),
        "job_id": job_id,
        "requested_at": requested_at,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_label);
    let principal = format!("{HUMAN_PRINCIPAL_PREFIX}{email}");
    if let Err(e) = sink
        .write(
            None,
            AuditKind::DataExportRequested,
            payload,
            Some(principal),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::me_export",
            error = %e,
            user_id = %user_id,
            "data_export.requested audit write failed"
        );
    }
}

fn principal_email(req: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers
        .lines()
        .skip(1)
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(PRINCIPAL_HEADER)
                .then(|| value.trim().to_string())
        })
        .and_then(|raw| raw.strip_prefix(HUMAN_PRINCIPAL_PREFIX).map(str::to_string))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.contains('@'))
}

fn mint_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn sha256_hex(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}

fn store_unavailable() -> (&'static str, String) {
    (
        "503 Service Unavailable",
        r#"{"error":"session store unavailable"}"#.to_string(),
    )
}

fn unauthorized() -> (&'static str, String) {
    (
        "401 Unauthorized",
        json!({
            "error": {
                "code": "unauthorized",
                "message": "magic-link sign-in required",
            }
        })
        .to_string(),
    )
}

fn not_found() -> (&'static str, String) {
    (
        "404 Not Found",
        r#"{"error":"not found"}"#.to_string(),
    )
}

fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({
            "error": {
                "code": "internal_error",
                "message": message,
            }
        })
        .to_string(),
    )
}

#[cfg(test)]
#[path = "routes_me_export_tests.rs"]
mod tests;
