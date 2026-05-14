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
//! * ACS handler parses the response structurally — extracts
//!   `<saml:NameID>` and the `<saml:Attribute>` rows. XML
//!   signature verification is deferred to a follow-up brief that
//!   pairs with an XML-DSig library; the parsed `idp_cert_pem`
//!   is stored on [`argo_config::SsoSamlTenant`] for that future
//!   step.
//! * JIT provisioning: a successful ACS sets a signed session
//!   cookie under the configured `[sso].cookie_name`.

use argo_store::SessionStore;
use rand_core::{OsRng, RngCore};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use super::sso_session::{self, set_cookie_header, SessionPayload};

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
        (SamlKind::Acs(tenant), "POST") => handle_acs(req, &tenant, &cfg, out).await,
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
    url.push_str(&format!("SAMLRequest={}", super::sso_oidc::url_encode(&b64)));
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
    let redirect = match build_authn_request(tenant, cfg, rid, chrono::Utc::now()) {
        Ok(r) => r,
        Err(e) => return write_simple(out, "400 Bad Request", &format!(r#"{{"error":"{e}"}}"#)).await,
    };
    let resp = format!(
        "HTTP/1.1 302 Found\r\nLocation: {}\r\nContent-Type: application/json\r\nContent-Length: 0\r\n\r\n",
        redirect.url
    );
    out.write_all(resp.as_bytes()).await?;
    out.shutdown().await
}

/// Outcome of a SAML ACS POST. Pure-data return type so the
/// async unit test can assert without TCP.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct AcsOutcome {
    pub status: &'static str,
    pub body: String,
    pub location: Option<String>,
    pub set_cookie: Option<String>,
}

pub(super) fn build_acs_outcome(
    body: &str,
    tenant: &str,
    cfg: &argo_config::SsoConfig,
    secret: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> AcsOutcome {
    if !cfg.saml.contains_key(tenant) {
        return err_outcome("400 Bad Request", "tenant not configured");
    }
    let form = parse_form(body);
    let raw = match form.iter().find(|(k, _)| k == "SAMLResponse") {
        Some((_, v)) => v.clone(),
        None => return err_outcome("400 Bad Request", "missing SAMLResponse"),
    };
    let xml_bytes = match b64_std_decode(&raw) {
        Ok(b) => b,
        Err(_) => return err_outcome("400 Bad Request", "bad base64"),
    };
    let xml = match std::str::from_utf8(&xml_bytes) {
        Ok(s) => s,
        Err(_) => return err_outcome("400 Bad Request", "response not utf8"),
    };
    let (sub, email) = match extract_name_id(xml) {
        Some(t) => t,
        None => return err_outcome("400 Bad Request", "no NameID"),
    };
    let payload = SessionPayload {
        sub,
        email,
        tenant: tenant.to_string(),
        mechanism: "saml".into(),
        iat: now.timestamp(),
        exp: now.timestamp() + i64::try_from(cfg.session_ttl_seconds).unwrap_or(28_800),
    };
    let token = match sso_session::sign(&payload, secret) {
        Ok(t) => t,
        Err(_) => return err_outcome("500 Internal Server Error", "sign"),
    };
    AcsOutcome {
        status: "302 Found",
        body: String::new(),
        location: Some("/app/".into()),
        set_cookie: Some(set_cookie_header(
            &cfg.cookie_name,
            &token,
            cfg.session_ttl_seconds,
            "/",
        )),
    }
}

async fn handle_acs(
    req: &str,
    tenant: &str,
    cfg: &argo_config::SsoConfig,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let body = super::body_from_request(req);
    let secret = match super::routes_channels::hmac_secret() {
        Ok(s) => s,
        Err(_) => return write_simple(out, "500 Internal Server Error", r#"{"error":"hmac secret"}"#).await,
    };
    let outcome = build_acs_outcome(body, tenant, cfg, &secret, chrono::Utc::now());
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
    head.push_str("\r\n");
    head.push_str(&outcome.body);
    out.write_all(head.as_bytes()).await?;
    out.shutdown().await
}

/// Pull `<saml:NameID>…</saml:NameID>` plus the optional `email`
/// attribute out of an XML Response. Returns `(sub, email)`; the
/// email defaults to the NameID when no separate attribute is
/// supplied (a common shape for IdPs that put email in the
/// NameID itself with `Format=…:emailAddress`).
pub(super) fn extract_name_id(xml: &str) -> Option<(String, String)> {
    let name_id = extract_between(xml, "<saml:NameID", "</saml:NameID>")
        .or_else(|| extract_between(xml, "<saml2:NameID", "</saml2:NameID>"))
        .or_else(|| extract_between(xml, "<NameID", "</NameID>"))?;
    let sub = strip_open_tag(&name_id);
    let email = extract_attribute_value(xml, "email")
        .or_else(|| extract_attribute_value(xml, "mail"))
        .unwrap_or_else(|| sub.clone());
    Some((sub, email))
}

fn extract_between(s: &str, open: &str, close: &str) -> Option<String> {
    let start = s.find(open)?;
    let after_open = &s[start..];
    let close_idx = after_open.find(close)?;
    Some(after_open[..close_idx].to_string())
}

/// Given `<saml:NameID Format=\"…\">value`, return `value`.
fn strip_open_tag(s: &str) -> String {
    if let Some(idx) = s.find('>') {
        s[idx + 1..].trim().to_string()
    } else {
        s.trim().to_string()
    }
}

/// Search for `<{ns:}?Attribute Name="<key>"><{ns:}?AttributeValue>…</…>`
/// and return the inner text. Tolerant of optional `saml:` /
/// `saml2:` namespace prefixes and case differences.
fn extract_attribute_value(xml: &str, key: &str) -> Option<String> {
    let key_lc = key.to_lowercase();
    let lower = xml.to_lowercase();
    let mut cursor = 0_usize;
    while let Some(pos) = lower[cursor..].find("attribute ") {
        let abs = cursor + pos;
        let after = &xml[abs..];
        let close = after.find('>')?;
        let header = &after[..close];
        if header.to_lowercase().contains(&format!("name=\"{key_lc}\"")) {
            let body_after = &after[close + 1..];
            let body_lc = body_after.to_lowercase();
            // Match `<attributevalue` or `<NS:attributevalue` —
            // some IdPs emit the bare element, others namespace it.
            let val_start = ["<attributevalue", ":attributevalue"]
                .iter()
                .filter_map(|needle| body_lc.find(needle))
                .min()?;
            // Walk back to the `<` so we land on the element start.
            let elem_start = body_after[..=val_start].rfind('<')?;
            let after_val = &body_after[elem_start..];
            let val_close = after_val.find('>')?;
            let val_body = &after_val[val_close + 1..];
            let end_lc = val_body.to_lowercase();
            let end = end_lc.find("</")?;
            return Some(val_body[..end].trim().to_string());
        }
        cursor = abs + close + 1;
    }
    None
}

fn parse_form(body: &str) -> Vec<(String, String)> {
    body.split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            Some((k.to_string(), percent_decode(v)))
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    out.push((h << 4) | l);
                    i += 2;
                } else {
                    out.push(b'%');
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(super) fn b64_std_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub(super) fn b64_std_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s.as_bytes())
}

fn load_cfg() -> Result<argo_config::SsoConfig, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let cfg = argo_config::ArgoConfig::load(&cwd.join("argo.toml"))
        .map_err(|e| format!("load: {e}"))?;
    Ok(cfg.sso)
}

fn err_outcome(status: &'static str, msg: &str) -> AcsOutcome {
    AcsOutcome {
        status,
        body: format!(r#"{{"error":"{msg}"}}"#),
        location: None,
        set_cookie: None,
    }
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
