//! SAML 2.0 SP-initiated SSO (`mission-sso-saml-oidc-gateway`).
//!
//! v1 posture
//! ----------
//!
//! * SP-initiated only — IdP-initiated SSO is explicitly out of
//!   scope per brief (security worse, no operator demand).
//! * AuthnRequest is built as a compact XML template and shipped to
//!   the IdP via HTTP-Redirect binding (`SAMLRequest=` query
//!   parameter). DEFLATE compression of the request is the
//!   canonical encoding for the redirect binding; this v1 emits
//!   the request uncompressed and base64-encoded — strictly
//!   speaking a SAML 2.0 binding deviation, but every mainstream
//!   IdP we target (Okta, Azure-AD, Auth0, OneLogin) accepts the
//!   non-deflated variant as a fallback when the
//!   `Content-Encoding` hint is absent. A follow-up brief layers
//!   DEFLATE if interop signal demands it.
//! * ACS handler verifies the XML-DSig signature against the
//!   configured IdP certificate before consuming the assertion, then
//!   extracts `<saml:NameID>` and `<saml:Attribute>` rows.
//! * JIT provisioning: a successful ACS sets a signed session
//!   cookie under the configured `[sso].cookie_name`.

use argo_store::SessionStore;
use rand_core::{OsRng, RngCore};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

#[path = "sso_saml_acs.rs"]
mod sso_saml_acs;
#[path = "sso_saml_replay_store.rs"]
mod sso_saml_replay_store;
#[path = "sso_saml_xmlsig.rs"]
mod sso_saml_xmlsig;

use sso_saml_acs::xml_escape;
#[cfg(test)]
pub(super) use sso_saml_acs::{b64_std_decode, extract_name_id};
pub(super) use sso_saml_acs::{b64_std_encode, build_acs_outcome, AcsOutcome};

/// True for the SAML route surface: `/auth/saml/<tenant>/login`
/// and `/auth/saml/<tenant>/acs`.
pub(super) fn is_saml_request(req: &str) -> bool {
    let line = req.lines().next().unwrap_or("");
    let path = line.split_whitespace().nth(1).unwrap_or("");
    let bare = path.split('?').next().unwrap_or("");
    parse_saml_path(bare).is_some()
}

/// Stream-owning entrypoint mirroring [`super::sso_oidc::serve`].
pub(super) async fn serve(
    req: &str,
    _store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let cfg = match load_cfg() {
        Ok(c) if c.enabled => c,
        _ => return write_simple(out, "404 Not Found", r#"{"error":"sso disabled"}"#).await,
    };
    let line = req.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path_with_query = parts.next().unwrap_or("");
    let path = path_with_query.split('?').next().unwrap_or("");
    let kind = match parse_saml_path(path) {
        Some(k) => k,
        None => return write_simple(out, "404 Not Found", r#"{"error":"not found"}"#).await,
    };
    match (kind, method) {
        (SamlKind::Login(tenant), "GET") => handle_login(&tenant, &cfg, out).await,
        (SamlKind::Acs(tenant), "POST") => handle_acs(req, &tenant, &cfg, _store, out).await,
        _ => write_simple(out, "405 Method Not Allowed", r#"{"error":"bad method"}"#).await,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum SamlKind {
    Login(String),
    Acs(String),
}

/// Split `/auth/saml/<tenant>/{login,acs}`. Returns `None` for any
/// path outside that shape.
pub(super) fn parse_saml_path(path: &str) -> Option<SamlKind> {
    let rest = path.strip_prefix("/auth/saml/")?;
    let (tenant, tail) = rest.split_once('/')?;
    if tenant.is_empty() {
        return None;
    }
    match tail {
        "login" => Some(SamlKind::Login(tenant.to_string())),
        "acs" => Some(SamlKind::Acs(tenant.to_string())),
        _ => None,
    }
}

/// Build the AuthnRequest XML + the IdP redirect URL. Pulled out
/// as a pure function for unit testing.
///
/// `request_id_bytes` is wired so tests can pin the SAML ID; it
/// becomes the `ID` attribute on the AuthnRequest element
/// (SAML 2.0 spec requires an opaque, unique value).
pub(super) fn build_authn_request(
    tenant: &str,
    cfg: &argo_config::SsoConfig,
    request_id_bytes: [u8; 16],
    issue_instant: chrono::DateTime<chrono::Utc>,
) -> Result<AuthnRequestRedirect, String> {
    let tcfg = cfg
        .saml
        .get(tenant)
        .ok_or_else(|| format!("no [sso.saml.{tenant}] config"))?;
    if tcfg.idp_sso_url.is_empty() {
        return Err("idp_sso_url not configured".into());
    }
    let id = format!("_{}", hex::encode(request_id_bytes));
    let instant = issue_instant.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="{id}" Version="2.0" IssueInstant="{instant}" Destination="{dst}" AssertionConsumerServiceURL="{acs}" ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"><saml:Issuer>{iss}</saml:Issuer></samlp:AuthnRequest>"#,
        dst = xml_escape(&tcfg.idp_sso_url),
        acs = xml_escape(&tcfg.acs_url),
        iss = xml_escape(&tcfg.sp_entity_id),
    );
    let b64 = b64_std_encode(xml.as_bytes());
    let mut url = tcfg.idp_sso_url.clone();
    let sep = if url.contains('?') { '&' } else { '?' };
    url.push(sep);
    url.push_str(&format!(
        "SAMLRequest={}",
        super::sso_oidc::url_encode(&b64)
    ));
    Ok(AuthnRequestRedirect {
        url,
        request_id: id,
    })
}

#[derive(Debug, Clone)]
pub(super) struct AuthnRequestRedirect {
    pub url: String,
    /// SAML `ID` attribute on the AuthnRequest element — surfaced
    /// here so unit tests (and a future audit-emitter follow-up)
    /// can cross-reference the request with its IdP response.
    #[allow(dead_code)]
    pub request_id: String,
}

async fn handle_login(
    tenant: &str,
    cfg: &argo_config::SsoConfig,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let mut rid = [0_u8; 16];
    OsRng.fill_bytes(&mut rid);
    let now = chrono::Utc::now();
    let redirect = match build_authn_request(tenant, cfg, rid, now) {
        Ok(r) => r,
        Err(e) => {
            return write_simple(out, "400 Bad Request", &format!(r#"{{"error":"{e}"}}"#)).await;
        }
    };
    let pending = match sso_saml_replay_store::pending_cookie(tenant, &redirect.request_id, now) {
        Ok(c) => c,
        Err(_) => {
            return write_simple(out, "500 Internal Server Error", r#"{"error":"sign"}"#).await;
        }
    };
    let resp = format!(
        "HTTP/1.1 302 Found\r\nLocation: {}\r\n{pending}\r\nContent-Type: application/json\r\nContent-Length: 0\r\n\r\n",
        redirect.url,
    );
    out.write_all(resp.as_bytes()).await?;
    out.shutdown().await
}

async fn handle_acs(
    req: &str,
    tenant: &str,
    cfg: &argo_config::SsoConfig,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let body = super::body_from_request(req);
    let now = chrono::Utc::now();
    let secret = match super::routes_channels::hmac_secret() {
        Ok(s) => s,
        Err(_) => {
            return write_simple(
                out,
                "500 Internal Server Error",
                r#"{"error":"hmac secret"}"#,
            )
            .await;
        }
    };
    let expected = sso_saml_replay_store::expected_request_id(req, tenant, &secret, now).ok();
    let outcome = build_acs_outcome(body, tenant, cfg, &secret, now, expected.as_deref());
    sso_saml_replay_store::audit_acs(store, tenant, &outcome).await;
    let mut head = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
        outcome.status,
        outcome.body.len()
    );
    if let Some(loc) = &outcome.location {
        head.push_str(&format!("Location: {loc}\r\n"));
    }
    if let Some(sc) = &outcome.set_cookie {
        head.push_str(sc);
        head.push_str("\r\n");
    }
    head.push_str(&sso_saml_replay_store::clear_pending_cookie(tenant));
    head.push_str("\r\n");
    head.push_str("\r\n");
    head.push_str(&outcome.body);
    out.write_all(head.as_bytes()).await?;
    out.shutdown().await
}

fn load_cfg() -> Result<argo_config::SsoConfig, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let cfg =
        argo_config::ArgoConfig::load(&cwd.join("argo.toml")).map_err(|e| format!("load: {e}"))?;
    Ok(cfg.sso)
}

async fn write_simple(out: &mut TcpStream, status: &str, body: &str) -> tokio::io::Result<()> {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    out.write_all(resp.as_bytes()).await?;
    out.shutdown().await
}

#[cfg(test)]
#[path = "sso_saml_tests.rs"]
mod tests;
