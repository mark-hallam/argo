use base64::Engine;
use jsonwebtoken::{EncodingKey, Header};
use rand_core::{OsRng, RngCore};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

const JWT_TTL_SECS: i64 = 7 * 24 * 60 * 60;

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
    iat: i64,
    exp: i64,
}

pub(crate) fn mint_jwt(
    user_id: &str,
    email: &str,
    tenant_id: Option<&str>,
    now: i64,
) -> Result<String, String> {
    let key = EncodingKey::from_secret(&jwt_secret());
    jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &Claims {
            sub: user_id,
            email,
            tnt: tenant_id,
            iat: now,
            exp: now + JWT_TTL_SECS,
        },
        &key,
    )
    .map_err(|e| format!("jwt encode: {e}"))
}

/// Production sets `SKYCOT_AUTH_JWT_SECRET`; dev runs mint an
/// ephemeral per-process secret so the loop works without explicit
/// config.
fn jwt_secret() -> Vec<u8> {
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
