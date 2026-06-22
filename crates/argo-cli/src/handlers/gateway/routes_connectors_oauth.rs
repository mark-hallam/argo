//! Gateway OAuth connect + callback for the OAuth connectors (Gmail, Google
//! Calendar, HubSpot, Notion(OAuth), Salesforce). Closes item ④: a non-developer
//! can connect their inbox/CRM from the browser instead of the 409 the
//! credential-POST path returns for OAuth (`routes_connectors.rs`).
//!
//! Mirrors the SSO OIDC handler ([`super::sso_oidc`]) — STREAM-OWNING so it can
//! emit `Location:` + `Set-Cookie:`, which the `(status, body)` dispatch tuple
//! used elsewhere cannot — and REUSES the CLI OAuth helpers
//! ([`crate::handlers::connector_authorize_oauth`]) for the authorize-URL build
//! and the server-side code→token exchange. The CLI persists process-global;
//! the gateway persists TENANT-SCOPED (`cred::set_tenant`) under the durable
//! vault root the MCP-spawn read path uses.
//!
//! Flow:
//! - `GET /api/v1/connectors/<id>/connect`  → 302 to the provider consent page,
//!   with a short-lived signed cookie carrying {tenant, csrf-state, connector}.
//! - `GET /api/v1/connectors/<id>/callback?code&state` → verify the cookie +
//!   the session tenant, exchange the code for tokens, store them for the tenant,
//!   then 302 back to the SPA `/settings/connectors`.
//!
//! INERT-until-configured: the provider client id/secret come from the gateway's
//! unscoped credential store (the OAuth app the OPERATOR registers). Absent →
//! the connect route bounces back to the SPA with `oauth=not_configured` rather
//! than a broken redirect. (Gmail/GCal also need Google's app verification before
//! external users may grant the scope — that's a provider-side timeline.)

use super::resolve_tenant::resolve_tenant_for_request;
use super::sso_oidc_helpers::{query_param, write_simple};
use super::sso_session::{self, clear_cookie_header, read_cookie, set_cookie_header, SessionPayload};
use crate::handlers::connector_authorize_oauth::{build_authorize_url, exchange_code, new_state};
use crate::handlers::connector_authorize_paths::spec_for;
use crate::handlers::connector_authorize_specs::{AuthKind, ConnectorSpec, OAuthSpec};
use argo_store::SessionStore;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// Short-lived signed cookie carrying the in-flight OAuth state from
/// `/connect` to `/callback`. Scoped to the connectors path.
const STATE_COOKIE: &str = "argo_connector_oauth";
const STATE_TTL_SECONDS: u64 = 600;

/// True when boot_router should hand this off to [`serve`] (stream-owning).
/// Matches `GET /api/v1/connectors/<id>/connect|callback` only.
pub(super) fn is_connector_oauth_request(req: &str) -> bool {
    let line = req.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    if parts.next() != Some("GET") {
        return false;
    }
    let path = parts.next().unwrap_or("").split('?').next().unwrap_or("");
    path.starts_with("/api/v1/connectors/")
        && (path.ends_with("/connect") || path.ends_with("/callback"))
}

/// Stream-owning entrypoint (mirrors `sso_oidc::serve`).
pub(super) async fn serve(req: &str, store: Option<&Arc<SessionStore>>, out: &mut TcpStream) {
    let line = req.lines().next().unwrap_or("");
    let path_q = line.split_whitespace().nth(1).unwrap_or("");
    let path = path_q.split('?').next().unwrap_or("");
    let query = path_q.split_once('?').map(|(_, q)| q).unwrap_or("");
    let rest = path.strip_prefix("/api/v1/connectors/").unwrap_or("");
    let id = rest.split('/').next().unwrap_or("").to_string();
    let r = if path.ends_with("/connect") {
        handle_connect(&id, req, store, out).await
    } else {
        handle_callback(&id, req, query, store, out).await
    };
    let _ = r;
}

/// Resolve the OAuth spec + its app-client-id, or return a reason string the
/// caller bounces back to the SPA.
fn oauth_ready(id: &str) -> Result<(&'static ConnectorSpec, &'static OAuthSpec, String), &'static str> {
    let spec = spec_for(id).map_err(|_| "unknown_connector")?;
    if spec.auth != AuthKind::OAuth {
        return Err("not_oauth");
    }
    let oauth = spec.oauth.as_ref().ok_or("not_oauth")?;
    let client_id = argo_config::cred::get_unscoped(oauth.client_id_key).unwrap_or_default();
    if client_id.trim().is_empty() {
        return Err("not_configured");
    }
    Ok((spec, oauth, client_id))
}

async fn handle_connect(
    id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let Some(store) = store else {
        return spa_redirect(out, id, "store_unavailable").await;
    };
    let (_spec, oauth, client_id) = match oauth_ready(id) {
        Ok(v) => v,
        Err(reason) => return spa_redirect(out, id, reason).await,
    };
    // Tenant from the signed-in session (the SPA navigates the browser here).
    let tenant = match resolve_tenant_for_request(req, store).await {
        Ok(t) => t.to_string(),
        Err(_) => return spa_redirect(out, id, "sign_in_required").await,
    };
    let secret = match super::routes_channels::hmac_secret() {
        Ok(s) => s,
        Err(_) => return write_simple(out, "500 Internal Server Error", r#"{"error":"hmac"}"#).await,
    };
    let state = new_state();
    let redirect_uri = callback_uri(req, id);
    let scopes: Vec<String> = oauth.scopes.iter().map(|s| (*s).to_string()).collect();
    let auth_url = build_authorize_url(oauth.auth_url, &client_id, &redirect_uri, &state, &scopes);
    // Sign {connector, csrf-state, tenant} into a short-lived cookie. Reuses the
    // SSO session payload shape: sub=connector, email=csrf-state, tenant=tenant.
    let now = chrono::Utc::now().timestamp();
    let payload = SessionPayload {
        sub: id.to_string(),
        email: state.clone(),
        tenant: tenant.clone(),
        mechanism: "connector_oauth".into(),
        iat: now,
        exp: now + i64::try_from(STATE_TTL_SECONDS).unwrap_or(600),
    };
    let cookie = match sso_session::sign(&payload, &secret) {
        Ok(c) => c,
        Err(_) => return write_simple(out, "500 Internal Server Error", r#"{"error":"sign"}"#).await,
    };
    let set = set_cookie_header(STATE_COOKIE, &cookie, STATE_TTL_SECONDS, "/api/v1/connectors/");
    let resp = format!(
        "HTTP/1.1 302 Found\r\nLocation: {auth_url}\r\n{set}\r\nContent-Type: application/json\r\nContent-Length: 0\r\n\r\n"
    );
    out.write_all(resp.as_bytes()).await?;
    out.shutdown().await
}

async fn handle_callback(
    id: &str,
    req: &str,
    query: &str,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let Some(store) = store else {
        return spa_redirect(out, id, "store_unavailable").await;
    };
    let (spec, oauth, client_id) = match oauth_ready(id) {
        Ok(v) => v,
        Err(reason) => return spa_redirect(out, id, reason).await,
    };
    let code = match query_param(query, "code") {
        Some(c) if !c.is_empty() => c,
        _ => return spa_redirect(out, id, "missing_code").await,
    };
    let state = query_param(query, "state").unwrap_or_default();
    let cookie = match read_cookie(req, STATE_COOKIE) {
        Some(c) => c,
        None => return spa_redirect(out, id, "missing_state_cookie").await,
    };
    let secret = match super::routes_channels::hmac_secret() {
        Ok(s) => s,
        Err(_) => return spa_redirect(out, id, "hmac").await,
    };
    let now = chrono::Utc::now().timestamp();
    let signed = match sso_session::verify_with_secret(&cookie, now, &secret) {
        Ok(p) => p,
        Err(_) => return spa_redirect(out, id, "bad_state_cookie").await,
    };
    // CSRF + connector binding.
    if signed.email != state || signed.sub != id {
        return spa_redirect(out, id, "state_mismatch").await;
    }
    // Re-resolve the tenant from the live session and bind it to the cookie's
    // tenant — so a token is stored for the SAME tenant that began the flow,
    // and we get a real `TenantId` without re-parsing a string.
    let tenant = match resolve_tenant_for_request(req, store).await {
        Ok(t) => t,
        Err(_) => return spa_redirect(out, id, "sign_in_required").await,
    };
    if tenant.to_string() != signed.tenant {
        return spa_redirect(out, id, "tenant_mismatch").await;
    }
    let client_secret = argo_config::cred::get_unscoped(oauth.client_secret_key).unwrap_or_default();
    let redirect_uri = callback_uri(req, id);
    let token = match exchange_code(oauth.token_url, &client_id, &client_secret, &redirect_uri, &code).await {
        Ok(t) => t,
        Err(_) => return spa_redirect(out, id, "exchange_failed").await,
    };
    if persist_tenant(&tenant, spec, oauth, &token).is_err() {
        return spa_redirect(out, id, "store_failed").await;
    }
    audit_connected(store, &tenant.to_string(), id).await;
    spa_redirect(out, id, "connected").await
}

/// Persist the exchanged token TENANT-SCOPED under the durable vault root the
/// MCP-spawn read path injects from (`cred::workspace_root`). Mirrors the CLI's
/// `persist_oauth`, but `set_tenant` instead of process-global `set`.
fn persist_tenant(
    tenant: &argo_types::TenantId,
    spec: &ConnectorSpec,
    oauth: &OAuthSpec,
    token: &Value,
) -> argo_types::ArgoResult<()> {
    let workspace = argo_config::cred::workspace_root();
    let access = token
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| argo_types::ArgoError::Provider("no access_token".into()))?;
    argo_config::cred::set_tenant(&workspace, tenant, spec.token_key, access)?;
    if let (Some(key), Some(value)) = (oauth.refresh_key, token.get("refresh_token").and_then(Value::as_str)) {
        argo_config::cred::set_tenant(&workspace, tenant, key, value)?;
    }
    if let (Some(key), Some(value)) = (oauth.instance_url_key, token.get("instance_url").and_then(Value::as_str)) {
        argo_config::cred::set_tenant(&workspace, tenant, key, value.trim_end_matches('/'))?;
    }
    Ok(())
}

/// The gateway's own public callback URL — must match the redirect URI the
/// operator registers at the provider. Prefers `ARGO_OAUTH_REDIRECT_BASE`,
/// else derives `https://<Host>` from the request.
fn callback_uri(req: &str, id: &str) -> String {
    if let Ok(base) = std::env::var("ARGO_OAUTH_REDIRECT_BASE") {
        let base = base.trim().trim_end_matches('/');
        if !base.is_empty() {
            return format!("{base}/api/v1/connectors/{id}/callback");
        }
    }
    let host = req
        .lines()
        .find_map(|l| {
            let l = l.trim_end();
            l.strip_prefix("Host: ").or_else(|| l.strip_prefix("host: "))
        })
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .unwrap_or("api.skycot.com");
    format!("https://{host}/api/v1/connectors/{id}/callback")
}

/// 302 the browser back to the SPA connectors page with the outcome, clearing
/// the in-flight state cookie. The SPA origin is `SKYCOT_PUBLIC_BASE_URL`
/// (the gateway lives on a different host than the SPA).
async fn spa_redirect(out: &mut TcpStream, id: &str, outcome: &str) -> tokio::io::Result<()> {
    let base = std::env::var("SKYCOT_PUBLIC_BASE_URL")
        .ok()
        .map(|b| b.trim().trim_end_matches('/').to_string())
        .filter(|b| !b.is_empty())
        .unwrap_or_else(|| "https://app.skycot.com".to_string());
    let loc = format!("{base}/settings/connectors?connector={id}&oauth={outcome}");
    let clear = clear_cookie_header(STATE_COOKIE, "/api/v1/connectors/");
    let resp = format!(
        "HTTP/1.1 302 Found\r\nLocation: {loc}\r\n{clear}\r\nContent-Type: application/json\r\nContent-Length: 0\r\n\r\n"
    );
    out.write_all(resp.as_bytes()).await?;
    out.shutdown().await
}

async fn audit_connected(store: &Arc<SessionStore>, tenant: &str, connector: &str) {
    let sink = argo_audit::AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    let payload = serde_json::json!({
        "event_type": "connector.oauth_connected",
        "connector": connector,
        "tenant": tenant,
    });
    let _ = sink
        .write(None, argo_audit::AuditKind::HttpRequest, payload, None)
        .await;
}

#[cfg(test)]
#[path = "routes_connectors_oauth_tests.rs"]
mod tests;
