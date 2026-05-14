//! Unit tests for the OIDC SSO middleware. Each test is hermetic
//! (no network, no TCP, no filesystem) — the IdP token endpoint is
//! mocked through the [`TokenResolver`] indirection.
use super::*;
use argo_config::{SsoConfig, SsoOidcTenant};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
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
            issuer: "https://idp.example.com".into(),
            jwks_uri: format!("https://idp.example.com/{name}/jwks"),
            jwks_cache_ttl_secs: 3600,
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

const RSA_PRIVATE_KEY: &str = r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAyRE6rHuNR0QbHO3H3Kt2pOKGVhQqGZXInOduQNxXzuKlvQTL
UTv4l4sggh5/CYYi/cvI+SXVT9kPWSKXxJXBXd/4LkvcPuUakBoAkfh+eiFVMh2V
rUyWyj3MFl0HTVF9KwRXLAcwkREiS3npThHRyIxuy0ZMeZfxVL5arMhw1SRELB8H
oGfG/AtH89BIE9jDBHZ9dLelK9a184zAf8LwoPLxvJb3Il5nncqPcSfKDDodMFBI
Mc4lQzDKL5gvmiXLXB1AGLm8KBjfE8s3L5xqi+yUod+j8MtvIj812dkS4QMiRVN/
by2h3ZY8LYVGrqZXZTcgn2ujn8uKjXLZVD5TdQIDAQABAoIBAHREk0I0O9DvECKd
WUpAmF3mY7oY9PNQiu44Yaf+AoSuyRpRUGTMIgc3u3eivOE8ALX0BmYUO5JtuRNZ
Dpvt4SAwqCnVUinIf6C+eH/wSurCpapSM0BAHp4aOA7igptyOMgMPYBHNA1e9A7j
E0dCxKWMl3DSWNyjQTk4zeRGEAEfbNjHrq6YCtjHSZSLmWiG80hnfnYos9hOr5Jn
LnyS7ZmFE/5P3XVrxLc/tQ5zum0R4cbrgzHiQP5RgfxGJaEi7XcgherCCOgurJSS
bYH29Gz8u5fFbS+Yg8s+OiCss3cs1rSgJ9/eHZuzGEdUZVARH6hVMjSuwvqVTFaE
8AgtleECgYEA+uLMn4kNqHlJS2A5uAnCkj90ZxEtNm3E8hAxUrhssktY5XSOAPBl
xyf5RuRGIImGtUVIr4HuJSa5TX48n3Vdt9MYCprO/iYl6moNRSPt5qowIIOJmIjY
2mqPDfDt/zw+fcDD3lmCJrFlzcnh0uea1CohxEbQnL3cypeLt+WbU6kCgYEAzSp1
9m1ajieFkqgoB0YTpt/OroDx38vvI5unInJlEeOjQ+oIAQdN2wpxBvTrRorMU6P0
7mFUbt1j+Co6CbNiw+X8HcCaqYLR5clbJOOWNR36PuzOpQLkfK8woupBxzW9B8gZ
mY8rB1mbJ+/WTPrEJy6YGmIEBkWylQ2VpW8O4O0CgYEApdbvvfFBlwD9YxbrcGz7
MeNCFbMz+MucqQntIKoKJ91ImPxvtc0y6e/Rhnv0oyNlaUOwJVu0yNgNG117w0g4
t/+Q38mvVC5xV7/cn7x9UMFk6MkqVir3dYGEqIl/OP1grY2Tq9HtB5iyG9L8NIam
QOLMyUqqMUILxdthHyFmiGkCgYEAn9+PjpjGMPHxL0gj8Q8VbzsFtou6b1deIRRA
2CHmSltltR1gYVTMwXxQeUhPMmgkMqUXzs4/WijgpthY44hK1TaZEKIuoxrS70nJ
4WQLf5a9k1065fDsFZD6yGjdGxvwEmlGMZgTwqV7t1I4X0Ilqhav5hcs5apYL7gn
PYPeRz0CgYALHCj/Ji8XSsDoF/MhVhnGdIs2P99NNdmo3R2Pv0CuZbDKMU559LJH
UvrKS8WkuWRDuKrz1W/EQKApFjDGpdqToZqriUFQzwy7mR3ayIiogzNtHcvbDHx8
oFnGY0OFksX/ye0/XGpy2SFxYRwGU98HPYeBvAQQrVjdkzfy7BmXQQ==
-----END RSA PRIVATE KEY-----"#;

fn jwks() -> JwkSet {
    serde_json::from_value(serde_json::json!({
        "keys": [{
            "kty": "RSA",
            "n": "yRE6rHuNR0QbHO3H3Kt2pOKGVhQqGZXInOduQNxXzuKlvQTLUTv4l4sggh5_CYYi_cvI-SXVT9kPWSKXxJXBXd_4LkvcPuUakBoAkfh-eiFVMh2VrUyWyj3MFl0HTVF9KwRXLAcwkREiS3npThHRyIxuy0ZMeZfxVL5arMhw1SRELB8HoGfG_AtH89BIE9jDBHZ9dLelK9a184zAf8LwoPLxvJb3Il5nncqPcSfKDDodMFBIMc4lQzDKL5gvmiXLXB1AGLm8KBjfE8s3L5xqi-yUod-j8MtvIj812dkS4QMiRVN_by2h3ZY8LYVGrqZXZTcgn2ujn8uKjXLZVD5TdQ",
            "e": "AQAB",
            "kid": "rsa01",
            "alg": "RS256",
            "use": "sig"
        }]
    }))
    .unwrap()
}

fn signed_id_token(overrides: serde_json::Value, kid: Option<&str>) -> String {
    let mut claims = serde_json::json!({
        "sub": "u-1",
        "email": "alice@example.com",
        "iss": "https://idp.example.com",
        "aud": "test-client",
        "exp": 2_000_000_000_i64,
        "iat": 1_700_000_000_i64,
        "nonce": "state-abc"
    });
    let obj = claims.as_object_mut().unwrap();
    for (key, value) in overrides.as_object().unwrap() {
        obj.insert(key.clone(), value.clone());
    }
    let mut header = Header::new(Algorithm::RS256);
    header.kid = kid.map(str::to_string);
    encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(RSA_PRIVATE_KEY.as_bytes()).unwrap(),
    )
    .unwrap()
}

#[test]
fn build_login_redirect_includes_pkce_challenge() {
    let cfg = cfg_with_tenant("default");
    let r = build_login_redirect("tenant=default", &cfg, [7_u8; 32], [3_u8; 16]).unwrap();
    assert!(r.url.starts_with("https://idp.example.com/authorize?"));
    assert!(r.url.contains("response_type=code"));
    assert!(r.url.contains("code_challenge_method=S256"));
    assert!(r.url.contains("state="));
    assert!(r.url.contains("nonce="));
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
    let token = signed_id_token(serde_json::json!({}), Some("rsa01"));
    let tenant = cfg.oidc.get("default").unwrap();
    sso_oidc_jwt::seed_jwks(&tenant.jwks_uri, now, jwks()).await;
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
    assert!(outcome.set_cookies.iter().any(|c| c.contains("argo_sso=")));
}

#[test]
fn verify_id_token_accepts_good_rs256_token() {
    let cfg = cfg_with_tenant("default");
    let token = signed_id_token(serde_json::json!({}), Some("rsa01"));
    let claims = sso_oidc_jwt::verify_and_parse_id_token(
        &token,
        &jwks(),
        cfg.oidc.get("default").unwrap(),
        1_800_000_000,
        "state-abc",
    )
    .unwrap();
    assert_eq!(claims.sub, "u-1");
    assert_eq!(claims.email, "alice@example.com");
}

#[test]
fn verify_id_token_rejects_bad_signature() {
    let cfg = cfg_with_tenant("default");
    let mut token = signed_id_token(serde_json::json!({}), Some("rsa01"));
    token.push('x');
    let err = sso_oidc_jwt::verify_and_parse_id_token(
        &token,
        &jwks(),
        cfg.oidc.get("default").unwrap(),
        1_800_000_000,
        "state-abc",
    )
    .unwrap_err();
    assert_eq!(err, sso_oidc_jwt::VerifyError::SignatureRejected);
}

#[test]
fn verify_id_token_rejects_expired() {
    let cfg = cfg_with_tenant("default");
    let token = signed_id_token(serde_json::json!({"exp": 1_700_000_000_i64}), Some("rsa01"));
    let err = sso_oidc_jwt::verify_and_parse_id_token(
        &token,
        &jwks(),
        cfg.oidc.get("default").unwrap(),
        1_800_000_000,
        "state-abc",
    )
    .unwrap_err();
    assert_eq!(err, sso_oidc_jwt::VerifyError::Expired);
}

#[test]
fn verify_id_token_rejects_wrong_audience() {
    let cfg = cfg_with_tenant("default");
    let token = signed_id_token(serde_json::json!({"aud": "other-client"}), Some("rsa01"));
    let err = sso_oidc_jwt::verify_and_parse_id_token(
        &token,
        &jwks(),
        cfg.oidc.get("default").unwrap(),
        1_800_000_000,
        "state-abc",
    )
    .unwrap_err();
    assert_eq!(err, sso_oidc_jwt::VerifyError::AudRejected);
}

#[test]
fn verify_id_token_rejects_wrong_issuer() {
    let cfg = cfg_with_tenant("default");
    let token = signed_id_token(
        serde_json::json!({"iss": "https://evil.example"}),
        Some("rsa01"),
    );
    let err = sso_oidc_jwt::verify_and_parse_id_token(
        &token,
        &jwks(),
        cfg.oidc.get("default").unwrap(),
        1_800_000_000,
        "state-abc",
    )
    .unwrap_err();
    assert_eq!(err, sso_oidc_jwt::VerifyError::IssuerRejected);
}

#[test]
fn verify_id_token_rejects_nonce_mismatch() {
    let cfg = cfg_with_tenant("default");
    let token = signed_id_token(serde_json::json!({"nonce": "other"}), Some("rsa01"));
    let err = sso_oidc_jwt::verify_and_parse_id_token(
        &token,
        &jwks(),
        cfg.oidc.get("default").unwrap(),
        1_800_000_000,
        "state-abc",
    )
    .unwrap_err();
    assert_eq!(err, sso_oidc_jwt::VerifyError::NonceRejected);
}

#[test]
fn verify_id_token_rejects_missing_kid() {
    let cfg = cfg_with_tenant("default");
    let token = signed_id_token(serde_json::json!({}), None);
    let err = sso_oidc_jwt::verify_and_parse_id_token(
        &token,
        &jwks(),
        cfg.oidc.get("default").unwrap(),
        1_800_000_000,
        "state-abc",
    )
    .unwrap_err();
    assert_eq!(err, sso_oidc_jwt::VerifyError::MissingKid);
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
    let resolver: TokenResolver = std::sync::Arc::new(|_, _| Box::pin(async { Ok(String::new()) }));
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
    let resolver: TokenResolver = std::sync::Arc::new(|_, _| Box::pin(async { Ok(String::new()) }));
    let outcome =
        handle_callback(req, "code=AUTH&state=s", &cfg, None, &resolver, TEST_SECRET).await;
    assert_eq!(outcome.status, "400 Bad Request");
    assert!(outcome.body.contains("missing pkce cookie"));
}

#[test]
fn is_oidc_request_matches_known_paths() {
    assert!(is_oidc_request("GET /auth/login HTTP/1.1\r\n\r\n"));
    assert!(is_oidc_request(
        "GET /auth/callback?code=x HTTP/1.1\r\n\r\n"
    ));
    assert!(is_oidc_request("GET /auth/logout HTTP/1.1\r\n\r\n"));
    assert!(!is_oidc_request(
        "GET /auth/saml/default/login HTTP/1.1\r\n\r\n"
    ));
    assert!(!is_oidc_request("GET /sessions HTTP/1.1\r\n\r\n"));
}
