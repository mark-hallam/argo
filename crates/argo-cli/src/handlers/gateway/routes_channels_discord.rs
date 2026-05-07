//! Discord approval interaction decision route.
//!
//! Sibling of [`super::routes_channels`]. Owns Discord interaction
//! payload parsing and delegates signed-token verification plus settle
//! behavior to the shared channel helpers.

use super::routes_channels::{
    bad_request, hmac_secret, percent_decode, server_error, settle, verify_token,
};
use serde_json::Value;

pub(super) fn is_discord_decision(req: &str) -> bool {
    req.starts_with("POST /channels/discord/decision ")
        || req.starts_with("POST /channels/discord/decision?")
}

pub(super) async fn discord_decision_response(req: &str) -> (&'static str, String) {
    let secret = match hmac_secret() {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let body = super::body_from_request(req);
    let parsed = match discord_payload(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&e),
    };
    let Some(token) = discord_action_token(&parsed).or_else(|| query_token(req)) else {
        return bad_request("missing Discord approval token");
    };
    let claims = match verify_token(&token, &secret) {
        Ok(c) => c,
        Err(e) => return bad_request(&e),
    };
    match settle(
        &claims,
        &discord_actor(&parsed),
        discord_note(&parsed).as_deref(),
        "discord",
    )
    .await
    {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

fn discord_payload(body: &str) -> Result<Value, String> {
    if body.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(body).map_err(|e| format!("parse Discord payload: {e}"))
}

fn discord_action_token(v: &Value) -> Option<String> {
    let raw = v
        .pointer("/data/custom_id")
        .or_else(|| v.get("custom_id"))
        .or_else(|| v.get("token"))?
        .as_str()?;
    Some(strip_custom_id_prefix(raw).to_string())
}

fn query_token(req: &str) -> Option<String> {
    let target = req.lines().next()?.split_whitespace().nth(1)?;
    let query = target.split_once('?')?.1;
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (percent_decode(k).as_deref() == Some("token"))
            .then(|| percent_decode(v))
            .flatten()
    })
}

fn strip_custom_id_prefix(raw: &str) -> &str {
    raw.strip_prefix("argo_approval:").unwrap_or(raw)
}

fn discord_actor(v: &Value) -> String {
    v.pointer("/member/user/id")
        .or_else(|| v.pointer("/user/id"))
        .or_else(|| v.pointer("/member/nick"))
        .and_then(Value::as_str)
        .unwrap_or("discord-operator")
        .to_string()
}

fn discord_note(v: &Value) -> Option<String> {
    v.pointer("/data/custom_id")
        .and_then(Value::as_str)
        .map(|_| "discord_component".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_payload_extracts_actor_and_token() {
        let parsed = discord_payload(
            r#"{"member":{"user":{"id":"U1"}},"data":{"custom_id":"argo_approval:act.approve.999.sig"}}"#,
        )
        .unwrap();
        assert_eq!(discord_actor(&parsed), "U1");
        assert_eq!(
            discord_action_token(&parsed),
            Some("act.approve.999.sig".into())
        );
    }

    #[test]
    fn query_token_extracts_percent_decoded_token() {
        let req = "POST /channels/discord/decision?token=act.approve.999.%73ig HTTP/1.1\r\n\r\n";
        assert_eq!(query_token(req), Some("act.approve.999.sig".into()));
    }

    #[tokio::test]
    async fn tampered_token_returns_bad_request() {
        std::env::set_var("ARGO_CHANNEL_HMAC_SECRET", "secret");
        let exp = chrono::Utc::now().timestamp() + 60;
        let req = format!(
            "POST /channels/discord/decision?token=act.approve.{exp}.bad HTTP/1.1\r\nHost: localhost\r\n\r\n"
        );
        let (status, body) = discord_decision_response(&req).await;
        assert_eq!(status, "400 Bad Request");
        assert!(body.contains("signature mismatch"));
    }
}
