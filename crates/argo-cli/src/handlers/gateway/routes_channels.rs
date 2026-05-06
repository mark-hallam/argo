//! Channel webhook routes.

use crate::handlers::worker_actions::{ActionRecord, ActionState};
use crate::handlers::worker_actions_transitions;
use argo_memory::{MemoryFile, MemoryStore};
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use serde_json::Value;
use sha2::{Digest, Sha256};

enum Decision {
    Approve,
    Reject,
}

struct TokenClaims {
    approval_id: String,
    decision: Decision,
}

pub(super) fn is_email_webhook(req: &str) -> bool {
    req.starts_with("POST /channels/email/webhook ")
        || req.starts_with("POST /channels/email/webhook?")
        || is_slack_webhook(req)
        || is_sms_decision(req)
}

fn is_slack_webhook(req: &str) -> bool {
    req.starts_with("POST /channels/slack/webhook ")
        || req.starts_with("POST /channels/slack/webhook?")
}

fn is_sms_decision(req: &str) -> bool {
    req.starts_with("GET /channels/sms/decision/")
        || req.starts_with("GET /channels/sms/decision?")
}

pub(super) async fn email_webhook_response(req: &str) -> (&'static str, String) {
    if is_slack_webhook(req) {
        return slack_webhook_response(req).await;
    }
    if is_sms_decision(req) {
        return sms_decision_response(req).await;
    }
    let Ok(secret) = std::env::var("ARGO_CHANNEL_HMAC_SECRET") else {
        return server_error("missing ARGO_CHANNEL_HMAC_SECRET");
    };
    if secret.is_empty() {
        return server_error("ARGO_CHANNEL_HMAC_SECRET must not be empty");
    }
    let body = super::body_from_request(req);
    let Some(sig) = header_value(req, "X-Postmark-Signature") else {
        return unauthorized("missing X-Postmark-Signature");
    };
    if !verify_body_signature(body.as_bytes(), sig, secret.as_bytes()) {
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
    let actor = actor_from_payload(&parsed);
    let note = note_from_payload(&parsed);
    match settle(&claims, &actor, note.as_deref(), "email").await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

async fn slack_webhook_response(req: &str) -> (&'static str, String) {
    let Ok(signing_secret) = std::env::var("SLACK_APPROVALS_SIGNING_SECRET") else {
        return server_error("missing SLACK_APPROVALS_SIGNING_SECRET");
    };
    let Ok(hmac_secret) = std::env::var("ARGO_CHANNEL_HMAC_SECRET") else {
        return server_error("missing ARGO_CHANNEL_HMAC_SECRET");
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
    let claims = match verify_token(&token, &hmac_secret) {
        Ok(c) => c,
        Err(e) => return unauthorized(&e),
    };
    let actor = slack_actor(&parsed);
    let note = slack_note(&parsed);
    match settle(&claims, &actor, note.as_deref(), "slack").await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

async fn sms_decision_response(req: &str) -> (&'static str, String) {
    let Ok(secret) = std::env::var("ARGO_CHANNEL_HMAC_SECRET") else {
        return server_error("missing ARGO_CHANNEL_HMAC_SECRET");
    };
    if secret.is_empty() {
        return server_error("ARGO_CHANNEL_HMAC_SECRET must not be empty");
    }
    let Some(token) = extract_sms_token(req) else {
        return bad_request("missing approval token");
    };
    let claims = match verify_token(&token, &secret) {
        Ok(c) => c,
        Err(e) => return unauthorized(&e),
    };
    match settle(&claims, "sms-operator", None, "sms").await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

async fn settle(
    claims: &TokenClaims,
    actor: &str,
    note: Option<&str>,
    source: &str,
) -> ArgoResult<String> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let located = worker_actions_transitions::find_action(&memory, ws, &claims.approval_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no approval '{}'", claims.approval_id)))?;
    if located.record.state != ActionState::Pending.as_str() {
        let by = resolved_by(&located.record);
        return Ok(serde_json::json!({
            "ok": true,
            "already_resolved": true,
            "approval_id": claims.approval_id,
            "state": located.record.state,
            "resolved_by": by,
        })
        .to_string());
    }
    if !located.record.requires_approval {
        return Ok(serde_json::json!({
            "ok": true,
            "already_resolved": true,
            "approval_id": claims.approval_id,
            "state": located.record.state,
            "resolved_by": "autonomous-safe",
        })
        .to_string());
    }
    match claims.decision {
        Decision::Approve => {
            let rec =
                worker_actions_transitions::approve(&memory, ws, &claims.approval_id, actor, note)
                    .await?;
            Ok(serde_json::json!({
                "ok": true,
                "approval_id": rec.action_id,
                "decision": "approve",
                "state": rec.state,
                "resolved_by": actor,
            })
            .to_string())
        }
        Decision::Reject => {
            let rec = reject_action(&memory, ws, &located, actor, note, source).await?;
            Ok(serde_json::json!({
                "ok": true,
                "approval_id": rec.action_id,
                "decision": "reject",
                "state": rec.state,
                "resolved_by": actor,
            })
            .to_string())
        }
    }
}

async fn reject_action(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    located: &worker_actions_transitions::LocatedAction,
    actor: &str,
    note: Option<&str>,
    source: &str,
) -> ArgoResult<ActionRecord> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut next = located.record.clone();
    next.state = ActionState::Blocked.as_str().to_string();
    next.created_at = now.clone();
    next.approved_by = Some(actor.to_string());
    next.approved_at = Some(now);
    next.blocked_reason = Some(format!("rejected by {actor} via {source}"));
    next.result = Some(serde_json::json!({
        "decision": "reject",
        "source": source,
        "note": note,
    }));
    let body = serde_json::to_string(&next)
        .map_err(|e| ArgoError::Provider(format!("serialize ActionRecord: {e}")))?;
    memory
        .append_for_vertical(
            workspace,
            MemoryFile::Memory,
            body,
            None,
            &located.vertical_id,
        )
        .await?;
    Ok(next)
}

fn resolved_by(record: &ActionRecord) -> String {
    record
        .approved_by
        .clone()
        .or_else(|| {
            record
                .blocked_reason
                .as_ref()
                .map(|reason| reason.trim_start_matches("rejected by ").to_string())
        })
        .unwrap_or_else(|| format!("state {}", record.state))
}

fn header_value<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    let head = req.split_once("\r\n\r\n").map(|(h, _)| h).unwrap_or(req);
    for line in head.lines().skip(1) {
        let (k, v) = line.split_once(':')?;
        if k.eq_ignore_ascii_case(name) {
            return Some(v.trim());
        }
    }
    None
}

fn slack_payload(body: &str) -> Result<Value, String> {
    let raw = form_value(body, "payload").unwrap_or_else(|| body.to_string());
    serde_json::from_str::<Value>(&raw).map_err(|e| format!("parse Slack payload: {e}"))
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
        .and_then(|x| x.as_str())
        .or_else(|| v.get("user")?.get("username").and_then(|x| x.as_str()))
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
    let base = format!("v0:{ts}:{body}");
    let expected = format!("v0={}", hmac_hex(base.as_bytes(), secret.as_bytes()));
    constant_time_eq(expected.as_bytes(), sig.trim().as_bytes())
}

fn form_value(body: &str, key: &str) -> Option<String> {
    for kv in body.split('&') {
        let (k, v) = kv.split_once('=')?;
        if percent_decode(k).as_deref() == Some(key) {
            return percent_decode(v);
        }
    }
    None
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hi = hex_val(bytes[i + 1])?;
                let lo = hex_val(bytes[i + 2])?;
                out.push((hi << 4) | lo);
                i += 2;
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8(out).ok()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn actor_from_payload(v: &Value) -> String {
    v.get("From")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("FromFull")?.get("Email").and_then(|x| x.as_str()))
        .unwrap_or("email-operator")
        .to_string()
}

fn note_from_payload(v: &Value) -> Option<String> {
    v.get("StrippedTextReply")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("TextBody").and_then(|x| x.as_str()))
        .map(str::to_string)
}

fn extract_token(req: &str, payload: &Value) -> Option<String> {
    request_query_token(req)
        .or_else(|| json_token(payload))
        .or_else(|| text_token(payload))
}

fn extract_sms_token(req: &str) -> Option<String> {
    sms_path_token(req).or_else(|| {
        let path = req.strip_prefix("GET ")?;
        let (target, _) = path.split_once(' ')?;
        let (_, query) = target.split_once('?')?;
        query
            .split('&')
            .find_map(|kv| kv.strip_prefix("token=").map(clean_token))
    })
}

fn sms_path_token(req: &str) -> Option<String> {
    let path = req.strip_prefix("GET /channels/sms/decision/")?;
    let (target, _) = path.split_once(' ')?;
    let token = target.split_once('?').map(|(t, _)| t).unwrap_or(target);
    Some(clean_token(token))
}

fn request_query_token(req: &str) -> Option<String> {
    let path = req.strip_prefix("POST ")?;
    let (target, _) = path.split_once(' ')?;
    let (_, query) = target.split_once('?')?;
    query
        .split('&')
        .find_map(|kv| kv.strip_prefix("token=").map(clean_token))
}

fn json_token(payload: &Value) -> Option<String> {
    for key in ["token", "Token", "approval_token", "ApprovalToken"] {
        if let Some(s) = payload.get(key).and_then(|v| v.as_str()) {
            return Some(clean_token(s));
        }
    }
    None
}

fn text_token(payload: &Value) -> Option<String> {
    for key in ["StrippedTextReply", "TextBody", "HtmlBody", "Subject"] {
        if let Some(s) = payload.get(key).and_then(|v| v.as_str()) {
            if let Some(t) = find_token_in_text(s) {
                return Some(t);
            }
        }
    }
    None
}

fn find_token_in_text(s: &str) -> Option<String> {
    for marker in ["approval_token=", "token="] {
        if let Some((_, rest)) = s.split_once(marker) {
            return Some(clean_token(rest));
        }
    }
    None
}

fn clean_token(s: &str) -> String {
    s.trim_matches(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | '('))
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | '('))
        .next()
        .unwrap_or("")
        .to_string()
}

fn verify_token(token: &str, secret: &str) -> Result<TokenClaims, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 4 {
        return Err("approval token must have 4 fields".into());
    }
    let decision = match parts[1] {
        "approve" => Decision::Approve,
        "reject" => Decision::Reject,
        _ => return Err("invalid approval decision".into()),
    };
    let exp = parts[2]
        .parse::<i64>()
        .map_err(|e| format!("invalid approval token exp: {e}"))?;
    if chrono::Utc::now().timestamp() > exp {
        return Err("approval token expired".into());
    }
    let payload = format!("{}|{}|{}", parts[0], parts[1], exp);
    let expected = hmac_hex(payload.as_bytes(), secret.as_bytes());
    if !constant_time_eq(expected.as_bytes(), parts[3].as_bytes()) {
        return Err("approval token signature mismatch".into());
    }
    Ok(TokenClaims {
        approval_id: parts[0].to_string(),
        decision,
    })
}

fn verify_body_signature(body: &[u8], sig: &str, secret: &[u8]) -> bool {
    let expected = hmac_hex(body, secret);
    constant_time_eq(expected.as_bytes(), sig.trim().as_bytes())
}

fn hmac_hex(data: &[u8], secret: &[u8]) -> String {
    const BLOCK: usize = 64;
    let mut key = [0_u8; BLOCK];
    if secret.len() > BLOCK {
        let digest = Sha256::digest(secret);
        key[..digest.len()].copy_from_slice(&digest);
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut ipad = [0x36_u8; BLOCK];
    let mut opad = [0x5c_u8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner);
    hex::encode(outer.finalize())
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

fn bad_request(msg: &str) -> (&'static str, String) {
    ("400 Bad Request", format!("{{\"error\":\"{msg}\"}}"))
}

fn unauthorized(msg: &str) -> (&'static str, String) {
    ("401 Unauthorized", format!("{{\"error\":\"{msg}\"}}"))
}

fn server_error(msg: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        format!("{{\"error\":\"{msg}\"}}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_token_from_postmark_body() {
        let payload = serde_json::json!({
            "TextBody": "ok\napproval_token=act.approve.999.sig\nthanks",
        });
        assert_eq!(
            extract_token("POST /channels/email/webhook HTTP/1.1\r\n\r\n{}", &payload),
            Some("act.approve.999.sig".to_string())
        );
    }

    #[test]
    fn extracts_sms_token_from_decision_path() {
        assert_eq!(
            extract_sms_token("GET /channels/sms/decision/act.reject.999.sig HTTP/1.1\r\n\r\n"),
            Some("act.reject.999.sig".to_string())
        );
    }

    #[test]
    fn detects_sms_decision_route() {
        assert!(is_email_webhook(
            "GET /channels/sms/decision/act.approve.999.sig HTTP/1.1\r\n\r\n"
        ));
    }

    #[test]
    fn verifies_manual_hmac_against_known_token() {
        let exp = chrono::Utc::now().timestamp() + 60;
        let payload = format!("act|approve|{exp}");
        let sig = hmac_hex(payload.as_bytes(), b"secret");
        let token = format!("act.approve.{exp}.{sig}");
        let claims = verify_token(&token, "secret").expect("claims");
        assert_eq!(claims.approval_id, "act");
    }

    #[test]
    fn parses_slack_interactivity_payload() {
        let body = "payload=%7B%22user%22%3A%7B%22id%22%3A%22U1%22%7D%2C%22actions%22%3A%5B%7B%22action_id%22%3A%22approve%22%2C%22value%22%3A%22act.approve.999.sig%22%7D%5D%7D";
        let parsed = slack_payload(body).expect("payload");
        assert_eq!(slack_actor(&parsed), "U1");
        assert_eq!(
            slack_action_token(&parsed),
            Some("act.approve.999.sig".into())
        );
    }

    #[test]
    fn verifies_slack_signature() {
        let ts = chrono::Utc::now().timestamp().to_string();
        let body = "payload=ok";
        let sig = format!(
            "v0={}",
            hmac_hex(format!("v0:{ts}:{body}").as_bytes(), b"secret")
        );
        assert!(verify_slack_signature(body, &ts, &sig, "secret"));
    }
}
