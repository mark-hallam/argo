//! Webhook secret-token verification for Telegram inbound deliveries.
//!
//! At `setWebhook` time, operators register a `secret_token` that Telegram
//! echoes back on every webhook POST via the
//! `X-Telegram-Bot-Api-Secret-Token` header. Compare in constant time to
//! avoid leaking the secret length through timing.

/// HTTP header Telegram uses to authenticate webhook deliveries.
pub const TELEGRAM_WEBHOOK_SECRET_HEADER: &str = "X-Telegram-Bot-Api-Secret-Token";

/// Constant-time check that the inbound `X-Telegram-Bot-Api-Secret-Token`
/// header matches the operator-configured `expected` value.
///
/// `header_value` is the raw value of the
/// `X-Telegram-Bot-Api-Secret-Token` header, or `None` if Telegram (or an
/// attacker) omitted it. Returns `false` for any mismatch, including
/// length mismatch and absent header.
#[must_use]
pub fn verify_webhook_secret(header_value: Option<&str>, expected: &str) -> bool {
    let Some(got) = header_value else {
        return false;
    };
    constant_time_eq(got.as_bytes(), expected.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_secret_returns_true() {
        assert!(verify_webhook_secret(Some("s3cret"), "s3cret"));
    }

    #[test]
    fn mismatched_secret_returns_false() {
        assert!(!verify_webhook_secret(Some("nope"), "s3cret"));
    }

    #[test]
    fn missing_header_returns_false() {
        assert!(!verify_webhook_secret(None, "s3cret"));
    }

    #[test]
    fn different_length_returns_false() {
        assert!(!verify_webhook_secret(Some("short"), "much-longer-secret"));
    }
}
