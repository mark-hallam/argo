//! `POST /sessions/<id>/approve` and `POST /sessions/<id>/revoke-approval`
//! gateway handlers (#665). Mirror of the CLI surface from
//! #664 so non-CLI clients can drive the escalation grant
//! flow without spawning a process per call.
//!
//! Body shape (both endpoints):
//!
//! ```json
//! { "tool": "delegate_task", "note": "approved by mark" }
//! ```
//!
//! `note` is optional; the rest of the contract matches the
//! CLI verbatim — same audit kinds, same SQLite write path,
//! same precedence rules.
use super::{body_from_request, store_unavailable};
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;
use uuid::Uuid;

/// `POST /sessions/<id>/approve`. Returns 200 + JSON
/// `{ok, grant_id, session_id, tool}` on success; 400 / 404 /
/// 503 on the failure modes documented inline.
pub(super) async fn session_approve_response(
    sid_str: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let Ok(sid) = parse_sid(sid_str) else {
        return bad_request(&format!("invalid session id '{sid_str}'"));
    };
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse JSON body: {e}")),
    };
    let Some(tool) = parsed.get("tool").and_then(|v| v.as_str()) else {
        return bad_request("missing `tool`");
    };
    let note = parsed.get("note").and_then(|v| v.as_str());
    // `one_shot` (#667) — boolean field; absent or false means
    // persistent grant, same as #664 default.
    let one_shot = parsed
        .get("one_shot")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Fail loud on ghost sessions — same UX as the CLI.
    let exists = match store.get_session(sid).await {
        Ok(opt) => opt.is_some(),
        Err(e) => return server_error(&format!("get_session: {e}")),
    };
    if !exists {
        return (
            "404 Not Found",
            format!("{{\"error\":\"no session with id '{sid_str}'\"}}"),
        );
    }

    let grant_id = match store
        .add_session_grant(sid, tool, "gateway", note, one_shot)
        .await
    {
        Ok(id) => id,
        Err(e) => return server_error(&format!("add_session_grant: {e}")),
    };
    let audit = AuditSink::new(store.clone());
    let payload = serde_json::json!({
        "tool": tool,
        "grant_id": grant_id.to_string(),
        "source": "gateway",
        "note": note,
        "one_shot": one_shot,
    });
    if let Err(e) = audit
        .write(Some(sid), AuditKind::ApprovalGranted, payload)
        .await
    {
        return server_error(&format!("audit write: {e}"));
    }

    (
        "200 OK",
        serde_json::to_string(&serde_json::json!({
            "ok": true,
            "grant_id": grant_id.to_string(),
            "session_id": sid_str,
            "tool": tool,
            "one_shot": one_shot,
        }))
        .unwrap_or_default(),
    )
}

/// `POST /sessions/<id>/revoke-approval`. Returns 200 + JSON
/// `{ok, removed, session_id, tool}`. `removed=0` when the
/// (session, tool) pair had no grants — a no-op revoke is not
/// an error.
pub(super) async fn session_revoke_approval_response(
    sid_str: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let Ok(sid) = parse_sid(sid_str) else {
        return bad_request(&format!("invalid session id '{sid_str}'"));
    };
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse JSON body: {e}")),
    };
    let Some(tool) = parsed.get("tool").and_then(|v| v.as_str()) else {
        return bad_request("missing `tool`");
    };

    let removed = match store.revoke_session_grants_for_tool(sid, tool).await {
        Ok(n) => n,
        Err(e) => return server_error(&format!("revoke_session_grants_for_tool: {e}")),
    };
    let audit = AuditSink::new(store.clone());
    let payload = serde_json::json!({
        "tool": tool,
        "removed": removed,
        "source": "gateway",
    });
    if let Err(e) = audit
        .write(Some(sid), AuditKind::ApprovalRevoked, payload)
        .await
    {
        return server_error(&format!("audit write: {e}"));
    }
    (
        "200 OK",
        serde_json::to_string(&serde_json::json!({
            "ok": true,
            "removed": removed,
            "session_id": sid_str,
            "tool": tool,
        }))
        .unwrap_or_default(),
    )
}

/// `GET /sessions/<id>/grants` — list every active human-
/// approval grant for the session (#666). Same JSON shape as
/// the CLI's `--json` mode: `{session_id, count, grants:[…]}`.
pub(super) async fn session_grants_get_response(
    sid_str: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let Ok(sid) = parse_sid(sid_str) else {
        return bad_request(&format!("invalid session id '{sid_str}'"));
    };
    let exists = match store.get_session(sid).await {
        Ok(opt) => opt.is_some(),
        Err(e) => return server_error(&format!("get_session: {e}")),
    };
    if !exists {
        return (
            "404 Not Found",
            format!("{{\"error\":\"no session with id '{sid_str}'\"}}"),
        );
    }
    let grants = match store.list_session_grants(sid).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("list_session_grants: {e}")),
    };
    let arr: Vec<serde_json::Value> = grants
        .iter()
        .map(|g| {
            serde_json::json!({
                "id": g.id.to_string(),
                "tool": g.tool_name,
                "granted_at_secs": g.granted_at_secs,
                "source": g.source,
                "note": g.notes,
                "one_shot": g.is_one_shot,
            })
        })
        .collect();
    (
        "200 OK",
        serde_json::to_string(&serde_json::json!({
            "session_id": sid_str,
            "count": arr.len(),
            "grants": arr,
        }))
        .unwrap_or_default(),
    )
}

fn parse_sid(s: &str) -> Result<SessionId, ()> {
    let s = s.trim_end_matches('/');
    Uuid::parse_str(s).map(SessionId).map_err(|_| ())
}

fn bad_request(msg: &str) -> (&'static str, String) {
    ("400 Bad Request", format!("{{\"error\":\"{msg}\"}}"))
}

fn server_error(msg: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        format!("{{\"error\":\"{msg}\"}}"),
    )
}
