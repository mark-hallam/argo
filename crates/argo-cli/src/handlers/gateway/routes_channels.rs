//! Channel webhook and web inbox routes.

use crate::handlers::worker_actions::{ActionRecord, ActionState};
use crate::handlers::worker_actions_transitions;
use argo_memory::{MemoryFile, MemoryStore};
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Decision { Approve, Reject }

struct TokenClaims { approval_id: String, decision: Decision }

#[derive(Clone, Debug, Deserialize, Serialize)]
struct InboxRow { operator_id: String, approval_id: String, action_id: String, risk_level: String, prompt_body: String, approve_url: String, reject_url: String, expiry: String, enqueued_at: String }

pub(super) fn is_email_webhook(req: &str) -> bool {
    req.starts_with("POST /channels/email/webhook ")
        || req.starts_with("POST /channels/email/webhook?")
        || is_slack_webhook(req)
        || is_webinbox(req)
        || is_push(req)
}

fn is_slack_webhook(req: &str) -> bool {
    req.starts_with("POST /channels/slack/webhook ")
        || req.starts_with("POST /channels/slack/webhook?")
}

fn is_webinbox(req: &str) -> bool {
    ["GET /inbox ", "GET /inbox?", "GET /inbox/ws ", "GET /inbox/ws?", "POST /inbox/approve/", "POST /inbox/reject/"].iter().any(|p| req.starts_with(p))
}

fn is_push(req: &str) -> bool { req.starts_with("POST /channels/push/devices ") || req.starts_with("POST /channels/push/devices?") || req.starts_with("POST /channels/push/decision/") }

pub(super) async fn email_webhook_response(req: &str) -> (&'static str, String) {
    if is_slack_webhook(req) {
        return slack_webhook_response(req).await;
    }
    if is_webinbox(req) {
        return webinbox_response(req).await;
    }
    if is_push(req) {
        return push_response(req).await;
    }
    let secret = match hmac_secret() {
        Ok(s) => s,
        Err(e) => return server_error(&e),
    };
    let body = super::body_from_request(req);
    let Some(sig) = header_value(req, "X-Postmark-Signature") else {
        return unauthorized("missing X-Postmark-Signature");
    };
    if !constant_time_eq(hmac_hex(body.as_bytes(), secret.as_bytes()).as_bytes(), sig.as_bytes()) {
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
    match settle(&claims, &actor_from_payload(&parsed), note_from_payload(&parsed).as_deref(), "email").await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

async fn slack_webhook_response(req: &str) -> (&'static str, String) {
    let Ok(signing_secret) = std::env::var("SLACK_APPROVALS_SIGNING_SECRET") else {
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
    match settle(&claims, &slack_actor(&parsed), slack_note(&parsed).as_deref(), "slack").await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

async fn webinbox_response(req: &str) -> (&'static str, String) {
    let operator = match operator_from_auth(req) { Ok(op) => op, Err(resp) => return resp };
    if req.starts_with("GET /inbox/ws ") || req.starts_with("GET /inbox/ws?") {
        return inbox_ws(req, &operator).await;
    }
    if req.starts_with("GET /inbox ") || req.starts_with("GET /inbox?") {
        return inbox_page(&operator).await;
    }
    let Some((decision, id)) = inbox_post(req) else {
        return bad_request("unsupported inbox route");
    };
    inbox_decide(&operator, &id, decision).await
}
async fn push_response(req: &str) -> (&'static str, String) {
    if req.starts_with("POST /channels/push/devices ") || req.starts_with("POST /channels/push/devices?") {
        let mut v: Value = match serde_json::from_str(super::body_from_request(req)) { Ok(v) => v, Err(e) => return bad_request(&format!("parse push device JSON: {e}")) };
        for k in ["operator_id", "expo_push_token", "platform"] {
            if v.get(k).and_then(Value::as_str).is_none_or(|s| s.trim().is_empty()) { return bad_request(&format!("missing push device {k}")); }
        }
        v["registered_at"] = json!(chrono::Utc::now().to_rfc3339());
        return match append_push_device(&v) { Ok(()) => ("200 OK", json!({"ok":true,"registered":true}).to_string()), Err(e) => server_error(&e) };
    }
    let Some((token, requested)) = push_decision(req) else { return bad_request("unsupported push route"); };
    let secret = match hmac_secret() { Ok(s) => s, Err(e) => return server_error(&e) };
    let mut claims = match verify_token(&token, &secret) { Ok(c) => c, Err(e) => return unauthorized(&e) };
    if let Some(d) = requested { claims.decision = d; }
    match settle(&claims, "push-operator", Some("push approval"), "push").await { Ok(v) => ("200 OK", v), Err(e) => server_error(&e.to_string()) }
}

async fn inbox_page(operator: &str) -> (&'static str, String) {
    let rows = match pending_rows(operator).await { Ok(r) => r, Err(e) => return server_error(&e.to_string()) };
    let body = if rows.is_empty() { "<p>No pending approvals.</p>".to_string() } else { rows.iter().map(render_row).collect::<String>() };
    let script = "<script>setInterval(()=>location.reload(),5000);try{let p=location.protocol==='https:'?'wss':'ws';let ws=new WebSocket(p+'://'+location.host+'/inbox/ws');ws.onmessage=()=>location.reload();}catch(e){}</script>";
    ("200 OK", format!("<!doctype html><html><head><title>Argo Inbox</title></head><body><h1>Argo Inbox</h1><p>operator {}</p>{body}{script}</body></html>", esc(operator)))
}

async fn inbox_ws(req: &str, operator: &str) -> (&'static str, String) {
    let rows = match pending_rows(operator).await { Ok(r) => r, Err(e) => return server_error(&e.to_string()) };
    let events: Vec<Value> = rows.into_iter().map(|r| json!({"type":"approval_pending","approval_id":r.approval_id,"action_id":r.action_id,"risk_level":r.risk_level})).collect();
    let status = if header_value(req, "Upgrade").is_some_and(|v| v.eq_ignore_ascii_case("websocket")) { "101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade" } else { "200 OK" };
    (status, json!({"type":"snapshot","events":events}).to_string())
}

async fn inbox_decide(operator: &str, approval_id: &str, decision: Decision) -> (&'static str, String) {
    let rows = match load_rows() { Ok(r) => r, Err(e) => return server_error(&e) };
    let Some(row) = rows.iter().find(|r| r.operator_id == operator && r.approval_id == approval_id) else { return bad_request("approval not in operator inbox"); };
    let url = if decision == Decision::Approve { &row.approve_url } else { &row.reject_url };
    let Some(token) = token_from_url(url) else {
        return bad_request("inbox row missing signed token");
    };
    let secret = match hmac_secret() { Ok(s) => s, Err(e) => return server_error(&e) };
    let claims = match verify_token(&token, &secret) {
        Ok(c) if c.approval_id == approval_id && c.decision == decision => c,
        Ok(_) => return unauthorized("approval token does not match inbox decision"),
        Err(e) => return unauthorized(&e),
    };
    match settle(&claims, operator, Some("web inbox"), "webinbox").await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

async fn pending_rows(operator: &str) -> ArgoResult<Vec<InboxRow>> {
    let rows = load_rows().map_err(ArgoError::Provider)?;
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let mut out = Vec::new();
    for row in rows.into_iter().filter(|r| r.operator_id == operator) {
        if let Some(located) = worker_actions_transitions::find_action(&memory, ws, &row.approval_id).await? {
            if located.record.state == ActionState::Pending.as_str() && located.record.requires_approval {
                out.push(row);
            }
        }
    }
    Ok(out)
}

fn render_row(row: &InboxRow) -> String {
    format!(
        "<section data-approval-id=\"{}\"><h2>{}</h2><p>{}</p><p>risk: {} expires: {}</p><form method=\"post\" action=\"/inbox/approve/{}\"><button>Approve</button></form><form method=\"post\" action=\"/inbox/reject/{}\"><button>Reject</button></form></section>",
        esc(&row.approval_id), esc(&row.action_id), esc(&row.prompt_body), esc(&row.risk_level), esc(&row.expiry), esc(&row.approval_id), esc(&row.approval_id)
    )
}

fn inbox_post(req: &str) -> Option<(Decision, String)> {
    let (decision, rest) = req.strip_prefix("POST /inbox/approve/").map(|r| (Decision::Approve, r)).or_else(|| req.strip_prefix("POST /inbox/reject/").map(|r| (Decision::Reject, r)))?;
    let id = rest.split([' ', '?']).next()?;
    Some((decision, percent_decode(id).unwrap_or_else(|| id.to_string())))
}

fn operator_from_auth(req: &str) -> Result<String, (&'static str, String)> {
    let token = header_value(req, "Authorization").and_then(|h| h.strip_prefix("Bearer ")).map(str::to_string).or_else(|| cookie_value(req, "argo_operator_token")).ok_or_else(|| unauthorized("missing bearer token"))?;
    if let Ok(expected) = std::env::var("ARGO_WEBINBOX_BEARER_TOKEN") {
        if !expected.is_empty() && token != expected {
            return Err(unauthorized("invalid bearer token"));
        }
    }
    Ok(std::env::var("ARGO_WEBINBOX_OPERATOR_ID").ok().filter(|s| !s.is_empty()).unwrap_or(token))
}

fn inbox_path() -> PathBuf {
    std::env::var_os("ARGO_WEBINBOX_PATH").map(PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(".skycot/webinbox.jsonl"))
}

fn push_devices_path() -> PathBuf { std::env::var_os("ARGO_PUSH_DEVICES_PATH").map(PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(".skycot/push_devices.jsonl")) }

fn append_push_device(v: &Value) -> Result<(), String> {
    let path = push_devices_path();
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e| format!("create push devices dir: {e}"))?; }
    let line = serde_json::to_string(v).map_err(|e| format!("serialize push device: {e}"))?;
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|e| format!("open push devices {}: {e}", path.display()))?;
    std::io::Write::write_all(&mut file, format!("{line}\n").as_bytes()).map_err(|e| format!("write push device: {e}"))
}

fn push_decision(req: &str) -> Option<(String, Option<Decision>)> {
    let target = req.strip_prefix("POST /channels/push/decision/")?.split_once(' ').map_or(req, |(p, _)| p);
    let (token, query) = target.split_once('?').unwrap_or((target, ""));
    let decision = query.split('&').filter_map(|kv| kv.split_once('=')).find_map(|(k, v)| (percent_decode(k).as_deref() == Some("d")).then(|| percent_decode(v)).flatten()).and_then(|v| match v.as_str() { "approve" => Some(Decision::Approve), "reject" => Some(Decision::Reject), _ => None });
    Some((percent_decode(token)?, decision))
}

fn load_rows() -> Result<Vec<InboxRow>, String> {
    let path = inbox_path();
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("read inbox {}: {e}", path.display())),
    };
    raw.lines().filter(|l| !l.trim().is_empty()).map(|l| serde_json::from_str(l).map_err(|e| format!("parse inbox row: {e}"))).collect()
}

#[cfg(test)]
fn append_inbox_row(row: &InboxRow) -> Result<(), String> {
    let path = inbox_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create inbox dir: {e}"))?;
    }
    let line = serde_json::to_string(row).map_err(|e| format!("serialize inbox row: {e}"))?;
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path).map_err(|e| format!("open inbox {}: {e}", path.display()))?;
    std::io::Write::write_all(&mut file, format!("{line}\n").as_bytes()).map_err(|e| format!("write inbox: {e}"))
}

async fn settle(claims: &TokenClaims, actor: &str, note: Option<&str>, source: &str) -> ArgoResult<String> {
    let (memory, ws) = worker_actions_transitions::open_workspace_memory().await?;
    let located = worker_actions_transitions::find_action(&memory, ws, &claims.approval_id)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no approval '{}'", claims.approval_id)))?;
    if located.record.state != ActionState::Pending.as_str() {
        return Ok(json!({"ok":true,"already_resolved":true,"approval_id":&claims.approval_id,"state":located.record.state,"resolved_by":resolved_by(&located.record)}).to_string());
    }
    if !located.record.requires_approval {
        return Ok(json!({"ok":true,"already_resolved":true,"approval_id":&claims.approval_id,"state":located.record.state,"resolved_by":"autonomous-safe"}).to_string());
    }
    let rec = match claims.decision {
        Decision::Approve => worker_actions_transitions::approve(&memory, ws, &claims.approval_id, actor, note).await?,
        Decision::Reject => reject_action(&memory, ws, &located, actor, note, source).await?,
    };
    Ok(json!({"ok":true,"approval_id":rec.action_id,"decision":decision_str(claims.decision),"state":rec.state,"resolved_by":actor}).to_string())
}

async fn reject_action(memory: &MemoryStore, workspace: WorkspaceId, located: &worker_actions_transitions::LocatedAction, actor: &str, note: Option<&str>, source: &str) -> ArgoResult<ActionRecord> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut next = located.record.clone();
    next.state = ActionState::Blocked.as_str().to_string();
    next.created_at = now.clone();
    next.approved_by = Some(actor.to_string());
    next.approved_at = Some(now);
    next.blocked_reason = Some(format!("rejected by {actor} via {source}"));
    next.result = Some(json!({"decision":"reject","source":source,"note":note}));
    let body = serde_json::to_string(&next).map_err(|e| ArgoError::Provider(format!("serialize ActionRecord: {e}")))?;
    memory.append_for_vertical(workspace, MemoryFile::Memory, body, None, &located.vertical_id).await?;
    Ok(next)
}

fn resolved_by(record: &ActionRecord) -> String {
    record.approved_by.clone().or_else(|| record.blocked_reason.as_ref().map(|r| r.trim_start_matches("rejected by ").to_string())).unwrap_or_else(|| format!("state {}", record.state))
}

fn header_value<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    let head = req.split_once("\r\n\r\n").map_or(req, |(h, _)| h);
    head.lines().skip(1).find_map(|line| { let (k, v) = line.split_once(':')?; k.eq_ignore_ascii_case(name).then(|| v.trim()) })
}

fn cookie_value(req: &str, key: &str) -> Option<String> {
    header_value(req, "Cookie")?.split(';').find_map(|part| { let (k, v) = part.trim().split_once('=')?; (k == key).then(|| v.to_string()) })
}

fn slack_payload(body: &str) -> Result<Value, String> {
    serde_json::from_str(&form_value(body, "payload").unwrap_or_else(|| body.to_string())).map_err(|e| format!("parse Slack payload: {e}"))
}

fn slack_action_token(v: &Value) -> Option<String> {
    v.get("actions")?.as_array()?.first()?.get("value")?.as_str().map(str::to_string)
}

fn slack_actor(v: &Value) -> String {
    v.get("user").and_then(|u| u.get("id")).and_then(Value::as_str).or_else(|| v.get("user")?.get("username").and_then(Value::as_str)).unwrap_or("slack-operator").to_string()
}

fn slack_note(v: &Value) -> Option<String> {
    v.get("actions")?.as_array()?.first()?.get("action_id")?.as_str().map(str::to_string)
}

fn verify_slack_signature(body: &str, ts: &str, sig: &str, secret: &str) -> bool {
    let Ok(ts_num) = ts.parse::<i64>() else {
        return false;
    };
    if (chrono::Utc::now().timestamp() - ts_num).abs() > 300 {
        return false;
    }
    constant_time_eq(format!("v0={}", hmac_hex(format!("v0:{ts}:{body}").as_bytes(), secret.as_bytes())).as_bytes(), sig.trim().as_bytes())
}

fn form_value(body: &str, key: &str) -> Option<String> {
    body.split('&').find_map(|kv| { let (k, v) = kv.split_once('=')?; (percent_decode(k).as_deref() == Some(key)).then(|| percent_decode(v)).flatten() })
}

fn percent_decode(s: &str) -> Option<String> {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                out.push((hex_val(bytes[i + 1])? << 4) | hex_val(bytes[i + 2])?);
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
    v.get("From").and_then(Value::as_str).or_else(|| v.get("FromFull")?.get("Email").and_then(Value::as_str)).unwrap_or("email-operator").to_string()
}

fn note_from_payload(v: &Value) -> Option<String> {
    ["StrippedTextReply", "TextBody"].iter().find_map(|k| v.get(*k).and_then(Value::as_str)).map(str::to_string)
}

fn extract_token(req: &str, payload: &Value) -> Option<String> {
    request_query_token(req).or_else(|| json_token(payload)).or_else(|| text_token(payload))
}

fn request_query_token(req: &str) -> Option<String> {
    req.strip_prefix("POST ")?.split_once(' ')?.0.split_once('?')?.1.split('&').find_map(|kv| kv.strip_prefix("token=").map(clean_token))
}

fn json_token(payload: &Value) -> Option<String> {
    ["token", "Token", "approval_token", "ApprovalToken"].iter().find_map(|k| payload.get(*k).and_then(Value::as_str).map(clean_token))
}

fn text_token(payload: &Value) -> Option<String> {
    ["StrippedTextReply", "TextBody", "HtmlBody", "Subject"].iter().filter_map(|k| payload.get(*k).and_then(Value::as_str)).find_map(find_token_in_text)
}

fn find_token_in_text(s: &str) -> Option<String> {
    ["approval_token=", "token="].iter().find_map(|m| s.split_once(m).map(|(_, rest)| clean_token(rest)))
}

fn token_from_url(url: &str) -> Option<String> {
    url.split_once('?')?.1.split('&').find_map(|kv| kv.strip_prefix("token=").and_then(percent_decode))
}

fn clean_token(s: &str) -> String {
    s.trim_matches(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | '(')).split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | '(')).next().unwrap_or("").to_string()
}

fn verify_token(token: &str, secret: &str) -> Result<TokenClaims, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 4 {
        return Err("approval token must have 4 fields".into());
    }
    let decision = match parts[1] { "approve" => Decision::Approve, "reject" => Decision::Reject, _ => return Err("invalid approval decision".into()) };
    let exp = parts[2].parse::<i64>().map_err(|e| format!("invalid approval token exp: {e}"))?;
    if chrono::Utc::now().timestamp() > exp {
        return Err("approval token expired".into());
    }
    let expected = hmac_hex(format!("{}|{}|{exp}", parts[0], parts[1]).as_bytes(), secret.as_bytes());
    if !constant_time_eq(expected.as_bytes(), parts[3].as_bytes()) {
        return Err("approval token signature mismatch".into());
    }
    Ok(TokenClaims { approval_id: parts[0].to_string(), decision })
}

fn hmac_secret() -> Result<String, String> {
    let secret = std::env::var("ARGO_CHANNEL_HMAC_SECRET").map_err(|_| "missing ARGO_CHANNEL_HMAC_SECRET".to_string())?;
    if secret.is_empty() { Err("ARGO_CHANNEL_HMAC_SECRET must not be empty".into()) } else { Ok(secret) }
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
    let mut ipad = [0x36_u8; BLOCK]; let mut opad = [0x5c_u8; BLOCK];
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
    a.len() == b.len() && a.iter().zip(b).fold(0_u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

fn decision_str(d: Decision) -> &'static str {
    match d { Decision::Approve => "approve", Decision::Reject => "reject" }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn bad_request(msg: &str) -> (&'static str, String) { ("400 Bad Request", format!("{{\"error\":\"{msg}\"}}")) }
fn unauthorized(msg: &str) -> (&'static str, String) { ("401 Unauthorized", format!("{{\"error\":\"{msg}\"}}")) }
fn server_error(msg: &str) -> (&'static str, String) { ("500 Internal Server Error", format!("{{\"error\":\"{msg}\"}}")) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn set_verticals_dir() {
        std::env::set_var("ARGO_VERTICALS_DIR", PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().and_then(|p| p.parent()).expect("workspace root").join("verticals"));
    }

    fn record(id: &str) -> ActionRecord {
        ActionRecord { schema_key: ActionRecord::SCHEMA_KEY.into(), action_id: id.into(), job_id: "job-1".into(), item_id: None, action_type: "post_listing".into(), risk_level: "high".into(), requires_approval: true, state: "pending".into(), payload: json!({}), approved_by: None, approved_at: None, executed_at: None, platform: None, result: None, evidence: Vec::new(), blocked_reason: None, created_at: chrono::Utc::now().to_rfc3339() }
    }

    #[tokio::test]
    async fn webinbox_lists_and_approves_pending_action() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::env::set_var("ARGO_HOME", tmp.path().join("home"));
        std::fs::create_dir_all(tmp.path().join("home")).unwrap();
        std::env::set_var("ARGO_CHANNEL_HMAC_SECRET", "secret");
        std::env::set_var("ARGO_WEBINBOX_PATH", tmp.path().join("webinbox.jsonl"));
        set_verticals_dir();
        let (memory, ws) = worker_actions_transitions::open_workspace_memory().await.unwrap();
        let rec = record("act-web");
        memory.append_for_vertical(ws, MemoryFile::Memory, serde_json::to_string(&rec).unwrap(), None, "gtm-engineer").await.unwrap();
        let exp = chrono::Utc::now().timestamp() + 60;
        let approve = format!("act-web.approve.{exp}.{}", hmac_hex(format!("act-web|approve|{exp}").as_bytes(), b"secret"));
        let reject = format!("act-web.reject.{exp}.{}", hmac_hex(format!("act-web|reject|{exp}").as_bytes(), b"secret"));
        append_inbox_row(&InboxRow { operator_id: "op-1".into(), approval_id: "act-web".into(), action_id: "act-web".into(), risk_level: "high".into(), prompt_body: "Approve listing?".into(), approve_url: format!("http://gw/decision?token={approve}"), reject_url: format!("http://gw/decision?token={reject}"), expiry: chrono::Utc::now().to_rfc3339(), enqueued_at: chrono::Utc::now().to_rfc3339() }).unwrap();
        let auth = "Authorization: Bearer op-1";
        let (_, html) = webinbox_response(&format!("GET /inbox HTTP/1.1\r\n{auth}\r\n\r\n")).await;
        assert!(html.contains("Approve listing?"));
        let (_, body) = webinbox_response(&format!("POST /inbox/approve/act-web HTTP/1.1\r\n{auth}\r\n\r\n")).await;
        assert!(body.contains("\"decision\":\"approve\""));
        let found = worker_actions_transitions::find_action(&memory, ws, "act-web").await.unwrap().unwrap();
        assert_eq!(found.record.state, "approved");
    }

    #[test]
    fn parses_existing_webhook_tokens_and_slack_payloads() {
        let payload = json!({"TextBody":"ok\napproval_token=act.approve.999.sig\nthanks"});
        assert_eq!(extract_token("POST /channels/email/webhook HTTP/1.1\r\n\r\n{}", &payload), Some("act.approve.999.sig".into()));
        let body = "payload=%7B%22user%22%3A%7B%22id%22%3A%22U1%22%7D%2C%22actions%22%3A%5B%7B%22action_id%22%3A%22approve%22%2C%22value%22%3A%22act.approve.999.sig%22%7D%5D%7D";
        let parsed = slack_payload(body).unwrap();
        assert_eq!(slack_actor(&parsed), "U1");
        assert_eq!(slack_action_token(&parsed), Some("act.approve.999.sig".into()));
    }
}
