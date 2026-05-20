use base64::Engine;
use jsonwebtoken::{EncodingKey, Header};
use rand_core::{OsRng, RngCore};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub(crate) const JWT_TTL_SECS: i64 = 7 * 24 * 60 * 60;

pub(crate) fn mint_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn sha256_hex(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}

#[derive(Debug, Serialize)]
struct Claims<'a> {
    sub: &'a str,
    email: &'a str,
    tnt: Option<&'a str>,
    /// Role the user holds in `tnt` — `'owner' | 'admin' | 'user' | 'guest'`.
    /// `None` when the user has no membership row yet (pre-onboarding).
    /// Gateway capability checks default `None` to the most-restrictive
    /// role (`guest`); the role-gate never trusts a missing claim.
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'a str>,
    /// JWT ID — uuid v7 minted per JWT. Doubles as the `__Host-csrf`
    /// cookie value (double-submit CSRF pattern). Sibling brief
    /// `mission-revocation-with-verify-middleware-v1x` keys the
    /// `revoked_tokens` table on this claim.
    jti: &'a str,
    iat: i64,
    exp: i64,
}

/// Mint a new HS256 session JWT. Returns `(jwt, jti)` so the caller
/// can plumb the jti into the `__Host-csrf` cookie value as well as
/// the JWT itself.
pub(crate) fn mint_jwt(
    user_id: &str,
    email: &str,
    tenant_id: Option<&str>,
    role: Option<&str>,
    now: i64,
) -> Result<(String, String), String> {
    let key = EncodingKey::from_secret(&jwt_secret());
    let jti = uuid::Uuid::now_v7().to_string();
    let token = jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &Claims {
            sub: user_id,
            email,
            tnt: tenant_id,
            role,
            jti: &jti,
            iat: now,
            exp: now + JWT_TTL_SECS,
        },
        &key,
    )
    .map_err(|e| format!("jwt encode: {e}"))?;
    Ok((token, jti))
}

/// Production sets `SKYCOT_AUTH_JWT_SECRET`; dev runs mint an
/// ephemeral per-process secret so the loop works without explicit
/// config. The gateway role-gate reads the same secret via
/// [`super::session_jwt`] so verify-side checks share one source of
/// truth.
pub(crate) fn jwt_secret() -> Vec<u8> {
    if let Ok(s) = std::env::var("SKYCOT_AUTH_JWT_SECRET") {
        if !s.is_empty() {
            return s.into_bytes();
        }
    }
    static EPHEMERAL: OnceLock<Vec<u8>> = OnceLock::new();
    EPHEMERAL
        .get_or_init(|| {
            let mut buf = vec![0u8; 32];
            OsRng.fill_bytes(&mut buf);
            tracing::warn!(
                target: "gateway::auth_magic_link",
                "SKYCOT_AUTH_JWT_SECRET unset — ephemeral key minted",
            );
            buf
        })
        .clone()
}
