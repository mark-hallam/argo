//! `GET /api/v1/audit/<actionId>/screenshots/<idx>?exp=…&sig=…` —
//! signed-URL screenshot endpoint (Stream J15). HMAC-SHA256 over
//! `actionId|idx|exp` with `ARGO_AUDIT_SCREENSHOT_SECRET` (dev
//! fallback when unset). 401 on bad/expired sig; 404 on missing
//! row or out-of-range index; 200 with the PNG payload otherwise.
//!
//! v1: PNGs come from `payload_json.screenshots[idx].png_b64`
//! (base64). `mission-enterprise-screenshot-retention` later swaps
//! this for object storage with per-tenant retention.
//!
//! Response shape: the gateway listener hard-codes
//! `Content-Type: application/json` and uses `String::len()` for
//! `Content-Length`, so raw PNG bytes would mis-advertise the type
//! and corrupt the byte count. The body is therefore JSON-wrapped:
//! `{ "kind":"png", "b64":"<base64>", "size_bytes":N }`. A future
//! brief that widens the listener can drop the wrapper.

use super::store_unavailable;
use argo_store::SessionStore;
use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Lifetime of a freshly-signed screenshot URL, in seconds. Brief J15
/// fixes this at 10 minutes (a leaked URL stays short-lived but the
/// operator can still click through without racing the clock).
pub(super) const SCREENSHOT_URL_TTL_SECS: i64 = 600;

const SECRET_ENV: &str = "ARGO_AUDIT_SCREENSHOT_SECRET";
const DEV_FALLBACK_SECRET: &[u8] = b"argo-dev-screenshot-secret-do-not-use-in-prod";
const SHA256_BLOCK: usize = 64;

/// `GET /api/v1/audit/<actionId>/screenshots/<idx>?exp=…&sig=…`.
pub(super) async fn screenshot_response(
    action_id: &str,
    idx_raw: &str,
    query: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let idx: usize = match idx_raw.parse() {
        Ok(n) => n,
        Err(_) => return bad_request("screenshot idx must be a non-negative integer"),
    };
    let Some(exp) = query_kv(query, "exp").and_then(|v| v.parse::<i64>().ok()) else {
        return unauthorized("missing or invalid exp");
    };
    let Some(sig) = query_kv(query, "sig") else {
        return unauthorized("missing sig");
    };
    if exp <= chrono::Utc::now().timestamp() {
        return unauthorized("signature expired");
    }
    if !verify_signature(action_id, idx, exp, &sig) {
        return unauthorized("signature mismatch");
    }
    let id_bytes = match hex::decode(action_id) {
        Ok(b) if b.len() == 16 => b,
        _ => return not_found("audit row not found"),
    };
    let payload = match fetch_payload(store, &id_bytes).await {
        Ok(Some(p)) => p,
        Ok(None) => return not_found("audit row not found"),
        Err(e) => return server_error(&format!("audit detail fetch: {e}")),
    };
    let parsed: Value = serde_json::from_str(&payload).unwrap_or(Value::Null);
    let Some(shot) = parsed
        .get("screenshots")
        .and_then(Value::as_array)
        .and_then(|arr| arr.get(idx))
    else {
        return not_found("screenshot index out of range");
    };
    let Some(b64) = shot.get("png_b64").and_then(Value::as_str) else {
        return not_found("screenshot bytes not stored");
    };
    let png = match general_purpose::STANDARD.decode(b64) {
        Ok(b) => b,
        Err(e) => return server_error(&format!("screenshot decode: {e}")),
    };
    let body = serde_json::json!({
        "kind": "png",
        "b64": general_purpose::STANDARD.encode(&png),
        "size_bytes": png.len(),
    });
    ("200 OK", body.to_string())
}

async fn fetch_payload(
    store: &Arc<SessionStore>,
    id_bytes: &[u8],
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>(
        "SELECT payload_json FROM audit_record WHERE id = ? LIMIT 1",
    )
    .bind(id_bytes)
    .fetch_optional(store.pool())
    .await
}

/// Build the public screenshot URL for `(action_id, idx)`. The
/// caller picks `exp` (typically `now + SCREENSHOT_URL_TTL_SECS`)
/// so a batch of URLs can share one expiry.
#[must_use]
pub(super) fn sign_screenshot_url(action_id: &str, idx: usize, exp: i64) -> String {
    let sig = compute_signature(action_id, idx, exp);
    format!("/api/v1/audit/{action_id}/screenshots/{idx}?exp={exp}&sig={sig}")
}

fn verify_signature(action_id: &str, idx: usize, exp: i64, presented: &str) -> bool {
    let expected = compute_signature(action_id, idx, exp);
    constant_time_eq(expected.as_bytes(), presented.as_bytes())
}

fn compute_signature(action_id: &str, idx: usize, exp: i64) -> String {
    let secret = secret_bytes();
    let msg = format!("{action_id}|{idx}|{exp}");
    hex::encode(hmac_sha256(&secret, msg.as_bytes()))
}

fn secret_bytes() -> Vec<u8> {
    argo_config::cred::get(SECRET_ENV)
        .ok()
        .filter(|s| !s.is_empty())
        .map_or_else(|| DEV_FALLBACK_SECRET.to_vec(), String::into_bytes)
}

/// HMAC-SHA256 per RFC 2104. Inlined to avoid taking `hmac` as a new
/// direct argo-cli dep; `sha2` is already a direct dep.
fn hmac_sha256(secret: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut key = [0u8; SHA256_BLOCK];
    if secret.len() > SHA256_BLOCK {
        let h: [u8; 32] = Sha256::digest(secret).into();
        key[..32].copy_from_slice(&h);
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut i_pad = [0x36_u8; SHA256_BLOCK];
    let mut o_pad = [0x5c_u8; SHA256_BLOCK];
    for i in 0..SHA256_BLOCK { i_pad[i] ^= key[i]; o_pad[i] ^= key[i]; }
    let mut inner = Sha256::new();
    inner.update(i_pad);
    inner.update(msg);
    let inner_hash = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(o_pad);
    outer.update(inner_hash);
    outer.finalize().into()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0_u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn query_kv(qs: &str, key: &str) -> Option<String> {
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| url_decode(v))
    })
}

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '+' => out.push(' '),
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(b) = u8::from_str_radix(&hex, 16) {
                    out.push(b as char);
                } else {
                    out.push('%');
                    out.push_str(&hex);
                }
            }
            other => out.push(other),
        }
    }
    out
}

fn unauthorized(msg: &str) -> (&'static str, String) { err("401 Unauthorized", "unauthorized", msg) }
fn not_found(msg: &str) -> (&'static str, String) { err("404 Not Found", "not_found", msg) }
fn bad_request(msg: &str) -> (&'static str, String) { err("400 Bad Request", "bad_request", msg) }
fn server_error(msg: &str) -> (&'static str, String) { err("500 Internal Server Error", "internal_error", msg) }

fn err(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (status, serde_json::json!({"error": {"code": code, "message": message}}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sha256_matches_known_vector() {
        // RFC 4231 test case 1.
        let key = [0x0b_u8; 20];
        let msg = b"Hi There";
        let expected = hex::decode(
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
        )
        .unwrap();
        assert_eq!(hmac_sha256(&key, msg).to_vec(), expected);
    }

    #[test]
    fn sign_url_round_trips() {
        let url = sign_screenshot_url("00112233445566778899aabbccddeeff", 3, 1_700_000_000);
        assert!(url.contains("/api/v1/audit/00112233445566778899aabbccddeeff/screenshots/3"));
        assert!(url.contains("exp=1700000000"));
        assert!(url.contains("sig="));
    }

    #[test]
    fn verify_signature_constant_time_rejects_mismatch() {
        let exp = 1_700_000_000;
        let good = compute_signature("aa", 0, exp);
        assert!(verify_signature("aa", 0, exp, &good));
        let bad = format!("{good}0");
        assert!(!verify_signature("aa", 0, exp, &bad));
    }

    #[test]
    fn url_decode_handles_plus_and_pct() {
        assert_eq!(url_decode("a+b"), "a b");
        assert_eq!(url_decode("%2F"), "/");
        assert_eq!(url_decode("%ZZ"), "%ZZ");
    }
}
