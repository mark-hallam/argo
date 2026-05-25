//! Internal helpers for [`super::routes_approvals_json`]. Split out so
//! both files stay under the 500-LOC HARD RULE 6 ceiling. Items are
//! `pub(super)`-only and not part of the gateway's public surface.

use serde_json::{json, Value};

/// Hard cap on `?limit=`. Brief: default 50, max 100.
pub(super) const DEFAULT_LIMIT: usize = 50;
pub(super) const MAX_LIMIT: usize = 100;

pub(super) fn short_payload(v: &Value) -> String {
    let s = serde_json::to_string(v).unwrap_or_default();
    if s.len() > 200 {
        format!("{}…", &s[..199])
    } else {
        s
    }
}

pub(super) fn parse_limit(tail: &str) -> Result<usize, (&'static str, String)> {
    let Some(raw) = parse_query(tail, "limit") else {
        return Ok(DEFAULT_LIMIT);
    };
    let Ok(n) = raw.parse::<usize>() else {
        return Err(bad_request(&format!("limit '{raw}' not a number")));
    };
    if n == 0 || n > MAX_LIMIT {
        return Err(bad_request(&format!("limit must be 1..={MAX_LIMIT}")));
    }
    Ok(n)
}

pub(super) fn parse_query(tail: &str, key: &str) -> Option<String> {
    let head = tail.split_once(' ').map(|(p, _)| p).unwrap_or(tail);
    let qs = head.split_once('?').map(|(_, q)| q)?;
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

pub(super) fn encode_cursor(ts: &str, id: &str) -> String {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::URL_SAFE_NO_PAD.encode(format!("{ts}|{id}"))
}

pub(super) fn decode_cursor(c: &str) -> Option<(String, String)> {
    use base64::{engine::general_purpose, Engine as _};
    let bytes = general_purpose::URL_SAFE_NO_PAD.decode(c).ok()?;
    let s = String::from_utf8(bytes).ok()?;
    let (ts, id) = s.split_once('|')?;
    Some((ts.to_string(), id.to_string()))
}

pub(super) fn ok_decide(approval_id: &str, decision: &str) -> (&'static str, String) {
    (
        "200 OK",
        json!({
            "ok": true,
            "approval_id": approval_id,
            "decision": decision,
        })
        .to_string(),
    )
}

pub(super) fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

pub(super) fn not_found(id: &str) -> (&'static str, String) {
    error_envelope(
        "404 Not Found",
        "approval_not_found",
        &format!("no approval with id '{id}'"),
    )
}

pub(super) fn conflict(id: &str, current: &str) -> (&'static str, String) {
    error_envelope(
        "409 Conflict",
        "already_decided",
        &format!("approval '{id}' already decided ({current})"),
    )
}

pub(super) fn gone(id: &str) -> (&'static str, String) {
    error_envelope(
        "410 Gone",
        "undo_window_expired",
        &format!("approval '{id}': undo window has expired"),
    )
}

pub(super) fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

pub(super) fn error_envelope(
    status: &'static str,
    code: &str,
    message: &str,
) -> (&'static str, String) {
    (
        status,
        json!({
            "error": {
                "code": code,
                "message": message,
            }
        })
        .to_string(),
    )
}
