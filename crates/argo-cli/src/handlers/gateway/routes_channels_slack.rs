//! Slack approval webhook decision route.
//!
//! Sibling of [`super::routes_channels`]. Owns the Slack-side
//! signature verification (HMAC over `v0:<ts>:<body>`), the
//! `payload=…` form-body parsing, and the actor/note shape Slack
//! posts back. Shared settle/HMAC/helpers live in the parent file.

use super::routes_channels::{
    bad_request, constant_time_eq, header_value, hmac_hex, hmac_secret, percent_decode,
    server_error, settle, unauthorized, verify_token,
};
use serde_json::Value;

pub(super) fn is_slack_webhook(req: &str) -> bool {
    req.starts_with("POST /channels/slack/webhook ")
        || req.starts_with("POST /channels/slack/webhook?")
}

pub(super) async fn slack_webhook_response(req: &str) -> (&'static str, String) {
    let Ok(signing_secret) = argo_config::cred::get("SLACK_APPROVALS_SIGNING_SECRET") else {
        return server_error("missing SLACK_APPROVALS_SIGNING_SECRET");
    };
    let secret = match hmac_secret() {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let body = super::body_from_request(req);
    let Some(sig) = header_value(req, "X-Slack-Signature") else {
        return unauthorized("missing X-Slack-Signature");
    };
    let Some(ts) = header_value(req, "X-Slack-Request-Timestamp") else {
        return unauthorized("missing X-Slack-Request-Timestamp");
    };
    if !verify_slack_signature(body, ts, sig, &signing_secret) {
        return unauthorized("invalid X-Slack-Signature");
    }
    let parsed = match slack_payload(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&e),
    };
    let Some(token) = slack_action_token(&parsed) else {
        return bad_request("missing Slack action token");
    };
    let claims = match verify_token(&token, &secret) {
        Ok(c) => c,
        Err(e) => return unauthorized(&e),
    };
    match settle(
        &claims,
        &slack_actor(&parsed),
        slack_note(&parsed).as_deref(),
        "slack",
    )
    .await
    {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

fn slack_payload(body: &str) -> Result<Value, String> {
    serde_json::from_str(&form_value(body, "payload").unwrap_or_else(|| body.to_string()))
        .map_err(|e| format!("parse Slack payload: {e}"))
}

fn slack_action_token(v: &Value) -> Option<String> {
    v.get("actions")?
        .as_array()?
        .first()?
        .get("value")?
        .as_str()
        .map(str::to_string)
}

fn slack_actor(v: &Value) -> String {
    v.get("user")
        .and_then(|u| u.get("id"))
        .and_then(Value::as_str)
        .or_else(|| {
            v.get("user")?
                .get("username")
                .and_then(Value::as_str)
        })
        .unwrap_or("slack-operator")
        .to_string()
}

fn slack_note(v: &Value) -> Option<String> {
    v.get("actions")?
        .as_array()?
        .first()?
        .get("action_id")?
        .as_str()
        .map(str::to_string)
}

fn verify_slack_signature(body: &str, ts: &str, sig: &str, secret: &str) -> bool {
    let Ok(ts_num) = ts.parse::<i64>() else {
        return false;
    };
    if (chrono::Utc::now().timestamp() - ts_num).abs() > 300 {
        return false;
    }
    constant_time_eq(
        format!(
            "v0={}",
            hmac_hex(format!("v0:{ts}:{body}").as_bytes(), secret.as_bytes())
        )
        .as_bytes(),
        sig.trim().as_bytes(),
    )
}

fn form_value(body: &str, key: &str) -> Option<String> {
    body.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (percent_decode(k).as_deref() == Some(key))
            .then(|| percent_decode(v))
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slack_payload_extracts_actor_and_token() {
        let body = "payload=%7B%22user%22%3A%7B%22id%22%3A%22U1%22%7D%2C%22actions%22%3A%5B%7B%22action_id%22%3A%22approve%22%2C%22value%22%3A%22act.approve.999.sig%22%7D%5D%7D";
        let parsed = slack_payload(body).unwrap();
        assert_eq!(slack_actor(&parsed), "U1");
        assert_eq!(
            slack_action_token(&parsed),
            Some("act.approve.999.sig".into())
        );
    }
}
