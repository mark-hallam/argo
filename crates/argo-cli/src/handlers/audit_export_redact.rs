//! GDPR-style email redaction for audit export payloads.
//!
//! Lifted out of `audit_export.rs` so that file stays under the
//! 500-LOC HARD RULE 6 ceiling. Pure data transform — no I/O, no
//! async — invoked by `audit_export::run` when the
//! `--gdpr-redact-emails` flag is set.
use sha2::{Digest, Sha256};

pub(super) fn redact_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => *s = redact_emails_in_str(s),
        serde_json::Value::Array(items) => {
            for item in items {
                redact_value(item);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                redact_value(item);
            }
        }
        _ => {}
    }
}

fn redact_emails_in_str(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut cursor = 0_usize;
    for (idx, byte) in bytes.iter().enumerate() {
        if *byte != b'@' {
            continue;
        }
        let start = scan_left(bytes, idx);
        let end = scan_right(bytes, idx + 1);
        if start == idx || end == idx + 1 {
            continue;
        }
        let candidate = &input[start..end];
        if !candidate
            .rsplit_once('.')
            .is_some_and(|(_, tld)| tld.as_bytes().iter().all(u8::is_ascii_alphanumeric))
        {
            continue;
        }
        if start < cursor {
            continue;
        }
        out.push_str(&input[cursor..start]);
        out.push_str(&redacted_email(candidate));
        cursor = end;
    }
    out.push_str(&input[cursor..]);
    out
}

fn scan_left(bytes: &[u8], at: usize) -> usize {
    let mut pos = at;
    while pos > 0 && is_email_local(bytes[pos - 1]) {
        pos -= 1;
    }
    pos
}

fn scan_right(bytes: &[u8], mut pos: usize) -> usize {
    while pos < bytes.len() && is_email_domain(bytes[pos]) {
        pos += 1;
    }
    pos
}

fn is_email_local(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'%' | b'+' | b'-')
}

fn is_email_domain(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-')
}

fn redacted_email(raw: &str) -> String {
    let mut h = Sha256::new();
    h.update(raw.to_ascii_lowercase().as_bytes());
    let hex = hex::encode(h.finalize());
    format!("redacted-{}@example.com", &hex[..12])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_email_consistently_in_nested_json() {
        let mut value = serde_json::json!({"email":"ALICE@example.com","note":"cc alice@example.com and bob@example.org","items":["alice@example.com"]});
        redact_value(&mut value);
        let first = value["email"].as_str().unwrap().to_string();
        assert!(first.starts_with("redacted-"));
        assert_eq!(value["items"][0], first);
        assert!(value["note"].as_str().unwrap().contains(&first));
        assert!(!value.to_string().contains("bob@example.org"));
    }

    #[test]
    fn redactor_leaves_non_email_strings_alone() {
        assert_eq!(
            redact_emails_in_str("not-an-email @ local"),
            "not-an-email @ local"
        );
    }
}
