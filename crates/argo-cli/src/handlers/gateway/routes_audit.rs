//! Audit-related GET handlers, extracted from `routes_post.rs`
//! to keep that file under the 500-LOC budget. Pure read-side —
//! no chain mutation here. The dispatcher in `routes.rs` still
//! owns URL parsing.

use super::store_unavailable;
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

/// `GET /audit/recent?limit=N&kind=<kind>` — most recent N
/// audit rows across every session, in seq order (oldest first
/// inside the window). Mirrors `argo audit show --tail N`.
/// Defaults to 50. With `kind=<kind>` (e.g. `turn_start`,
/// `assistant_message`, `tool_call`), the WHERE-clause runs
/// before the LIMIT so the caller gets up to N rows of that kind.
pub(super) async fn audit_recent_response(
    limit: u32,
    kind: Option<&str>,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    match argo_audit::tail_rows_filtered(store, limit, kind).await {
        Ok(rows) => (
            "200 OK",
            serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"audit tail: {e}\"}}"),
        ),
    }
}

/// `GET /audit/sessions/<id>/verify` — verify the audit chain
/// for one session and return `{ok, rows, breaks}`. Mirrors
/// `argo audit verify --session <id>`. 200 in both the
/// chain-ok and chain-broken cases — `ok` says which.
pub(super) async fn audit_session_verify_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    match argo_audit::verify_session(store, id).await {
        Ok(report) => {
            let payload = serde_json::json!({
                "ok": report.ok,
                "rows": report.rows,
                "breaks": report.breaks.len(),
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"verify_session: {e}\"}}"),
        ),
    }
}

/// `GET /audit/sessions/<id>?kind=<kind>&since-minutes=<m>` —
/// list audit rows scoped to one session, in `seq` order.
/// Mirrors `argo audit show --session <id>`. Filters apply
/// after fetching the full session window (sessions are
/// typically small, so this keeps the SQL trivial).
pub(super) async fn audit_session_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    kind: Option<&str>,
    since_minutes: Option<i64>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    match argo_audit::rows_for_session(store, id).await {
        Ok(mut rows) => {
            if let Some(want) = kind {
                rows.retain(|r| r.kind == want);
            }
            if let Some(m) = since_minutes {
                let cutoff_micros = (chrono::Utc::now() - chrono::Duration::minutes(m))
                    .timestamp_micros();
                rows.retain(|r| r.created_at_micros >= cutoff_micros);
            }
            (
                "200 OK",
                serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"audit session: {e}\"}}"),
        ),
    }
}
