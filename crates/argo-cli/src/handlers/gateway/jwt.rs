//! Ed25519-signed JWT decode + verify for the `/inbox/ws` query-param
//! auth path (J3).
//!
//! v1 posture (see brief): browser WebSocket can't send custom headers
//! so the bearer token is wrapped in a short-lived JWT and shipped as
//! `?token=<JWT>`. The server verifies the JWT against an Ed25519
//! public key shipped at deploy time. v1.5 (`mission-enterprise-ws-jwt-rotation`)
//! adds per-tenant signing keys with rotation history.
//!
//! Format (brief §JWT format):
//! - Header: `{"alg":"EdDSA","typ":"JWT"}`
//! - Payload: `{"sub":"<principal_id>","tnt":"<tenant_id>","iat":<unix>,"exp":<unix>}`
//! - Signature: Ed25519 over `<base64url(header)>.<base64url(payload)>`,
//!   then base64url'd unpadded.
//!
//! Public-key sourcing:
//! - `ARGO_INBOX_WS_JWT_PUBKEY` env var (base64-encoded 32-byte raw
//!   Ed25519 public key) — preferred for tests and ephemeral deploys.
//! - `ARGO_INBOX_WS_JWT_PUBKEY_PEM` env var (PEM block matching the
//!   `ARGO RELEASE PUBKEY` format used by `self_update`) — preferred
//!   for production rotation via file template.
//! - If neither is set, every JWT verification fails closed (the WS
//!   handler falls back to the legacy bearer-token auth path).

use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

const PUBKEY_HEADER: &str = "-----BEGIN ARGO RELEASE PUBKEY-----";
const PUBKEY_FOOTER: &str = "-----END ARGO RELEASE PUBKEY-----";

/// JWT verification errors. Variants are reported back to the client
/// verbatim in the 401 body, so keep messages low-detail.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum JwtError {
    /// No `ARGO_INBOX_WS_JWT_PUBKEY*` env var is set on the server.
    NoPubkey,
    /// Token shape doesn't match `header.payload.signature`.
    Malformed,
    /// One of the three segments isn't valid base64url.
    Base64,
    /// Header or payload isn't JSON / is missing a required claim.
    Json,
    /// Header `alg` field is not `EdDSA` (or `typ` is not `JWT`).
    UnsupportedAlg,
    /// Signature failed Ed25519 verification.
    BadSignature,
    /// `exp` is at/before now (or `iat` is in the future by > 60s skew).
    Expired,
}

impl JwtError {
    /// One-word reason emitted as the 401 body so clients can branch
    /// on token failure mode without paging through prose.
    pub(super) fn reason(&self) -> &'static str {
        match self {
            JwtError::NoPubkey => "jwt_pubkey_not_configured",
            JwtError::Malformed => "malformed_jwt",
            JwtError::Base64 => "jwt_base64",
            JwtError::Json => "jwt_json",
            JwtError::UnsupportedAlg => "unsupported_alg",
            JwtError::BadSignature => "bad_signature",
            JwtError::Expired => "expired",
        }
    }
}

/// Decoded JWT payload. Only the brief-locked claim names are
/// preserved; unknown fields are dropped during decode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Claims {
    /// Subject — the principal id (operator or worker).
    pub sub: String,
    /// Tenant id — the broadcast key.
    pub tnt: String,
    /// Issued-at (unix seconds).
    pub iat: i64,
    /// Expiry (unix seconds).
    pub exp: i64,
}

#[derive(Debug, Deserialize)]
struct Header {
    alg: String,
    #[serde(default)]
    typ: Option<String>,
}

/// Verify an `?token=<JWT>` against the server's configured Ed25519
/// public key. `now_unix` is parameterized so unit tests can pin time
/// without monkeypatching the clock.
pub(super) fn verify_jwt(token: &str, now_unix: i64) -> Result<Claims, JwtError> {
    let pubkey = load_pubkey()?;
    let mut parts = token.split('.');
    let header_b64 = parts.next().ok_or(JwtError::Malformed)?;
    let payload_b64 = parts.next().ok_or(JwtError::Malformed)?;
    let sig_b64 = parts.next().ok_or(JwtError::Malformed)?;
    if parts.next().is_some() {
        return Err(JwtError::Malformed);
    }
    let header_bytes = b64url_decode(header_b64)?;
    let payload_bytes = b64url_decode(payload_b64)?;
    let sig_bytes = b64url_decode(sig_b64)?;
    let header: Header = serde_json::from_slice(&header_bytes).map_err(|_| JwtError::Json)?;
    if header.alg != "EdDSA" || header.typ.as_deref().is_some_and(|t| t != "JWT") {
        return Err(JwtError::UnsupportedAlg);
    }
    let signing_input = format!("{header_b64}.{payload_b64}");
    let arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| JwtError::BadSignature)?;
    let signature = Signature::from_bytes(&arr);
    pubkey
        .verify(signing_input.as_bytes(), &signature)
        .map_err(|_| JwtError::BadSignature)?;
    let claims: Claims = serde_json::from_slice(&payload_bytes).map_err(|_| JwtError::Json)?;
    // Allow ±60s clock skew on iat; exp is a hard wall.
    if claims.iat > now_unix + 60 || claims.exp <= now_unix {
        return Err(JwtError::Expired);
    }
    Ok(claims)
}

fn load_pubkey() -> Result<VerifyingKey, JwtError> {
    if let Ok(raw) = std::env::var("ARGO_INBOX_WS_JWT_PUBKEY") {
        if !raw.is_empty() {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(raw.as_bytes())
                .map_err(|_| JwtError::NoPubkey)?;
            let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| JwtError::NoPubkey)?;
            return VerifyingKey::from_bytes(&arr).map_err(|_| JwtError::NoPubkey);
        }
    }
    if let Ok(pem) = std::env::var("ARGO_INBOX_WS_JWT_PUBKEY_PEM") {
        if !pem.is_empty() {
            return parse_pem(&pem);
        }
    }
    Err(JwtError::NoPubkey)
}

fn parse_pem(pem: &str) -> Result<VerifyingKey, JwtError> {
    let mut body = String::new();
    let mut inside = false;
    for line in pem.lines() {
        let line = line.trim();
        if line == PUBKEY_HEADER {
            inside = true;
            continue;
        }
        if line == PUBKEY_FOOTER {
            break;
        }
        if inside && !line.is_empty() {
            body.push_str(line);
        }
    }
    if body.is_empty() {
        return Err(JwtError::NoPubkey);
    }
    let raw = base64::engine::general_purpose::STANDARD
        .decode(body.as_bytes())
        .map_err(|_| JwtError::NoPubkey)?;
    let arr: [u8; 32] = raw.as_slice().try_into().map_err(|_| JwtError::NoPubkey)?;
    VerifyingKey::from_bytes(&arr).map_err(|_| JwtError::NoPubkey)
}

fn b64url_decode(s: &str) -> Result<Vec<u8>, JwtError> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s.as_bytes())
        .map_err(|_| JwtError::Base64)
}

/// Test-only helper: sign a JWT with the provided Ed25519 signing key.
/// Lives in production code (rather than `#[cfg(test)]`) so the
/// neighboring `ws_inbox_events_tests.rs` and integration scaffolding
/// can mint tokens too, but the function is not part of the public
/// gateway API.
#[cfg(test)]
pub(super) fn sign_jwt_for_test(
    signing_key: &ed25519_dalek::SigningKey,
    claims: &Claims,
) -> String {
    use ed25519_dalek::Signer;
    let header_json = r#"{"alg":"EdDSA","typ":"JWT"}"#;
    let payload_json = serde_json::to_string(claims).unwrap_or_default();
    let h = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header_json.as_bytes());
    let p = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
    let signing_input = format!("{h}.{p}");
    let sig = signing_key.sign(signing_input.as_bytes());
    let s = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes());
    format!("{h}.{p}.{s}")
}

#[cfg(test)]
#[path = "jwt_tests.rs"]
mod tests;
