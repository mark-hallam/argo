//! Push (Expo) approval channel routes.
//!
//! Sibling of [`super::routes_channels`]. Owns device registration
//! (`POST /channels/push/devices`) and the deep-link decision
//! callback (`POST /channels/push/decision/<token>`). Shared
//! settle/HMAC/helpers live in the parent file.

use super::routes_channels::{
    bad_request, hmac_secret, percent_decode, server_error, settle, unauthorized, verify_token,
    Decision,
};
use serde_json::{json, Value};
use std::path::PathBuf;

pub(super) fn is_push(req: &str) -> bool {
    req.starts_with("POST /channels/push/devices ")
        || req.starts_with("POST /channels/push/devices?")
        || req.starts_with("POST /channels/push/decision/")
}

pub(super) async fn push_response(req: &str) -> (&'static str, String) {
    if req.starts_with("POST /channels/push/devices ") || req.starts_with("POST /channels/push/devices?") {
        let mut v: Value = match serde_json::from_str(super::body_from_request(req)) { Ok(v) => v, Err(e) => return bad_request(&format!("parse push device JSON: {e}")) };
        for k in ["operator_id", "expo_push_token", "platform"] {
            if v.get(k).and_then(Value::as_str).is_none_or(|s| s.trim().is_empty()) {
                return bad_request(&format!("missing push device {k}"));
            }
        }
        v["registered_at"] = json!(chrono::Utc::now().to_rfc3339());
        return match append_push_device(&v) {
            Ok(()) => ("200 OK", json!({"ok":true,"registered":true}).to_string()),
            Err(e) => server_error(&e),
        };
    }
    let Some((token, requested)) = push_decision(req) else { return bad_request("unsupported push route"); };
    let secret = match hmac_secret() { Ok(s) => s, Err(e) => return server_error(&e) };
    let mut claims = match verify_token(&token, &secret) { Ok(c) => c, Err(e) => return unauthorized(&e) };
    if let Some(d) = requested { claims.decision = d; }
    match settle(&claims, "push-operator", Some("push approval"), "push").await {
        Ok(v) => ("200 OK", v),
        Err(e) => server_error(&e.to_string()),
    }
}

fn push_devices_path() -> PathBuf {
    std::env::var_os("ARGO_PUSH_DEVICES_PATH").map(PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(".skycot/push_devices.jsonl"))
}

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
