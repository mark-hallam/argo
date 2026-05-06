//! RFC 2822 message construction for Gmail `raw` payloads.
use base64::prelude::{Engine, BASE64_URL_SAFE_NO_PAD};

/// Return a base64url Gmail `raw` message from either `raw` or fields.
pub fn raw_message(args: &serde_json::Value) -> Result<String, String> {
    if let Some(raw) = args.get("raw").and_then(|v| v.as_str()) {
        return Ok(BASE64_URL_SAFE_NO_PAD.encode(raw.as_bytes()));
    }
    let to = clean_header(required_str(args, "to")?, "to")?;
    let subject = clean_header(required_str(args, "subject")?, "subject")?;
    let body = required_str(args, "body_text")?;
    let mut lines = vec![
        format!("To: {to}"),
        format!("Subject: {subject}"),
        "MIME-Version: 1.0".into(),
        "Content-Type: text/plain; charset=\"UTF-8\"".into(),
        "Content-Transfer-Encoding: 8bit".into(),
    ];
    for (field, header) in [("cc", "Cc"), ("bcc", "Bcc"), ("from", "From")] {
        if let Some(value) = args.get(field).and_then(|v| v.as_str()) {
            lines.push(format!(
                "{header}: {}",
                clean_header(value.to_string(), field)?
            ));
        }
    }
    lines.push(String::new());
    lines.push(body);
    Ok(BASE64_URL_SAFE_NO_PAD.encode(lines.join("\r\n").as_bytes()))
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn clean_header(value: String, field: &str) -> Result<String, String> {
    if value.contains('\r') || value.contains('\n') {
        Err(format!("header field '{field}' contains newline"))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_message_encodes() {
        let raw = raw_message(&serde_json::json!({
            "to": "a@example.com",
            "subject": "Hello",
            "body_text": "Body"
        }))
        .expect("raw");
        assert!(!raw.contains('='));
    }

    #[test]
    fn header_newline_is_rejected() {
        let err = raw_message(&serde_json::json!({
            "to": "a@example.com\nBcc: b@example.com",
            "subject": "Hello",
            "body_text": "Body"
        }))
        .unwrap_err();
        assert!(err.contains("newline"));
    }
}
