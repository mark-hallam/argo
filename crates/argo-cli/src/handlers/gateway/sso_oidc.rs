//! OIDC authorization-code flow with PKCE (`mission-sso-saml-oidc-gateway`).
//!
//! v1 posture
//! ----------
//!
//! * Code-flow + PKCE (RFC 7636 S256) — the modern non-implicit
//!   profile recommended for new deployments.
//! * Token exchange happens server-side over TLS direct to the IdP
//!   `token_endpoint` carrying the client secret. Because the ID
//!   token reaches us straight from that endpoint (never through
//!   the user-agent), we trust its claims without independently
//!   verifying the JWT signature in v1. Operators wanting deep
//!   verification can layer a follow-up brief that consults the
//!   IdP JWKS — the same flow can be hardened without changing the
//!   request shape exposed to the IdP.
//! * Per-tenant IdP registration via `[sso.oidc.<tenant>]` —
//!   `tenant` is read off the `?tenant=` query parameter on
//!   `/auth/login` and persisted into the PKCE cookie so the
//!   callback resolves the right IdP without re-reading the URL.
//! * Session cookie is HMAC-SHA256 signed via
//!   [`super::sso_session`].
use argo_store::SessionStore;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use super::sso_session::{
    self, clear_cookie_header, read_cookie, set_cookie_header, SessionPayload,
};

/// Default scopes when the operator did not configure any.
pub(super) const DEFAULT_SCOPES: &str = "openid email profile";
/// Cookie name carrying the short-lived PKCE+state record from
/// `/auth/login` to `/auth/callback`.
pub(super) const PKCE_COOKIE: &str = "argo_sso_pkce";
/// PKCE cookie TTL — long enough for slow IdP login, short enough
/// to limit replay surface.
pub(super) const PKCE_TTL_SECONDS: u64 = 600;

/// Returns true when the boot.rs accept loop should hand this
/// request off to [`serve`] instead of falling through to the
/// dispatcher. Match is path-prefix only; method-level rejection
/// happens inside [`serve`].
pub(super) fn is_oidc_request(req: &str) -> bool {
    let line = req.lines().next().unwrap_or("");
    let path = line.split_whitespace().nth(1).unwrap_or("");
    matches!(
        path.split('?').next().unwrap_or(""),
        "/auth/login" | "/auth/callback" | "/auth/logout"
    )
}

/// Stream-owning entrypoint. Builds the full HTTP response so we
/// can set `Location:` and `Set-Cookie:` headers, which the
/// `(status, body)` dispatch tuple used elsewhere cannot do.
pub(super) async fn serve(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let cfg = match load_cfg() {
        Ok(c) if c.enabled => c,
        _ => {
            return write_simple(out, "404 Not Found", r#"{"error":"sso disabled"}"#).await;
        }
    };
    let line = req.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path_with_query = parts.next().unwrap_or("");
    let path = path_with_query.split('?').next().unwrap_or("");
    let query = path_with_query.split_once('?').map(|(_, q)| q).unwrap_or("");
    if method != "GET" {
        return write_simple(out, "405 Method Not Allowed", r#"{"error":"GET only"}"#).await;
    }
    match path {
        "/auth/login" => handle_login(query, &cfg, out).await,
        "/auth/callback" => {
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
            handle_callback(req, query, &cfg, store, &out_resolver(), &secret)
                .await
                .handle(out)
                .await
        }
        "/auth/logout" => handle_logout(&cfg, out).await,
        _ => write_simple(out, "404 Not Found", r#"{"error":"not found"}"#).await,
    }
}

/// Resolver indirection so unit tests can inject a fake IdP
/// without spinning up a TCP listener. Production path uses
/// [`real_token_exchange`].
type TokenResolver = std::sync::Arc<
    dyn Fn(
            String,
            std::collections::BTreeMap<String, String>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<String, String>> + Send>,
        > + Send
        + Sync,
>;

fn out_resolver() -> TokenResolver {
    std::sync::Arc::new(|url, form| Box::pin(real_token_exchange(url, form)))
}

/// Build the IdP authorization URL + PKCE bookkeeping for the
/// `/auth/login` step. Pulled out as a pure function so the unit
/// test can assert PKCE invariants without spinning up TCP.
pub(super) fn build_login_redirect(
    query: &str,
    cfg: &argo_config::SsoConfig,
    rng_bytes: [u8; 32],
    state_bytes: [u8; 16],
) -> Result<LoginRedirect, String> {
    let tenant = query_param(query, "tenant").unwrap_or_else(|| "default".into());
    let tcfg = cfg
        .oidc
        .get(&tenant)
        .ok_or_else(|| format!("no [sso.oidc.{tenant}] config"))?;
    if tcfg.authorization_endpoint.is_empty() {
        return Err("authorization_endpoint not configured".into());
    }
    let scopes = if tcfg.scopes.is_empty() {
        DEFAULT_SCOPES.to_string()
    } else {
        tcfg.scopes.clone()
    };
    let verifier = sso_session::b64url_encode(&rng_bytes);
    let challenge = sso_session::b64url_encode(&Sha256::digest(verifier.as_bytes()));
    let state = sso_session::b64url_encode(&state_bytes);
    let mut url = tcfg.authorization_endpoint.clone();
    let sep = if url.contains('?') { '&' } else { '?' };
    url.push(sep);
    url.push_str(&format!(
        "response_type=code&client_id={cid}&redirect_uri={ru}&scope={sc}&state={st}&code_challenge={ch}&code_challenge_method=S256",
        cid = url_encode(&tcfg.client_id),
        ru = url_encode(&tcfg.redirect_uri),
        sc = url_encode(&scopes),
        st = url_encode(&state),
        ch = url_encode(&challenge),
    ));
    Ok(LoginRedirect {
        url,
        tenant,
        state,
        verifier,
    })
}

/// Outcome of [`build_login_redirect`] — the IdP redirect target
/// and the PKCE bookkeeping shipped in the temp cookie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoginRedirect {
    pub url: String,
    pub tenant: String,
    pub state: String,
    pub verifier: String,
}

async fn handle_login(
    query: &str,
    cfg: &argo_config::SsoConfig,
    out: &mut TcpStream,
) -> tokio::io::Result<()> {
    let mut rng_bytes = [0_u8; 32];
    let mut state_bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut rng_bytes);
    OsRng.fill_bytes(&mut state_bytes);
    let redirect = match build_login_redirect(query, cfg, rng_bytes, state_bytes) {
        Ok(r) => r,
        Err(e) => {
            return write_simple(out, "400 Bad Request", &format!(r#"{{"error":"{e}"}}"#)).await;
        }
    };
    let secret = match super::routes_channels::hmac_secret() {
        Ok(s) => s,
        Err(_) => return write_simple(out, "500 Internal Server Error", r#"{"error":"hmac secret unconfigured"}"#).await,
    };
    let now = chrono::Utc::now().timestamp();
    let pkce_payload = SessionPayload {
        sub: redirect.tenant.clone(),
        email: redirect.state.clone(),
        tenant: redirect.tenant.clone(),
        mechanism: redirect.verifier.clone(),
        iat: now,
        exp: now + i64::try_from(PKCE_TTL_SECONDS).unwrap_or(600),
    };
    let cookie = match sso_session::sign(&pkce_payload, &secret) {
        Ok(c) => c,
        Err(_) => return write_simple(out, "500 Internal Server Error", r#"{"error":"sign pkce"}"#).await,
    };
    let set = set_cookie_header(PKCE_COOKIE, &cookie, PKCE_TTL_SECONDS, "/auth/");
    let resp = format!(
        "HTTP/1.1 302 Found\r\nLocation: {}\r\n{set}\r\nContent-Type: application/json\r\nContent-Length: 0\r\n\r\n",
        redirect.url
    );
    out.write_all(resp.as_bytes()).await?;
    out.shutdown().await
}

/// Callback handler return value — the resolved (status, body,
/// cookie-headers) triple. Modeled as its own type so the async
/// test can assert outcomes without TCP.
pub(super) struct CallbackOutcome {
    pub status: &'static str,
    pub body: String,
    pub location: Option<String>,
    pub set_cookies: Vec<String>,
}

impl CallbackOutcome {
    async fn handle(self, out: &mut TcpStream) -> tokio::io::Result<()> {
        let mut head = format!(
            "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n",
            self.status,
            self.body.len()
        );
        if let Some(loc) = &self.location {
            head.push_str(&format!("Location: {loc}\r\n"));
        }
        for h in &self.set_cookies {
            head.push_str(h);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");
        head.push_str(&self.body);
        out.write_all(head.as_bytes()).await?;
        out.shutdown().await
    }
}

pub(super) async fn handle_callback(
    req: &str,
    query: &str,
    cfg: &argo_config::SsoConfig,
    _store: Option<&Arc<SessionStore>>,
    resolver: &TokenResolver,
    secret: &str,
) -> CallbackOutcome {
    let code = match query_param(query, "code") {
        Some(c) if !c.is_empty() => c,
        _ => return err_outcome("400 Bad Request", "missing code"),
    };
    let state = match query_param(query, "state") {
        Some(s) => s,
        None => return err_outcome("400 Bad Request", "missing state"),
    };
    let pkce_cookie = match read_cookie(req, PKCE_COOKIE) {
        Some(c) => c,
        None => return err_outcome("400 Bad Request", "missing pkce cookie"),
    };
    let now = chrono::Utc::now().timestamp();
    let pkce = match sso_session::verify_with_secret(&pkce_cookie, now, secret) {
        Ok(p) => p,
        Err(e) => return err_outcome("401 Unauthorized", e.reason()),
    };
    if pkce.email != state {
        return err_outcome("401 Unauthorized", "state mismatch");
    }
    let tenant = pkce.tenant.clone();
    let tcfg = match cfg.oidc.get(&tenant) {
        Some(t) => t,
        None => return err_outcome("400 Bad Request", "tenant not configured"),
    };
    let client_secret = argo_config::cred::get(&tcfg.client_secret_env).unwrap_or_default();
    let mut form: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    form.insert("grant_type".into(), "authorization_code".into());
    form.insert("code".into(), code);
    form.insert("redirect_uri".into(), tcfg.redirect_uri.clone());
    form.insert("client_id".into(), tcfg.client_id.clone());
    form.insert("code_verifier".into(), pkce.mechanism.clone());
    if !client_secret.is_empty() {
        form.insert("client_secret".into(), client_secret);
    }
    let body = match (resolver)(tcfg.token_endpoint.clone(), form).await {
        Ok(b) => b,
        Err(e) => return err_outcome("502 Bad Gateway", &format!("token exchange: {e}")),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return err_outcome("502 Bad Gateway", "token response not json"),
    };
    let id_token = match parsed.get("id_token").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return err_outcome("502 Bad Gateway", "no id_token in response"),
    };
    let (sub, email) = match extract_id_token_claims(&id_token) {
        Ok(p) => p,
        Err(e) => return err_outcome("401 Unauthorized", &e),
    };
    let payload = SessionPayload {
        sub,
        email,
        tenant,
        mechanism: "oidc".into(),
        iat: now,
        exp: now + i64::try_from(cfg.session_ttl_seconds).unwrap_or(28_800),
    };
    let token = match sso_session::sign(&payload, secret) {
        Ok(t) => t,
        Err(_) => return err_outcome("500 Internal Server Error", "sign"),
    };
    CallbackOutcome {
        status: "302 Found",
        body: String::new(),
        location: Some("/app/".into()),
        set_cookies: vec![
            set_cookie_header(&cfg.cookie_name, &token, cfg.session_ttl_seconds, "/"),
            clear_cookie_header(PKCE_COOKIE, "/auth/"),
        ],
    }
}

async fn handle_logout(cfg: &argo_config::SsoConfig, out: &mut TcpStream) -> tokio::io::Result<()> {
    let clear = clear_cookie_header(&cfg.cookie_name, "/");
    let resp = format!(
        "HTTP/1.1 302 Found\r\nLocation: /\r\n{clear}\r\nContent-Type: application/json\r\nContent-Length: 0\r\n\r\n"
    );
    out.write_all(resp.as_bytes()).await?;
    out.shutdown().await
}

/// Parse an OIDC ID token's payload claims without verifying its
/// signature. v1 posture: we received the token directly from the
/// IdP `token_endpoint` over TLS while presenting our client
/// secret, so the JWT signature would be a belt-and-braces check
/// rather than the load-bearing trust anchor. Returns
/// `(sub, email)`; `email` falls back to the empty string when
/// the IdP omits it.
pub(super) fn extract_id_token_claims(token: &str) -> Result<(String, String), String> {
    let mut parts = token.split('.');
    let _ = parts.next().ok_or("malformed jwt".to_string())?;
    let payload = parts.next().ok_or("malformed jwt".to_string())?;
    let bytes = sso_session::b64url_decode(payload).map_err(|_| "b64 decode")?;
    let v: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "id_token payload not json")?;
    let sub = v
        .get("sub")
        .and_then(|s| s.as_str())
        .ok_or("no sub claim")?
        .to_string();
    let email = v
        .get("email")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    Ok((sub, email))
}

async fn real_token_exchange(
    url: String,
    form: std::collections::BTreeMap<String, String>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("build client: {e}"))?;
    let body: String = form
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let resp = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("send: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("idp status {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("read: {e}"))
}

fn load_cfg() -> Result<argo_config::SsoConfig, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let cfg = argo_config::ArgoConfig::load(&cwd.join("argo.toml"))
        .map_err(|e| format!("load: {e}"))?;
    Ok(cfg.sso)
}

fn err_outcome(status: &'static str, msg: &str) -> CallbackOutcome {
    CallbackOutcome {
        status,
        body: format!(r#"{{"error":"{msg}"}}"#),
        location: None,
        set_cookies: vec![],
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

pub(super) fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub(super) fn query_param(query: &str, key: &str) -> Option<String> {
    for kv in query.split('&') {
        let (k, v) = kv.split_once('=')?;
        if k == key {
            return Some(percent_decode(v));
        }
    }
    None
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

#[cfg(test)]
#[path = "sso_oidc_tests.rs"]
mod tests;
