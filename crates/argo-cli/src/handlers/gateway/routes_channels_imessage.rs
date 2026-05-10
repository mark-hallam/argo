//! Blue Bubbles iMessage approval webhook route.
//!
//! Sibling of [`super::routes_channels`]. Owns iMessage signature
//! verification (HMAC-SHA256 hex over the raw body) and reply-text
//! → approval token decode. Shared settle / HMAC helpers live in
//! the parent file. The brief defaults to Sendblue, but the in-repo
//! channel ([`argo_channels::imessage::IMessageChannel`]) is wired
//! to Blue Bubbles, so this handler matches Blue Bubbles too;
//! switching providers is a follow-up brief.

use super::routes_channels::{
    bad_request, constant_time_eq, header_value, hmac_hex, hmac_secret, server_error, settle,
    unauthorized, verify_token,
};
use serde_json::Value;

const SECRET_ENV: &str = "ARGO_IMESSAGE_WEBHOOK_SECRET";
const SIGNATURE_HEADER: &str = "X-BlueBubbles-Signature";

pub(super) fn is_imessage_webhook(req: &str) -> bool {
    req.starts_with("POST /channels/imessage/webhook ")
        || req.starts_with("POST /channels/imessage/webhook?")
}

pub(super) async fn imessage_webhook_response(req: &str) -> (&'static str, String) {
    let Some(webhook_secret) = configured_secret() else {
        return secret_not_configured();
    };
    let approval_secret = match hmac_secret() {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let body = super::body_from_request(req);
    let Some(sig) = header_value(req, SIGNATURE_HEADER) else {
        return unauthorized_empty();
    };
    if !verify_imessage_signature(body.as_bytes(), sig, &webhook_secret) {
        return unauthorized_empty();
    }
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse iMessage payload: {e}")),
    };
    let Some(text) = imessage_text(&parsed) else {
        return bad_request("missing iMessage text");
    };
    let Some(token) = extract_approval_token(&text) else {
        return bad_request("missing approval token in iMessage reply");
    };
    let claims = match verify_token(&token, &approval_secret) {
        Ok(c) => c,
        Err(e) => return unauthorized(&e),
    };
    let actor = imessage_actor(&parsed);
    match settle(&claims, &actor, Some(&text), "imessage").await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

fn configured_secret() -> Option<String> {
    argo_config::cred::get(SECRET_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn verify_imessage_signature(body: &[u8], sig: &str, secret: &str) -> bool {
    let expected = hmac_hex(body, secret.as_bytes());
    constant_time_eq(expected.as_bytes(), sig.trim().as_bytes())
}

fn imessage_text(v: &Value) -> Option<String> {
    let msg = v.get("message").unwrap_or(v);
    string_at(msg, &["text", "message", "body"])
        .or_else(|| string_at(v, &["text", "message", "body"]))
}

fn imessage_actor(v: &Value) -> String {
    let msg = v.get("message").unwrap_or(v);
    string_at(msg, &["sender", "handle", "address", "from"])
        .or_else(|| string_at(v, &["sender", "handle", "from"]))
        .unwrap_or_else(|| "imessage-operator".to_string())
}

fn string_at(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::to_owned)
}

fn extract_approval_token(text: &str) -> Option<String> {
    for marker in ["approval_token=", "token="] {
        if let Some((_, rest)) = text.split_once(marker) {
            let token = clean_token(rest);
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    text.split_whitespace()
        .map(clean_token)
        .find(|t| t.split('.').count() == 4)
}

fn clean_token(raw: &str) -> String {
    raw.split_whitespace()
        .next()
        .unwrap_or(raw)
        .trim_matches(|c: char| matches!(c, '"' | '\'' | ',' | ')' | ']'))
        .to_string()
}

fn unauthorized_empty() -> (&'static str, String) {
    ("401 Unauthorized", String::new())
}

fn secret_not_configured() -> (&'static str, String) {
    (
        "503 Service Unavailable",
        "{\"error\":\"imessage webhook secret not configured\"}".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn predicate_recognises_imessage_webhook() {
        let req = "POST /channels/imessage/webhook HTTP/1.1\r\nHost: localhost\r\n\r\n{}";
        assert!(is_imessage_webhook(req));
        let with_query = "POST /channels/imessage/webhook?op=u-1 HTTP/1.1\r\n\r\n{}";
        assert!(is_imessage_webhook(with_query));
    }

    #[test]
    fn predicate_rejects_slack_webhook_path() {
        let slack = "POST /channels/slack/webhook HTTP/1.1\r\n\r\npayload=%7B%7D";
        assert!(!is_imessage_webhook(slack));
        let unrelated = "GET /health HTTP/1.1\r\n\r\n";
        assert!(!is_imessage_webhook(unrelated));
    }

    #[test]
    fn signature_valid_passes() {
        let body = br#"{"message":{"text":"YES approval_token=act.approve.999.sig"}}"#;
        let secret = "webhook-secret";
        let sig = hmac_hex(body, secret.as_bytes());
        assert!(verify_imessage_signature(body, &sig, secret));
    }

    #[test]
    fn signature_invalid_rejects() {
        let body = br#"{"message":{"text":"YES approval_token=act.approve.999.sig"}}"#;
        let secret = "webhook-secret";
        let good = hmac_hex(body, secret.as_bytes());
        assert!(!verify_imessage_signature(body, &good, "wrong-secret"));
        let tampered = br#"{"message":{"text":"NO approval_token=act.reject.999.sig"}}"#;
        assert!(!verify_imessage_signature(tampered, &good, secret));
    }

    #[test]
    fn payload_decoded_routes_to_approval_reply() {
        // The dispatcher arm extracts the operator's phone handle as
        // actor and the approval token from the reply text. These are
        // the two inputs `settle` consumes downstream.
        let body = serde_json::json!({
            "message": {
                "text": "yes approval_token=act.approve.42.sig thanks",
                "handle": "+15551234567"
            }
        });
        let text = imessage_text(&body).expect("text");
        assert_eq!(text, "yes approval_token=act.approve.42.sig thanks");
        assert_eq!(imessage_actor(&body), "+15551234567");
        assert_eq!(
            extract_approval_token(&text).as_deref(),
            Some("act.approve.42.sig")
        );
    }

    #[tokio::test]
    async fn webhook_secret_lifecycle() {
        // Sequenced in one test: env vars are process-global, parallel
        // tests would race on `SECRET_ENV`.
        std::env::remove_var(SECRET_ENV);
        let req = "POST /channels/imessage/webhook HTTP/1.1\r\nHost: localhost\r\n\r\n{}";
        let (status, body) = imessage_webhook_response(req).await;
        assert_eq!(status, "503 Service Unavailable");
        assert!(body.contains("imessage webhook secret not configured"));
        std::env::set_var(SECRET_ENV, "webhook-secret");
        std::env::set_var("ARGO_CHANNEL_HMAC_SECRET", "approval-secret");
        let (status, body) = imessage_webhook_response(req).await;
        assert_eq!(status, "401 Unauthorized");
        assert!(body.is_empty());
        std::env::remove_var(SECRET_ENV);
    }
}
