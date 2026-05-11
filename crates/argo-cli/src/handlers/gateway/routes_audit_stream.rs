//! `GET /audit/stream` — initial snapshot response.
//!
//! Real WebSocket upgrades are intercepted upstream by
//! [`super::ws_upgrade`] (`mission-inbox-ws-real-websocket`):
//! when the request carries `Upgrade: websocket` the accept loop
//! in `boot.rs` hands the raw `TcpStream` to the WS handler, which
//! performs the RFC 6455 handshake and runs the frame loop.
//!
//! This module owns the plain-GET snapshot path — a JSON body of
//! the most recent audit rows matching the supplied filters. Tools
//! that don't speak WebSocket (curl probes, dashboards in
//! one-shot polling mode) keep working unchanged.
//!
//! Auth mirrors `/inbox/ws`: bearer token via `Authorization`
//! header or `argo_operator_token` cookie. When
//! `ARGO_AUDIT_STREAM_BEARER_TOKEN` is set, the supplied token
//! must match. Missing → 401; mismatched → 401.
use argo_audit::tail_rows_filtered;
use argo_store::SessionStore;
use serde_json::json;
use std::sync::Arc;

/// Limit on the snapshot row count returned at handshake. The
/// live broadcast channel is the steady-state mechanism; this
/// snapshot just lets the client paint immediately.
const SNAPSHOT_DEFAULT: u32 = 50;
const SNAPSHOT_MAX: u32 = 500;

/// Top-level dispatcher for `GET /audit/stream`.
pub(super) async fn dispatch_audit_stream(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return (
            "503 Service Unavailable",
            json!({"error":"store unavailable"}).to_string(),
        );
    };
    if let Err(resp) = check_auth(req) {
        return resp;
    }
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let kind = parse_query_kv(tail, "kind");
    let worker = parse_query_kv(tail, "worker");
    let limit: u32 = parse_query_kv(tail, "limit")
        .and_then(|v| v.parse().ok())
        .map_or(SNAPSHOT_DEFAULT, |n: u32| n.min(SNAPSHOT_MAX));
    if let Some(k) = kind.as_deref() {
        if !valid_kind_filter(k) {
            return bad_request(&format!(
                "invalid kind filter '{k}': expected csv of audit kinds"
            ));
        }
    }
    if let Some(w) = worker.as_deref() {
        if !valid_worker_filter(w) {
            return bad_request(&format!(
                "invalid worker filter '{w}': expected uuid"
            ));
        }
    }
    snapshot_response(store, kind.as_deref(), worker.as_deref(), limit).await
}

async fn snapshot_response(
    store: &SessionStore,
    kind: Option<&str>,
    worker: Option<&str>,
    limit: u32,
) -> (&'static str, String) {
    // The existing TailFilters only accepts a single `kind`. When
    // the operator supplies a CSV (e.g. `?kind=tool_call,turn_end`),
    // we walk each kind and merge the per-kind windows. This is fine
    // for the snapshot path; the live broadcast doesn't go through
    // SQL so the CSV applies post-receive.
    let single_kinds: Vec<String> = kind
        .map(|k| {
            k.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let mut rows = Vec::new();
    if single_kinds.is_empty() {
        let filters = argo_audit::TailFilters::default();
        rows.extend(
            tail_rows_filtered(store, limit, &filters)
                .await
                .unwrap_or_default(),
        );
    } else {
        for k in &single_kinds {
            let filters = argo_audit::TailFilters {
                kind: Some(k.as_str()),
                ..Default::default()
            };
            rows.extend(
                tail_rows_filtered(store, limit, &filters)
                    .await
                    .unwrap_or_default(),
            );
        }
        rows.sort_by_key(|r| r.seq);
        rows.truncate(limit as usize);
    }
    let filtered: Vec<_> = rows
        .into_iter()
        .filter(|r| match worker {
            Some(w) => r
                .payload
                .get("worker_id")
                .and_then(|v| v.as_str())
                .is_some_and(|val| val == w),
            None => true,
        })
        .collect();
    let body = json!({
        "type": "snapshot",
        "filter": {"kind": kind, "worker": worker, "limit": limit},
        "rows": filtered,
        "broadcast_capacity": argo_audit::AUDIT_BROADCAST_CAPACITY,
    })
    .to_string();
    // Real WS upgrades are intercepted in `boot.rs` ahead of route
    // dispatch — anything reaching this handler is a plain GET that
    // just wants the snapshot body.
    ("200 OK", body)
}

fn check_auth(req: &str) -> Result<(), (&'static str, String)> {
    let token = header_value(req, "Authorization")
        .and_then(|h| h.strip_prefix("Bearer ").map(str::to_string))
        .or_else(|| cookie_value(req, "argo_operator_token"));
    let token = token.ok_or_else(|| {
        (
            "401 Unauthorized",
            json!({"error":"missing bearer token"}).to_string(),
        )
    })?;
    if let Ok(expected) = argo_config::cred::get("ARGO_AUDIT_STREAM_BEARER_TOKEN") {
        if !expected.is_empty() && token != expected {
            return Err((
                "401 Unauthorized",
                json!({"error":"invalid bearer token"}).to_string(),
            ));
        }
    }
    Ok(())
}

fn parse_query_kv(line: &str, key: &str) -> Option<String> {
    let q = line.split_once('?').map(|(_, q)| q).unwrap_or("");
    let q = q.split_once(' ').map_or(q, |(q, _)| q);
    for kv in q.split('&') {
        if let Some(v) = kv.strip_prefix(&format!("{key}=")) {
            return Some(percent_decode(v));
        }
    }
    None
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(h), Some(l)) = (hi, lo) {
                if let Ok(byte) = u8::from_str_radix(&format!("{h}{l}"), 16) {
                    out.push(byte as char);
                    continue;
                }
            }
            out.push(c);
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn header_value<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    req.lines()
        .skip(1)
        .find(|l| l.split_once(':').map(|(k, _)| k.eq_ignore_ascii_case(name)).unwrap_or(false))
        .and_then(|l| l.split_once(':').map(|(_, v)| v.trim()))
}

fn cookie_value(req: &str, key: &str) -> Option<String> {
    let raw = header_value(req, "Cookie")?;
    for kv in raw.split(';') {
        let kv = kv.trim();
        if let Some(rest) = kv.strip_prefix(&format!("{key}=")) {
            return Some(rest.to_string());
        }
    }
    None
}

fn bad_request(msg: &str) -> (&'static str, String) {
    ("400 Bad Request", json!({"error": msg}).to_string())
}

fn valid_kind_filter(s: &str) -> bool {
    !s.is_empty()
        && s.split(',')
            .map(str::trim)
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.'))
}

fn valid_worker_filter(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_query_kv_single() {
        assert_eq!(
            parse_query_kv("/audit/stream?kind=tool_call ", "kind"),
            Some("tool_call".to_string())
        );
    }

    #[test]
    fn parse_query_kv_multi() {
        assert_eq!(
            parse_query_kv("/audit/stream?kind=tool_call&worker=abc ", "worker"),
            Some("abc".to_string())
        );
    }

    #[test]
    fn parse_query_kv_missing() {
        assert_eq!(parse_query_kv("/audit/stream ", "kind"), None);
    }

    #[test]
    fn valid_kind_filter_accepts_csv() {
        assert!(valid_kind_filter("tool_call"));
        assert!(valid_kind_filter("tool_call,turn_start"));
        assert!(valid_kind_filter("escalation.required"));
    }

    #[test]
    fn valid_kind_filter_rejects_empties() {
        assert!(!valid_kind_filter(""));
        assert!(!valid_kind_filter(","));
        assert!(!valid_kind_filter("tool call"));
    }

    #[test]
    fn valid_worker_filter_accepts_uuid() {
        assert!(valid_worker_filter("00000000-0000-0000-0000-000000000000"));
    }

    #[test]
    fn valid_worker_filter_rejects_garbage() {
        assert!(!valid_worker_filter("not-a-uuid"));
        assert!(!valid_worker_filter(""));
    }

    #[test]
    fn percent_decode_handles_plus_and_hex() {
        assert_eq!(percent_decode("a+b%3Dc"), "a b=c");
    }

    #[test]
    fn check_auth_missing_token_rejects() {
        let r = check_auth("GET /audit/stream HTTP/1.1\r\n\r\n");
        assert!(r.is_err());
    }

    #[test]
    fn check_auth_serial_paths() {
        // Combined to avoid env-var cross-test races; auth is
        // process-global so we test both paths in sequence.
        std::env::remove_var("ARGO_AUDIT_STREAM_BEARER_TOKEN");
        let ok = check_auth("GET /audit/stream HTTP/1.1\r\nAuthorization: Bearer t\r\n\r\n");
        assert!(ok.is_ok(), "no-env should accept any bearer");

        std::env::set_var("ARGO_AUDIT_STREAM_BEARER_TOKEN", "expected");
        let bad = check_auth("GET /audit/stream HTTP/1.1\r\nAuthorization: Bearer wrong\r\n\r\n");
        assert!(bad.is_err(), "env-locked token should reject mismatch");

        let good = check_auth(
            "GET /audit/stream HTTP/1.1\r\nAuthorization: Bearer expected\r\n\r\n",
        );
        assert!(good.is_ok(), "env-locked token should accept match");
        std::env::remove_var("ARGO_AUDIT_STREAM_BEARER_TOKEN");
    }
}
