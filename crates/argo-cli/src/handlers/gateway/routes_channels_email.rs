//! Email (Postmark) approval webhook decision route.
//!
//! Sibling of [`super::routes_channels`]. Owns the Postmark-side
//! token extraction (HTTP query, JSON body field, or text-body
//! pattern) and the actor/note shape Postmark posts back. Shared
//! settle/HMAC/helpers live in the parent file.

use super::routes_channels::{
    bad_request, clean_token, constant_time_eq, header_value, hmac_hex, hmac_secret,
    percent_decode, server_error, settle, unauthorized, verify_token,
};
use serde_json::Value;

pub(super) async fn handle_email_webhook(req: &str) -> (&'static str, String) {
    let secret = match hmac_secret() {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let body = super::body_from_request(req);
    let Some(sig) = header_value(req, "X-Postmark-Signature") else {
        return unauthorized("missing X-Postmark-Signature");
    };
    if !constant_time_eq(
        hmac_hex(body.as_bytes(), secret.as_bytes()).as_bytes(),
        sig.as_bytes(),
    ) {
        return unauthorized("invalid X-Postmark-Signature");
    }
    let parsed = match serde_json::from_str::<Value>(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse JSON body: {e}")),
    };
    let Some(token) = extract_token(req, &parsed) else {
        return bad_request("missing approval token");
    };
    let claims = match verify_token(&token, &secret) {
        Ok(c) => c,
        Err(e) => return unauthorized(&e),
    };
    match settle(
        &claims,
        &actor_from_payload(&parsed),
        note_from_payload(&parsed).as_deref(),
        "email",
    )
    .await
    {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

pub(super) fn extract_token(req: &str, payload: &Value) -> Option<String> {
    request_query_token(req)
        .or_else(|| json_token(payload))
        .or_else(|| text_token(payload))
}

fn request_query_token(req: &str) -> Option<String> {
    req.strip_prefix("POST ")?
        .split_once(' ')?
        .0
        .split_once('?')?
        .1
        .split('&')
        .find_map(|kv| kv.strip_prefix("token=").map(clean_token))
}

fn json_token(payload: &Value) -> Option<String> {
    ["token", "Token", "approval_token", "ApprovalToken"]
        .iter()
        .find_map(|k| payload.get(*k).and_then(Value::as_str).map(clean_token))
}

fn text_token(payload: &Value) -> Option<String> {
    ["StrippedTextReply", "TextBody", "HtmlBody", "Subject"]
        .iter()
        .filter_map(|k| payload.get(*k).and_then(Value::as_str))
        .find_map(find_token_in_text)
}

fn find_token_in_text(s: &str) -> Option<String> {
    ["approval_token=", "token="]
        .iter()
        .find_map(|m| s.split_once(m).map(|(_, rest)| clean_token(rest)))
}

pub(super) fn token_from_url(url: &str) -> Option<String> {
    url.split_once('?')?
        .1
        .split('&')
        .find_map(|kv| kv.strip_prefix("token=").and_then(percent_decode))
}

fn actor_from_payload(v: &Value) -> String {
    v.get("From")
        .and_then(Value::as_str)
        .or_else(|| v.get("FromFull")?.get("Email").and_then(Value::as_str))
        .unwrap_or("email-operator")
        .to_string()
}

fn note_from_payload(v: &Value) -> Option<String> {
    ["StrippedTextReply", "TextBody"]
        .iter()
        .find_map(|k| v.get(*k).and_then(Value::as_str))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_token_finds_token_in_email_payload() {
        let payload = json!({"TextBody":"ok\napproval_token=act.approve.999.sig\nthanks"});
        assert_eq!(
            extract_token(
                "POST /channels/email/webhook HTTP/1.1\r\n\r\n{}",
                &payload
            ),
            Some("act.approve.999.sig".into())
        );
    }
}
