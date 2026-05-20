//! Round-trip + tamper tests for the session-JWT verifier.
#![cfg(test)]

use super::super::auth_magic_link::mint_jwt;
use super::{
    bearer_from_request, extract_session_claims, session_cookie_from_request, verify_session_jwt,
};

const JWT_SECRET_ENV: &str = "SKYCOT_AUTH_JWT_SECRET";

fn fresh_secret() {
    std::env::set_var(JWT_SECRET_ENV, "session-jwt-tests-secret-deadbeef");
}

#[test]
fn round_trip_claims() {
    fresh_secret();
    let now = chrono::Utc::now().timestamp();
    let (jwt, _jti) = mint_jwt(
        "user-1",
        "alice@example.com",
        Some("tenant-1"),
        Some("owner"),
        now,
    )
    .expect("mint");
    let claims = verify_session_jwt(&jwt).expect("verify");
    assert_eq!(claims.sub, "user-1");
    assert_eq!(claims.email, "alice@example.com");
    assert_eq!(claims.tnt.as_deref(), Some("tenant-1"));
    assert_eq!(claims.role.as_deref(), Some("owner"));
}

#[test]
fn rejects_tampered_payload() {
    fresh_secret();
    let now = chrono::Utc::now().timestamp();
    let (jwt, _jti) = mint_jwt("u", "e", Some("t"), Some("guest"), now).expect("mint");
    let mut parts: Vec<&str> = jwt.splitn(3, '.').collect();
    // Replace payload with an "owner" claim — signature should fail.
    parts[1] =
        "eyJzdWIiOiJ1IiwiZW1haWwiOiJlIiwidG50IjoidCIsInJvbGUiOiJvd25lciIsImlhdCI6MCwiZXhwIjo5OTk5OTk5OTk5fQ";
    let tampered = parts.join(".");
    assert!(verify_session_jwt(&tampered).is_none());
}

#[test]
fn rejects_static_bearer_tokens() {
    fresh_secret();
    // A static bearer token (no dots) is unambiguously not a JWT.
    assert!(verify_session_jwt("static-bearer-token-abc").is_none());
}

#[test]
fn rejects_expired_jwt() {
    fresh_secret();
    // 2-hour-old token with exp at iat+1h
    let now = chrono::Utc::now().timestamp() - 2 * 60 * 60;
    // mint_jwt sets exp = now + 7d so we can't easily make this expire
    // — instead, hand-craft a payload that's already expired.
    // The verifier honors the encoded exp, not its own wall clock vs
    // iat, so a clean expired token is enough.
    use base64::Engine as _;
    use jsonwebtoken::{EncodingKey, Header};
    let header = Header::new(jsonwebtoken::Algorithm::HS256);
    #[derive(serde::Serialize)]
    struct Expired<'a> {
        sub: &'a str,
        email: &'a str,
        tnt: Option<&'a str>,
        role: Option<&'a str>,
        iat: i64,
        exp: i64,
    }
    let key = EncodingKey::from_secret(b"session-jwt-tests-secret-deadbeef");
    let _ = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"x");
    let expired = jsonwebtoken::encode(
        &header,
        &Expired {
            sub: "u",
            email: "e",
            tnt: None,
            role: Some("owner"),
            iat: now,
            exp: now + 100,
        },
        &key,
    )
    .expect("encode");
    assert!(verify_session_jwt(&expired).is_none());
}

#[test]
fn bearer_from_request_strips_scheme() {
    let req = "POST / HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer xyz.abc.123\r\n\r\n";
    assert_eq!(bearer_from_request(req).as_deref(), Some("xyz.abc.123"));
    let req = "POST / HTTP/1.1\r\nHost: x\r\n\r\n";
    assert_eq!(bearer_from_request(req), None);
}

#[test]
fn session_cookie_picks_correct_pair() {
    let req = "GET / HTTP/1.1\r\nHost: x\r\nCookie: other=1; __Host-session=ABCD.EF.GH; foo=bar\r\n\r\n";
    assert_eq!(
        session_cookie_from_request(req).as_deref(),
        Some("ABCD.EF.GH")
    );
    let req = "GET / HTTP/1.1\r\nHost: x\r\nCookie: nope=1\r\n\r\n";
    assert_eq!(session_cookie_from_request(req), None);
}

#[test]
fn extract_session_claims_prefers_bearer_then_cookie() {
    fresh_secret();
    let now = chrono::Utc::now().timestamp();
    let (jwt, _jti) = mint_jwt("u", "e", Some("t"), Some("admin"), now).expect("mint");
    // Bearer wins.
    let req = format!(
        "GET / HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {jwt}\r\nCookie: __Host-session=garbage\r\n\r\n"
    );
    let claims = extract_session_claims(&req).expect("verify");
    assert_eq!(claims.role.as_deref(), Some("admin"));
    // Cookie-only path.
    let req = format!(
        "GET / HTTP/1.1\r\nHost: x\r\nCookie: __Host-session={jwt}\r\n\r\n"
    );
    let claims = extract_session_claims(&req).expect("verify");
    assert_eq!(claims.role.as_deref(), Some("admin"));
}
