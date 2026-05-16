//! Conservative PII scrubbing for opt-in error reporting.
//!
//! The patterns intentionally over-redact. Error payloads are for
//! triage, not source-of-truth audit, so preserving privacy beats
//! preserving exact user input.

use hmac::{Hmac, Mac};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

type HmacSha256 = Hmac<Sha256>;

/// Redact email addresses, phone-like strings, JWTs, and provider
/// API keys from `input`.
#[must_use]
pub fn scrub_pii(input: &str) -> String {
    let mut out = input.to_string();
    for (pattern, replacement) in patterns() {
        out = pattern.replace_all(&out, *replacement).into_owned();
    }
    out
}

/// Stable tenant pseudonym for exported error events.
///
/// The raw tenant id never leaves the process. The returned value is
/// `tenant-<16 lowercase hex chars>` derived from HMAC-SHA256 using
/// the operator-provided salt.
#[must_use]
pub fn pseudonymize_tenant(tenant_id: &str, salt: &str) -> String {
    match HmacSha256::new_from_slice(salt.as_bytes()) {
        Ok(mut mac) => {
            mac.update(tenant_id.as_bytes());
            let digest = mac.finalize().into_bytes();
            format!("tenant-{}", hex::encode(&digest[..8]))
        }
        Err(_) => {
            let mut h = Sha256::new();
            h.update(salt.as_bytes());
            h.update(tenant_id.as_bytes());
            let digest = h.finalize();
            format!("tenant-{}", hex::encode(&digest[..8]))
        }
    }
}

fn patterns() -> &'static [(Regex, &'static str)] {
    static CELL: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    CELL.get_or_init(|| {
        [
            (
                r"(?i)[A-Z0-9._%+\-]+@[A-Z0-9.\-]+\.[A-Z]{2,}",
                "<redacted:email>",
            ),
            (
                r"eyJ[A-Za-z0-9_-]{20,}(?:\.[A-Za-z0-9_-]+){0,2}",
                "<redacted:jwt>",
            ),
            (r"sk-[A-Za-z0-9]{20,}", "<redacted:provider-key>"),
            (r"\+?[0-9][0-9 ()\-\.]{7,}[0-9]", "<redacted:phone>"),
        ]
        .into_iter()
        .filter_map(|(p, r)| Regex::new(p).ok().map(|re| (re, r)))
        .collect()
    })
}
