//! Mailgun webhook signature verification.
//!
//! Mailgun signs every webhook delivery with
//! `HMAC-SHA256(timestamp + token, api_key)`, hex-encoded. The receiver
//! must constant-time compare the resulting signature against the one
//! Mailgun supplied AND reject deliveries whose timestamp is older than a
//! short replay window (default 15 minutes per Mailgun's recommendation).

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Default Mailgun-recommended replay-protection window.
pub const DEFAULT_WEBHOOK_SKEW_SECONDS: u64 = 15 * 60;

/// Verify a Mailgun webhook signature using the default 15-minute replay
/// window. Returns `true` iff the HMAC matches in constant time AND the
/// timestamp is within the window of the system clock.
///
/// Returns `false` rather than an error so the verifier composes cleanly
/// with `if !verify_webhook(...) { reject }` patterns; a Mailgun forgery
/// is not a recoverable condition that benefits from a typed error.
#[must_use]
pub fn verify_webhook(api_key: &str, timestamp: &str, token: &str, signature: &str) -> bool {
    verify_webhook_with_skew(
        api_key,
        timestamp,
        token,
        signature,
        DEFAULT_WEBHOOK_SKEW_SECONDS,
        current_unix_seconds(),
    )
}

/// Verify a Mailgun webhook signature with an explicit skew window and
/// reference time. Splitting `now` out keeps the function deterministic for
/// tests; production callers should use [`verify_webhook`].
#[must_use]
pub fn verify_webhook_with_skew(
    api_key: &str,
    timestamp: &str,
    token: &str,
    signature: &str,
    max_skew_seconds: u64,
    now_unix_seconds: u64,
) -> bool {
    let Ok(ts) = timestamp.parse::<u64>() else {
        return false;
    };
    let skew = now_unix_seconds.abs_diff(ts);
    if skew > max_skew_seconds {
        return false;
    }
    let Some(expected) = compute_signature(api_key, timestamp, token) else {
        return false;
    };
    constant_time_eq(expected.as_bytes(), signature.as_bytes())
}

/// Compute the hex-encoded `HMAC-SHA256(timestamp + token, api_key)`
/// expected by Mailgun. Returns `None` only on the (theoretically
/// impossible for HMAC) key-init failure.
#[must_use]
pub fn compute_signature(api_key: &str, timestamp: &str, token: &str) -> Option<String> {
    let mut mac = HmacSha256::new_from_slice(api_key.as_bytes()).ok()?;
    mac.update(timestamp.as_bytes());
    mac.update(token.as_bytes());
    Some(hex::encode(mac.finalize().into_bytes()))
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

fn current_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
