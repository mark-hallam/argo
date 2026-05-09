//! HTTP handlers for the `/peer/*` mesh endpoints.
//!
//! Wired into `dispatch_route` through `dispatch(req, store)`.
//! Endpoints:
//!
//! - `GET /peer/sync` — return the local gateway's recent
//!   sessions + audit records, JSON-encoded. Bearer-gated by
//!   `ARGO_PEER_SECRET`.
//! - `POST /peer/poke` — receive a "something changed locally on
//!   the sender" hint; nudge our poll loop into a fast resync.
//!   Bearer-gated.
//! - `GET /peer/state` — return our cached snapshot of every
//!   configured peer. Local introspection — NOT bearer-gated;
//!   the body never includes secrets.
//!
//! Design rationale lives at
//! `docs/skycot/MULTI_HOST_ORCHESTRATOR.md`.
use super::peer;
use argo_store::SessionStore;
use std::sync::Arc;

const SESSION_LIMIT: u32 = 50;
const AUDIT_LIMIT: i64 = 100;

/// Dispatch the `/peer/*` family. Returns `Some(resp)` on a
/// match, `None` otherwise so the parent dispatcher can fall
/// through. Also calls `peer::ensure_started()` on every match
/// so the background poll loop boots on first /peer/* hit.
pub(super) async fn dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let is_peer_route = req.starts_with("GET /peer/")
        || req.starts_with("POST /peer/")
        || req.starts_with("GET /peer ")
        || req.starts_with("POST /peer ");
    if !is_peer_route {
        return None;
    }
    peer::ensure_started();
    if req.starts_with("GET /peer/sync ") || req.starts_with("GET /peer/sync?") {
        if let Some(resp) = check_peer_auth(req) {
            return Some(resp);
        }
        return Some(sync_response(store).await);
    }
    if req.starts_with("POST /peer/poke ") || req.starts_with("POST /peer/poke?") {
        if let Some(resp) = check_peer_auth(req) {
            return Some(resp);
        }
        return Some(poke_response());
    }
    if req.starts_with("GET /peer/state ") || req.starts_with("GET /peer/state?") {
        return Some(peer_state_response().await);
    }
    Some((
        "404 Not Found",
        r#"{"error":"unknown peer endpoint"}"#.to_string(),
    ))
}

/// Bearer gate for `/peer/sync` and `/peer/poke`. Distinct from
/// `routes_auth::check_bearer_auth` because peers carry their
/// own secret (`ARGO_PEER_SECRET`) — the operator-facing
/// `ARGO_GATEWAY_BEARER_TOKEN` is for human/CLI traffic.
fn check_peer_auth(req: &str) -> Option<(&'static str, String)> {
    if std::env::var("ARGO_GATEWAY_AUTH_BYPASS").ok().as_deref() == Some("1") {
        return None;
    }
    let Some(expected) = peer::configured_secret() else {
        return Some((
            "401 Unauthorized",
            r#"{"error":"unauthorized","reason":"ARGO_PEER_SECRET not configured"}"#
                .to_string(),
        ));
    };
    let presented = presented_bearer(req);
    if constant_time_eq(presented.as_deref().unwrap_or(""), &expected) {
        None
    } else {
        Some((
            "401 Unauthorized",
            r#"{"error":"unauthorized","reason":"invalid peer bearer token"}"#
                .to_string(),
        ))
    }
}

fn presented_bearer(req: &str) -> Option<String> {
    req.lines()
        .find_map(|l| {
            l.split_once(':').and_then(|(k, v)| {
                k.eq_ignore_ascii_case("Authorization").then(|| v.trim())
            })
        })
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::to_string)
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

async fn sync_response(store: Option<&Arc<SessionStore>>) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let now_micros = chrono::Utc::now().timestamp_micros();
    let sessions = match store.list_all_sessions(SESSION_LIMIT).await {
        Ok(rows) => rows,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"sessions: {e}\"}}"),
            );
        }
    };
    let audit = match recent_audit(store, AUDIT_LIMIT).await {
        Ok(rows) => rows,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"audit: {e}\"}}"),
            );
        }
    };
    let body = serde_json::json!({
        "epoch_micros": now_micros,
        "sessions": sessions,
        "audit": audit,
    });
    ("200 OK", body.to_string())
}

#[derive(serde::Serialize)]
struct AuditWire {
    seq: i64,
    kind: String,
    payload_json: String,
    created_at: i64,
}

async fn recent_audit(
    store: &Arc<SessionStore>,
    limit: i64,
) -> Result<Vec<AuditWire>, String> {
    let rows: Vec<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT seq, kind, payload_json, created_at FROM audit_record \
         ORDER BY seq DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(store.pool())
    .await
    .map_err(|e| format!("query: {e}"))?;
    Ok(rows
        .into_iter()
        .map(|(seq, kind, payload_json, created_at)| AuditWire {
            seq,
            kind,
            payload_json,
            created_at,
        })
        .collect())
}

fn poke_response() -> (&'static str, String) {
    peer::nudge();
    ("200 OK", r#"{"status":"poked"}"#.to_string())
}

async fn peer_state_response() -> (&'static str, String) {
    let snapshots = peer::snapshot_all().await;
    let body = serde_json::json!({
        "self_peers": peer::configured_peers(),
        "peers": snapshots,
    });
    ("200 OK", body.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presented_bearer_extracts_token_case_insensitively() {
        let req = "POST /peer/poke HTTP/1.1\r\n\
                   authorization: Bearer secret-tok\r\n\
                   Content-Length: 0\r\n\r\n";
        assert_eq!(presented_bearer(req).as_deref(), Some("secret-tok"));
    }

    #[test]
    fn presented_bearer_returns_none_without_header() {
        let req = "POST /peer/poke HTTP/1.1\r\nContent-Length: 0\r\n\r\n";
        assert!(presented_bearer(req).is_none());
    }

    #[test]
    fn constant_time_eq_matches_only_on_full_equality() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "abcd"));
        assert!(!constant_time_eq("", "x"));
        assert!(constant_time_eq("", ""));
    }
}
