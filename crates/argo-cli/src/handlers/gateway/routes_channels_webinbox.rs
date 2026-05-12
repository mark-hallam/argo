//! Web inbox routes (`/inbox`, `/inbox/ws`, `/inbox/approve|reject`).
//!
//! Sibling of [`super::routes_channels`]. Owns the operator-bound
//! pending-approvals view, server-rendered HTML form, and decision
//! POST handler. Shared settle/HMAC/helpers live in the parent
//! file.

use super::routes_channels::{
    bad_request, header_value, hmac_secret, percent_decode, server_error, settle, unauthorized,
    verify_token, Decision,
};
use super::routes_channels_email::token_from_url;
use crate::handlers::worker_actions::ActionState;
use crate::handlers::worker_actions_transitions;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct InboxRow { operator_id: String, approval_id: String, action_id: String, risk_level: String, prompt_body: String, approve_url: String, reject_url: String, expiry: String, enqueued_at: String }

pub(super) fn is_webinbox(req: &str) -> bool {
    ["GET /inbox ", "GET /inbox?", "GET /inbox/ws ", "GET /inbox/ws?", "POST /inbox/approve/", "POST /inbox/reject/"].iter().any(|p| req.starts_with(p))
}

pub(super) async fn webinbox_response(req: &str) -> (&'static str, String) {
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

async fn inbox_page(operator: &str) -> (&'static str, String) {
    let rows = match pending_rows(operator).await { Ok(r) => r, Err(e) => return server_error(&e.to_string()) };
    let body = if rows.is_empty() { "<p>No pending approvals.</p>".to_string() } else { rows.iter().map(render_row).collect::<String>() };
    // Polling fallback runs until the WS upgrade succeeds; we clear
    // the interval on `open` so the page doesn't refresh every 5s
    // (which would tear down the live socket). If the WS later closes
    // we reinstall the timer so the operator still sees decisions.
    let script = "<script>let t=setInterval(()=>location.reload(),5000);try{let p=location.protocol==='https:'?'wss':'ws';let ws=new WebSocket(p+'://'+location.host+'/inbox/ws');ws.onopen=()=>{clearInterval(t);t=null;};ws.onmessage=()=>location.reload();ws.onclose=()=>{if(!t)t=setInterval(()=>location.reload(),5000);};}catch(e){}</script>";
    ("200 OK", format!("<!doctype html><html><head><title>Argo Inbox</title></head><body><h1>Argo Inbox</h1><p>operator {}</p>{body}{script}</body></html>", esc(operator)))
}

/// Plain-GET snapshot (no `Upgrade: websocket` header). Real WS
/// upgrade requests are routed up in `boot.rs` to
/// [`super::ws_upgrade::serve`] before this handler is reached, so
/// the WS branch here only fires for tools that mis-spell the header
/// (curl probes etc.) — keep emitting the legacy snapshot for them.
async fn inbox_ws(_req: &str, operator: &str) -> (&'static str, String) {
    let rows = match pending_rows(operator).await { Ok(r) => r, Err(e) => return server_error(&e.to_string()) };
    let events: Vec<Value> = rows.into_iter().map(|r| json!({"type":"approval_pending","approval_id":r.approval_id,"action_id":r.action_id,"risk_level":r.risk_level})).collect();
    ("200 OK", json!({"type":"snapshot","events":events}).to_string())
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
        Ok(v) => {
            // Legacy text-frame reload signal for the HTML inbox — kept
            // verbatim so old consumers still refetch on decision.
            super::ws_upgrade::publish_inbox_event(json!({
                "event": "decision",
                "approval_id": approval_id,
                "decision": match decision { Decision::Approve => "approve", Decision::Reject => "reject" },
                "operator": operator,
            }));
            // J3 structured event — per-tenant fan-out for SPA
            // consumers. v1 maps operator_id → tenant_id (see
            // ws_inbox_events.rs).
            argo_channels::webinbox::dispatch_event(
                argo_channels::webinbox::WebInboxEvent::ApprovalDecided {
                    tenant_id: operator.to_string(),
                    row: argo_channels::webinbox::WebInboxRow {
                        operator_id: operator.to_string(),
                        approval_id: approval_id.to_string(),
                        action_id: approval_id.to_string(),
                        risk_level: argo_channels::RiskLevel::High,
                        prompt_body: String::new(),
                        approve_url: String::new(),
                        reject_url: String::new(),
                        expiry: chrono::Utc::now(),
                        enqueued_at: chrono::Utc::now(),
                    },
                    decision: match decision {
                        Decision::Approve => "approve".into(),
                        Decision::Reject => "refuse".into(),
                    },
                },
            );
            ("200 OK", v)
        }
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
    if let Ok(expected) = argo_config::cred::get("ARGO_WEBINBOX_BEARER_TOKEN") {
        if !expected.is_empty() && token != expected {
            return Err(unauthorized("invalid bearer token"));
        }
    }
    Ok(std::env::var("ARGO_WEBINBOX_OPERATOR_ID").ok().filter(|s| !s.is_empty()).unwrap_or(token))
}

fn cookie_value(req: &str, key: &str) -> Option<String> {
    header_value(req, "Cookie")?.split(';').find_map(|part| { let (k, v) = part.trim().split_once('=')?; (k == key).then(|| v.to_string()) })
}

fn inbox_path() -> PathBuf {
    std::env::var_os("ARGO_WEBINBOX_PATH").map(PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(".skycot/webinbox.jsonl"))
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

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
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

#[cfg(test)]
mod tests {
    use super::super::routes_channels::hmac_hex;
    use super::*;
    use crate::handlers::worker_actions::ActionRecord;
    use argo_memory::MemoryFile;

    fn set_verticals_dir() {
        std::env::set_var("ARGO_VERTICALS_DIR", PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().and_then(|p| p.parent()).expect("workspace root").join("verticals"));
    }

    fn record(id: &str) -> ActionRecord {
        ActionRecord { schema_key: ActionRecord::SCHEMA_KEY.into(), action_id: id.into(), job_id: "job-1".into(), worker_id: argo_identity::WorkerId::nil(), item_id: None, action_type: "post_listing".into(), risk_level: "high".into(), requires_approval: true, state: "pending".into(), payload: json!({}), approved_by: None, approved_at: None, executed_at: None, platform: None, result: None, evidence: Vec::new(), blocked_reason: None, created_at: chrono::Utc::now().to_rfc3339() }
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
}
