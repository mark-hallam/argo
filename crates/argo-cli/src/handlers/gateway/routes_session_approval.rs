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
use super::routes_auth_principal::{principal_id_for_audit, principal_payload};
use super::tenant_header::{assert_session_belongs_to_tenant, request_tenant_id};
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
    // Tenant guard (MF-2): reject cross-tenant writes before any
    // unscoped read/write. Mirrors the sibling
    // `routes_session_skills_lifecycle.rs` pattern verbatim.
    if let Err(resp) =
        assert_session_belongs_to_tenant(store, sid, sid_str, &request_tenant_id(req)).await
    {
        return resp;
    }
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
    let session = match store.get_session(sid).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no session with id '{sid_str}'\"}}"),
            );
        }
        Err(e) => return server_error(&format!("get_session: {e}")),
    };

    // Per-target scope (`target_glob`) is not yet exposed on the
    // gateway body — `None` preserves the v7/v8 session-global
    // behaviour. A follow-up brief lifts it through the JSON body
    // alongside the existing `tool`/`note`/`one_shot` fields.
    let grant_id = match store
        .add_session_grant(sid, tool, "gateway", note, one_shot, None)
        .await
    {
        Ok(id) => id,
        Err(e) => return server_error(&format!("add_session_grant: {e}")),
    };
    let audit = AuditSink::new_with_tenant(store.clone(), session.tenant_id.to_string());
    let payload = serde_json::json!({
        "tool": tool,
        "grant_id": grant_id.to_string(),
        "source": "gateway",
        "note": note,
        "one_shot": one_shot,
    });
    let payload = match principal_payload(req, payload) {
        Ok(payload) => payload,
        Err(_) => return bad_request("bad principal"),
    };
    let principal_id = principal_id_for_audit(req);
    if let Err(e) = audit
        .write(Some(sid), AuditKind::ApprovalGranted, payload, principal_id)
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
    // Tenant guard (MF-2): reject cross-tenant revokes before any
    // unscoped read/write. Mirrors the sibling
    // `routes_session_skills_lifecycle.rs` pattern verbatim.
    if let Err(resp) =
        assert_session_belongs_to_tenant(store, sid, sid_str, &request_tenant_id(req)).await
    {
        return resp;
    }
    let body = body_from_request(req);
    let parsed: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse JSON body: {e}")),
    };
    let Some(tool) = parsed.get("tool").and_then(|v| v.as_str()) else {
        return bad_request("missing `tool`");
    };

    let session = match store.get_session(sid).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no session with id '{sid_str}'\"}}"),
            );
        }
        Err(e) => return server_error(&format!("get_session: {e}")),
    };
    let removed = match store.revoke_session_grants_for_tool(sid, tool).await {
        Ok(n) => n,
        Err(e) => return server_error(&format!("revoke_session_grants_for_tool: {e}")),
    };
    let audit = AuditSink::new_with_tenant(store.clone(), session.tenant_id.to_string());
    let payload = serde_json::json!({
        "tool": tool,
        "removed": removed,
        "source": "gateway",
    });
    let payload = match principal_payload(req, payload) {
        Ok(payload) => payload,
        Err(_) => return bad_request("bad principal"),
    };
    let principal_id = principal_id_for_audit(req);
    if let Err(e) = audit
        .write(Some(sid), AuditKind::ApprovalRevoked, payload, principal_id)
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
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let Ok(sid) = parse_sid(sid_str) else {
        return bad_request(&format!("invalid session id '{sid_str}'"));
    };
    // Tenant guard (MF-2): cross-tenant reads return 404 (the same
    // shape as a non-existent session) instead of leaking another
    // tenant's grant list. Mirrors the sibling
    // `routes_session_skills_lifecycle.rs` pattern verbatim.
    if let Err(resp) =
        assert_session_belongs_to_tenant(store, sid, sid_str, &request_tenant_id(req)).await
    {
        return resp;
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

#[cfg(test)]
mod tests {
    //! MF-2 regression: cross-tenant session-approval writes/reads.
    //! Before the fix all three handlers called unscoped
    //! `store.get_session(sid)` and returned 200 for a request
    //! carrying a *different* tenant's `X-Argo-Tenant-Id` header —
    //! letting tenant B grant/revoke/list approvals on tenant A's
    //! session. These tests would FAIL (return 200) before the
    //! `assert_session_belongs_to_tenant` guard was added.
    use super::*;
    use argo_types::{TenantId, WorkspaceId};

    async fn mem_store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.expect("open mem db");
        store.migrate().await.expect("migrate");
        Arc::new(store)
    }

    async fn seed_session(store: &Arc<SessionStore>) -> (TenantId, SessionId) {
        let tenant = TenantId::new();
        let workspace = WorkspaceId::new();
        sqlx::query(
            "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
             VALUES (?, ?, 'mf2-approval', '.', strftime('%s','now'))",
        )
        .bind(workspace.0.as_bytes().as_slice())
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .expect("seed workspace");
        let session = store
            .create_session(tenant, workspace, "m", "p", Some("mf2-approval"), None)
            .await
            .expect("create session");
        (tenant, session)
    }

    fn req_with_tenant(method: &str, path: &str, tenant: &TenantId, body: &str) -> String {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {}\r\nContent-Length: {}\r\n\r\n{body}",
            tenant.0,
            body.len()
        )
    }

    #[tokio::test]
    async fn approve_rejects_cross_tenant_and_writes_no_grant() {
        let store = mem_store().await;
        let (_tenant_a, session) = seed_session(&store).await;
        let sid = session.0.to_string();
        // Tenant B (a *different* tenant) tries to grant on A's session.
        let tenant_b = TenantId::new();
        let req = req_with_tenant(
            "POST",
            &format!("/sessions/{sid}/approve"),
            &tenant_b,
            "{\"tool\":\"delegate_task\"}",
        );

        let (status, body) = session_approve_response(&sid, &req, Some(&store)).await;

        assert!(
            !status.starts_with("200"),
            "cross-tenant approve must NOT succeed, got status={status} body={body}"
        );
        assert!(status.starts_with("404"), "expected 404, got {status}");
        // No grant should have been written to A's session.
        let grants = store
            .list_session_grants(session)
            .await
            .expect("list grants");
        assert!(
            grants.is_empty(),
            "cross-tenant approve leaked a grant: {grants:?}"
        );
    }

    #[tokio::test]
    async fn approve_allows_matching_tenant() {
        let store = mem_store().await;
        let (tenant_a, session) = seed_session(&store).await;
        let sid = session.0.to_string();
        let req = req_with_tenant(
            "POST",
            &format!("/sessions/{sid}/approve"),
            &tenant_a,
            "{\"tool\":\"delegate_task\"}",
        );

        let (status, body) = session_approve_response(&sid, &req, Some(&store)).await;

        assert!(
            status.starts_with("200"),
            "matching-tenant approve must succeed, got status={status} body={body}"
        );
        let grants = store
            .list_session_grants(session)
            .await
            .expect("list grants");
        assert_eq!(grants.len(), 1, "matching-tenant approve should grant once");
    }

    #[tokio::test]
    async fn revoke_rejects_cross_tenant() {
        let store = mem_store().await;
        let (tenant_a, session) = seed_session(&store).await;
        let sid = session.0.to_string();
        // A real grant exists on A's session.
        store
            .add_session_grant(session, "delegate_task", "gateway", None, false, None)
            .await
            .expect("seed grant");
        let tenant_b = TenantId::new();
        let req = req_with_tenant(
            "POST",
            &format!("/sessions/{sid}/revoke-approval"),
            &tenant_b,
            "{\"tool\":\"delegate_task\"}",
        );

        let (status, body) =
            session_revoke_approval_response(&sid, &req, Some(&store)).await;

        assert!(
            !status.starts_with("200"),
            "cross-tenant revoke must NOT succeed, got status={status} body={body}"
        );
        assert!(status.starts_with("404"), "expected 404, got {status}");
        // A's grant must still be present.
        let grants = store
            .list_session_grants(session)
            .await
            .expect("list grants");
        assert_eq!(
            grants.len(),
            1,
            "cross-tenant revoke must not delete A's grant"
        );
        // Sanity: tenant A *can* revoke it.
        let req_a = req_with_tenant(
            "POST",
            &format!("/sessions/{sid}/revoke-approval"),
            &tenant_a,
            "{\"tool\":\"delegate_task\"}",
        );
        let (status_a, _) = session_revoke_approval_response(&sid, &req_a, Some(&store)).await;
        assert!(status_a.starts_with("200"), "owner revoke should succeed");
    }

    #[tokio::test]
    async fn grants_get_rejects_cross_tenant() {
        let store = mem_store().await;
        let (tenant_a, session) = seed_session(&store).await;
        let sid = session.0.to_string();
        store
            .add_session_grant(session, "delegate_task", "gateway", None, false, None)
            .await
            .expect("seed grant");
        let tenant_b = TenantId::new();
        let req = req_with_tenant("GET", &format!("/sessions/{sid}/grants"), &tenant_b, "");

        let (status, body) = session_grants_get_response(&sid, &req, Some(&store)).await;

        assert!(
            !status.starts_with("200"),
            "cross-tenant grants-list must NOT succeed, got status={status} body={body}"
        );
        assert!(status.starts_with("404"), "expected 404, got {status}");
        // The owning tenant still sees the grant.
        let req_a = req_with_tenant("GET", &format!("/sessions/{sid}/grants"), &tenant_a, "");
        let (status_a, body_a) = session_grants_get_response(&sid, &req_a, Some(&store)).await;
        assert!(status_a.starts_with("200"), "owner list should succeed");
        assert!(body_a.contains("delegate_task"), "owner should see grant");
    }
}
