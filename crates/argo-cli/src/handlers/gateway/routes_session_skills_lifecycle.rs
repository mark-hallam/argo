//! Session-lifecycle endpoints (end, end-all, reopen, rename,
//! delete) split out of `routes_session_skills.rs` to keep that
//! file under the 500-LOC ceiling. The sibling
//! `routes_session_skills.rs` re-exports each handler so
//! `routes_dispatch_post.rs` imports stay unchanged.
use super::{body_from_request, store_unavailable};
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

/// `POST /sessions/end-all[?older-than-minutes=N]` — mark
/// open sessions (ended_at IS NULL) as ended. Without filter,
/// all open sessions get ended (operator emergency stop).
/// With `?older-than-minutes=N`, only sessions whose
/// started_at is older than N minutes ago are ended — useful
/// for periodic cleanup of stale sessions left behind by
/// crashed clients. Returns 200 + `{ended: N}`.
pub(super) async fn session_end_all_response(
    older_than_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let rows = match store.list_all_sessions(500).await {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"list_all_sessions: {e}\"}}"),
            );
        }
    };
    let cutoff = older_than_minutes
        .map(|m| chrono::Utc::now() - chrono::Duration::minutes(m));
    let mut ended: u64 = 0;
    for s in rows
        .into_iter()
        .filter(|s| s.ended_at.is_none())
        .filter(|s| match cutoff {
            Some(c) => s.started_at < c,
            None => true,
        })
    {
        if let Ok(n) = store.end_session(SessionId(s.id.0)).await {
            ended += n;
        }
    }
    ("200 OK", format!("{{\"ended\":{ended}}}"))
}

/// `POST /sessions/<id>/reopen` — clear `ended_at` back to
/// NULL. Inverse of `/end`. Idempotent. Returns 200 +
/// `{reopened:true, session}`; 400 on bad uuid; 404 when no
/// row matches.
pub(super) async fn session_reopen_response(
    raw_sid: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let sid = match uuid::Uuid::parse_str(raw_sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    match store.reopen_session(sid).await {
        Ok(0) => (
            "404 Not Found",
            format!("{{\"error\":\"no session with id {raw_sid}\"}}"),
        ),
        Ok(_) => (
            "200 OK",
            format!("{{\"reopened\":true,\"session\":\"{raw_sid}\"}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"reopen_session: {e}\"}}"),
        ),
    }
}

/// `POST /sessions/<id>/end` — mark the session ended (stamps
/// `ended_at`). Idempotent. Returns 200 + `{ended:true,
/// session}`; 400 on bad uuid; 404 when no row matches.
pub(super) async fn session_end_response(
    raw_sid: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let sid = match uuid::Uuid::parse_str(raw_sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    match store.end_session(sid).await {
        Ok(0) => (
            "404 Not Found",
            format!("{{\"error\":\"no session with id {raw_sid}\"}}"),
        ),
        Ok(_) => (
            "200 OK",
            format!("{{\"ended\":true,\"session\":\"{raw_sid}\"}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"end_session: {e}\"}}"),
        ),
    }
}

/// `POST /sessions/<id>/title` — rename a session. Body
/// `{title}` (empty title clears it). Returns 200 +
/// `{renamed:true, session, title}`; 400 on bad uuid;
/// 404 when session id has no row.
pub(super) async fn session_set_title_response(
    raw_sid: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let sid = match uuid::Uuid::parse_str(raw_sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    let parsed: serde_json::Value = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse body: {e}\"}}"),
            );
        }
    };
    let title = parsed.get("title").and_then(|v| v.as_str()).unwrap_or("");
    match store.set_session_title(sid, title).await {
        Ok(0) => (
            "404 Not Found",
            format!("{{\"error\":\"no session with id {raw_sid}\"}}"),
        ),
        Ok(_) => (
            "200 OK",
            format!(
                "{{\"renamed\":true,\"session\":\"{raw_sid}\",\"title\":\"{}\"}}",
                title.replace('"', "\\\"")
            ),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"set_session_title: {e}\"}}"),
        ),
    }
}

/// `POST /sessions/<id>/delete` — delete the session row + all
/// dependent rows (messages, audit, spend, judge). Mirrors
/// `argo session delete <id>`. Idempotent — 404 only when the
/// uuid never existed; deleting a deleted id returns the 0-rows
/// case the same way the CLI does.
pub(super) async fn session_delete_response(
    raw_sid: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let sid = match uuid::Uuid::parse_str(raw_sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    match store.delete_session(sid).await {
        Ok(0) => (
            "404 Not Found",
            format!("{{\"error\":\"no session with id {raw_sid}\"}}"),
        ),
        Ok(n) => (
            "200 OK",
            format!("{{\"deleted\":true,\"session\":\"{raw_sid}\",\"rows_affected\":{n}}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"delete_session: {e}\"}}"),
        ),
    }
}
