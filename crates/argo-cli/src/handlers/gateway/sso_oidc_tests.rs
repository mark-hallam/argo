//! Unit tests for the OIDC SSO middleware. Each test is hermetic
//! (no network, no TCP, no filesystem) — the IdP token endpoint is
//! mocked through the [`TokenResolver`] indirection.
use super::*;
use argo_config::{SsoConfig, SsoOidcTenant};
use std::collections::BTreeMap;

fn cfg_with_tenant(name: &str) -> SsoConfig {
    let mut oidc = BTreeMap::new();
    oidc.insert(
        name.into(),
        SsoOidcTenant {
            client_id: "test-client".into(),
            client_secret_env: "ARGO_TEST_OIDC_SECRET".into(),
            authorization_endpoint: "https://idp.example.com/authorize".into(),
            token_endpoint: "https://idp.example.com/token".into(),
            redirect_uri: "https://sp.example.com/auth/callback".into(),
            scopes: "openid email".into(),
        },
    );
    SsoConfig {
        enabled: true,
        session_ttl_seconds: 3600,
        cookie_name: "argo_sso".into(),
        oidc,
        saml: BTreeMap::new(),
    }
}

fn b64url_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn mint_id_token(sub: &str, email: &str) -> String {
    let header = b64url_encode(br#"{"alg":"none","typ":"JWT"}"#);
    let payload_json = format!(r#"{{"sub":"{sub}","email":"{email}"}}"#);
    let payload = b64url_encode(payload_json.as_bytes());
    format!("{header}.{payload}.sig")
}

#[test]
fn build_login_redirect_includes_pkce_challenge() {
    let cfg = cfg_with_tenant("default");
    let r = build_login_redirect("tenant=default", &cfg, [7_u8; 32], [3_u8; 16]).unwrap();
    assert!(r.url.starts_with("https://idp.example.com/authorize?"));
    assert!(r.url.contains("response_type=code"));
    assert!(r.url.contains("code_challenge_method=S256"));
    assert!(r.url.contains("state="));
    assert!(r.url.contains("code_challenge="));
    assert!(r.url.contains("client_id=test-client"));
    // Verifier is base64url of the random bytes; challenge is
    // base64url(sha256(verifier)) — confirm via length only here.
    assert_eq!(r.tenant, "default");
    assert!(!r.verifier.is_empty());
    assert!(!r.state.is_empty());
}

#[test]
fn build_login_redirect_rejects_unknown_tenant() {
    let cfg = cfg_with_tenant("default");
    let err = build_login_redirect("tenant=other", &cfg, [0; 32], [0; 16]).unwrap_err();
    assert!(err.contains("no [sso.oidc.other]"));
}

#[test]
fn build_login_defaults_scopes_when_empty() {
    let mut cfg = cfg_with_tenant("default");
    cfg.oidc.get_mut("default").unwrap().scopes = String::new();
    let r = build_login_redirect("", &cfg, [0; 32], [0; 16]).unwrap();
    assert!(r.url.contains("scope=openid%20email%20profile"));
}

#[test]
fn extract_id_token_pulls_sub_and_email() {
    let token = mint_id_token("u-42", "operator@example.com");
    let (sub, email) = extract_id_token_claims(&token).unwrap();
    assert_eq!(sub, "u-42");
    assert_eq!(email, "operator@example.com");
}

#[test]
fn extract_id_token_rejects_malformed() {
    let err = extract_id_token_claims("not-a-jwt").unwrap_err();
    assert!(err.contains("malformed") || err.contains("b64") || err.contains("json"));
}

const TEST_SECRET: &str = "oidc-test-secret-must-be-32-chars-or-more";

#[tokio::test]
async fn callback_round_trips_with_mock_idp() {
    let cfg = cfg_with_tenant("default");
    // Pre-mint a PKCE cookie whose `email` carries the state and
    // whose `mechanism` carries the verifier (the cookie's
    // SessionPayload doubles as a transport here — see sso_oidc.rs
    // `handle_login`).
    let now = chrono::Utc::now().timestamp();
    let pkce = SessionPayload {
        sub: "default".into(),
        email: "state-abc".into(),
        tenant: "default".into(),
        mechanism: "verifier-xyz".into(),
        iat: now,
        exp: now + 600,
    };
    let cookie = sso_session::sign(&pkce, TEST_SECRET).unwrap();
    let req = format!(
        "GET /auth/callback?code=AUTH&state=state-abc HTTP/1.1\r\nCookie: argo_sso_pkce={cookie}\r\n\r\n",
    );
    let token = mint_id_token("u-1", "alice@example.com");
    let resolver: TokenResolver = std::sync::Arc::new(move |_url, _form| {
        let token = token.clone();
        Box::pin(async move { Ok(format!(r#"{{"id_token":"{token}"}}"#)) })
    });
    let outcome = handle_callback(
        &req,
        "code=AUTH&state=state-abc",
        &cfg,
        None,
        &resolver,
        TEST_SECRET,
    )
    .await;
    assert_eq!(outcome.status, "302 Found");
    assert_eq!(outcome.location.as_deref(), Some("/app/"));
    assert!(outcome
        .set_cookies
        .iter()
        .any(|c| c.contains("argo_sso=")));
}

#[tokio::test]
async fn callback_rejects_state_mismatch() {
    let cfg = cfg_with_tenant("default");
    let now = chrono::Utc::now().timestamp();
    let pkce = SessionPayload {
        sub: "default".into(),
        email: "state-real".into(),
        tenant: "default".into(),
        mechanism: "v".into(),
        iat: now,
        exp: now + 600,
    };
    let cookie = sso_session::sign(&pkce, TEST_SECRET).unwrap();
    let req = format!(
        "GET /auth/callback?code=AUTH&state=state-FORGED HTTP/1.1\r\nCookie: argo_sso_pkce={cookie}\r\n\r\n",
    );
    let resolver: TokenResolver =
        std::sync::Arc::new(|_, _| Box::pin(async { Ok(String::new()) }));
    let outcome = handle_callback(
        &req,
        "code=AUTH&state=state-FORGED",
        &cfg,
        None,
        &resolver,
        TEST_SECRET,
    )
    .await;
    assert_eq!(outcome.status, "401 Unauthorized");
    assert!(outcome.body.contains("state mismatch"));
}

#[tokio::test]
async fn callback_rejects_missing_pkce_cookie() {
    let cfg = cfg_with_tenant("default");
    let req = "GET /auth/callback?code=AUTH&state=s HTTP/1.1\r\n\r\n";
    let resolver: TokenResolver =
        std::sync::Arc::new(|_, _| Box::pin(async { Ok(String::new()) }));
    let outcome = handle_callback(
        req,
        "code=AUTH&state=s",
        &cfg,
        None,
        &resolver,
        TEST_SECRET,
    )
    .await;
    assert_eq!(outcome.status, "400 Bad Request");
    assert!(outcome.body.contains("missing pkce cookie"));
}

#[test]
fn is_oidc_request_matches_known_paths() {
    assert!(is_oidc_request("GET /auth/login HTTP/1.1\r\n\r\n"));
    assert!(is_oidc_request("GET /auth/callback?code=x HTTP/1.1\r\n\r\n"));
    assert!(is_oidc_request("GET /auth/logout HTTP/1.1\r\n\r\n"));
    assert!(!is_oidc_request("GET /auth/saml/default/login HTTP/1.1\r\n\r\n"));
    assert!(!is_oidc_request("GET /sessions HTTP/1.1\r\n\r\n"));
}
