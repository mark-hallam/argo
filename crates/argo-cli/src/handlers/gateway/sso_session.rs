//! Signed-cookie session helper for the SSO middleware
//! (`mission-sso-saml-oidc-gateway`).
//!
//! Format
//! ------
//!
//! `<base64url(payload_json)>.<hex(hmac_sha256)>`
//!
//! - `payload_json` is the [`SessionPayload`] serialized as JSON.
//! - `hmac_sha256` is taken over the base64url payload using the
//!   gateway's `ARGO_CHANNEL_HMAC_SECRET` (shared with the DTMF /
//!   channels signing path; explicit in the brief as intentional
//!   single-host posture).
//!
//! Tamper detection: any byte flipped in either segment fails the
//! constant-time HMAC compare in [`verify`]. The constant-time
//! comparator + HMAC primitives are reused from
//! [`super::routes_channels`] so we don't reinvent crypto.

use serde::{Deserialize, Serialize};

use super::routes_channels::{constant_time_eq, hmac_hex};

/// Claims persisted in the SSO session cookie. The payload doubles as
/// the audit-side principal record so the runtime can attribute
/// requests without a second store hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SessionPayload {
    /// IdP-issued subject identifier (`sub` from OIDC id-token, or
    /// SAML `NameID`). Stable across IdP logins for the same human.
    pub sub: String,
    /// Operator email (empty string when the IdP did not return one).
    pub email: String,
    /// Tenant id the IdP was registered under — drives
    /// multi-tenant route isolation.
    pub tenant: String,
    /// `oidc` | `saml`. Carried so audit emissions can distinguish
    /// the auth path without re-reading config.
    pub mechanism: String,
    /// Issued-at (unix seconds).
    pub iat: i64,
    /// Expiry (unix seconds).
    pub exp: i64,
}

/// Result of decoding a presented cookie value. The variant feeds
/// 401 responses without leaking which check failed beyond the
/// stable reason string returned by [`SessionError::reason`].
#[derive(Debug, PartialEq, Eq)]
pub(super) enum SessionError {
    /// Cookie missing the `.` separator or has extra segments.
    Malformed,
    /// One of the segments is not valid base64url / hex.
    Encoding,
    /// JSON payload doesn't deserialize.
    Json,
    /// HMAC compare failed — tamper or wrong secret.
    Signature,
    /// `exp` is at/before `now`.
    Expired,
}

impl SessionError {
    pub(super) fn reason(&self) -> &'static str {
        match self {
            SessionError::Malformed => "malformed_cookie",
            SessionError::Encoding => "cookie_encoding",
            SessionError::Json => "cookie_json",
            SessionError::Signature => "cookie_signature",
            SessionError::Expired => "session_expired",
        }
    }
}

/// Mint a `<b64payload>.<hexhmac>` token. `secret` is hexified once
/// per session creation — callers should resolve it from
/// [`hmac_secret`].
pub(super) fn sign(payload: &SessionPayload, secret: &str) -> Result<String, SessionError> {
    let json = serde_json::to_vec(payload).map_err(|_| SessionError::Json)?;
    let b64 = b64url_encode(&json);
    let mac = hmac_hex(b64.as_bytes(), secret.as_bytes());
    Ok(format!("{b64}.{mac}"))
}

/// Verify a presented `<b64payload>.<hexhmac>` value. `now_unix` and
/// `secret` are parameterized so unit tests can pin the clock and
/// pass a fixture key without round-tripping through the
/// process-wide `ARGO_CHANNEL_HMAC_SECRET` env var.
pub(super) fn verify_with_secret(
    token: &str,
    now_unix: i64,
    secret: &str,
) -> Result<SessionPayload, SessionError> {
    let (b64, mac) = token.split_once('.').ok_or(SessionError::Malformed)?;
    if b64.is_empty() || mac.is_empty() || mac.contains('.') {
        return Err(SessionError::Malformed);
    }
    let expected = hmac_hex(b64.as_bytes(), secret.as_bytes());
    if !constant_time_eq(expected.as_bytes(), mac.as_bytes()) {
        return Err(SessionError::Signature);
    }
    let bytes = b64url_decode(b64).map_err(|_| SessionError::Encoding)?;
    let payload: SessionPayload =
        serde_json::from_slice(&bytes).map_err(|_| SessionError::Json)?;
    if payload.exp <= now_unix {
        return Err(SessionError::Expired);
    }
    Ok(payload)
}

/// Format a `Set-Cookie` header value carrying the SSO session
/// token. HttpOnly + Secure + SameSite=Lax per brief notes.
pub(super) fn set_cookie_header(name: &str, token: &str, ttl_seconds: u64, path: &str) -> String {
    format!(
        "Set-Cookie: {name}={token}; Max-Age={ttl_seconds}; Path={path}; HttpOnly; Secure; SameSite=Lax"
    )
}

/// Format a `Set-Cookie` header value that clears the named cookie.
/// Mirrors [`set_cookie_header`] attributes so the browser matches
/// and deletes the original.
pub(super) fn clear_cookie_header(name: &str, path: &str) -> String {
    format!("Set-Cookie: {name}=; Max-Age=0; Path={path}; HttpOnly; Secure; SameSite=Lax")
}

/// Look up the value of a named cookie inside an HTTP request's
/// `Cookie:` header. Returns `None` when the header is absent or
/// the cookie is not present.
pub(super) fn read_cookie(req: &str, name: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    let line = headers
        .lines()
        .skip(1)
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case("Cookie").then(|| v.trim())
        })?;
    for kv in line.split(';') {
        let kv = kv.trim();
        if let Some(rest) = kv.strip_prefix(&format!("{name}=")) {
            return Some(rest.to_string());
        }
    }
    None
}

pub(super) fn b64url_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub(super) fn b64url_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "session-test-secret-must-be-32-chars-min";

    fn sample(exp: i64) -> SessionPayload {
        SessionPayload {
            sub: "u-123".into(),
            email: "op@example.com".into(),
            tenant: "default".into(),
            mechanism: "oidc".into(),
            iat: 100,
            exp,
        }
    }

    #[test]
    fn sign_then_verify_round_trips() {
        let payload = sample(1_000);
        let token = sign(&payload, TEST_SECRET).unwrap();
        let back = verify_with_secret(&token, 500, TEST_SECRET).unwrap();
        assert_eq!(back, payload);
    }

    #[test]
    fn tamper_in_payload_segment_rejects() {
        let token = sign(&sample(1_000), TEST_SECRET).unwrap();
        let (b64, mac) = token.split_once('.').unwrap();
        let mut chars: Vec<char> = b64.chars().collect();
        chars[0] = if chars[0] == 'A' { 'B' } else { 'A' };
        let bad = format!("{}.{mac}", chars.iter().collect::<String>());
        let err = verify_with_secret(&bad, 500, TEST_SECRET).unwrap_err();
        // Either signature mismatch or encoding break — both
        // are tamper signals, not session-valid.
        assert!(matches!(
            err,
            SessionError::Signature | SessionError::Encoding | SessionError::Json
        ));
    }

    #[test]
    fn expired_session_is_rejected() {
        let token = sign(&sample(100), TEST_SECRET).unwrap();
        let err = verify_with_secret(&token, 200, TEST_SECRET).unwrap_err();
        assert_eq!(err, SessionError::Expired);
    }

    #[test]
    fn read_cookie_finds_named_value() {
        let req = "GET /app HTTP/1.1\r\nCookie: foo=1; argo_sso=abc.def; bar=2\r\n\r\n";
        assert_eq!(read_cookie(req, "argo_sso"), Some("abc.def".into()));
        assert_eq!(read_cookie(req, "missing"), None);
    }

    #[test]
    fn missing_separator_is_malformed() {
        let err = verify_with_secret("abcnoseparator", 0, TEST_SECRET).unwrap_err();
        assert_eq!(err, SessionError::Malformed);
    }

    #[test]
    fn clear_cookie_zeroes_max_age() {
        let h = clear_cookie_header("argo_sso", "/");
        assert!(h.contains("Max-Age=0"));
        assert!(h.contains("HttpOnly"));
    }
}
