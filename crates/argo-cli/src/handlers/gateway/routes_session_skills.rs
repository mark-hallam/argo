//! Per-session skill attach / detach / list / clear handlers.
//! Session-lifecycle handlers (end / end-all / reopen / title /
//! delete) live in the sibling `routes_session_skills_lifecycle`
//! to keep this file under the 500-LOC ceiling, and are
//! re-exported below so the dispatcher import surface stays
//! unchanged.
use super::tenant_header::{assert_session_belongs_to_tenant, request_tenant_id};
use super::{body_from_request, store_unavailable};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

pub(super) use super::routes_session_skills_lifecycle::{
    session_delete_response, session_end_all_response, session_end_response,
    session_reopen_response, session_set_title_response,
};

/// `POST /sessions/<id>/skills/attach` — attach an active
/// skill to a session. Body shape: `{"skill":"<name>"}`.
/// Returns 200 + `{attached, session, skill, skill_id}` on
/// success; 400 on bad uuid or missing skill name; 404 when
/// no active skill matches the given name.
pub(super) async fn session_attach_skill_response(
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
    let tenant_id = request_tenant_id(req);
    if let Err(resp) = assert_session_belongs_to_tenant(store, sid, raw_sid, &tenant_id).await {
        return resp;
    }
    let skill_name = match parse_skill_field(req) {
        Ok(s) => s,
        Err(resp) => return resp,
    };
    let skills = SkillStore::new(Arc::clone(store));
    let skill = match skills.find_active_by_name(&skill_name).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no active skill named '{skill_name}'\"}}"),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"find_active_by_name: {e}\"}}"),
            );
        }
    };
    match skills.attach_to_session(sid, skill.id).await {
        Ok(()) => (
            "200 OK",
            format!(
                "{{\"attached\":true,\"session\":\"{raw_sid}\",\"skill\":\"{skill_name}\",\"skill_id\":\"{}\"}}",
                skill.id.0
            ),
        ),
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"attach_to_session: {e}\"}}"),
        ),
    }
}

/// `POST /sessions/<id>/skills/detach` — detach a skill from
/// a session. Body shape: `{"skill":"<name>"}`. Idempotent —
/// 200 even when the (session, skill) pair wasn't attached.
/// 400 on bad uuid or missing skill name; 404 when the skill
/// name doesn't resolve.
pub(super) async fn session_detach_skill_response(
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
    let tenant_id = request_tenant_id(req);
    if let Err(resp) = assert_session_belongs_to_tenant(store, sid, raw_sid, &tenant_id).await {
        return resp;
    }
    let skill_name = match parse_skill_field(req) {
        Ok(s) => s,
        Err(resp) => return resp,
    };
    let skills = SkillStore::new(Arc::clone(store));
    let skill = match skills.find_active_by_name(&skill_name).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no active skill named '{skill_name}'\"}}"),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"find_active_by_name: {e}\"}}"),
            );
        }
    };
    match skills.detach_from_session(sid, skill.id).await {
        Ok(rows) => (
            "200 OK",
            format!(
                "{{\"detached\":{},\"session\":\"{raw_sid}\",\"skill\":\"{skill_name}\"}}",
                rows > 0
            ),
        ),
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"detach_from_session: {e}\"}}"),
        ),
    }
}

/// `GET /sessions/<id>/skills[?format=markdown]` — list the
/// skills attached to the session. Empty array (or empty
/// markdown table) means the runner falls back to all active
/// skills for this session.
pub(super) async fn session_skills_list_response(
    raw_sid: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
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
    let tenant_id = request_tenant_id(req);
    if let Err(resp) = assert_session_belongs_to_tenant(store, sid, raw_sid, &tenant_id).await {
        return resp;
    }
    let skills = SkillStore::new(Arc::clone(store));
    match skills.skills_for_session(sid).await {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                let mut out =
                    format!("# Skills attached to session `{raw_sid}` ({n} attached)\n\n",);
                if rows.is_empty() {
                    out.push_str(
                        "_No skills attached — runner falls back to every active skill in the workspace._\n",
                    );
                } else {
                    out.push_str("| name | version |\n");
                    out.push_str("| --- | --- |\n");
                    for s in &rows {
                        out.push_str(&format!(
                            "| `{}` | `{}` |\n",
                            s.frontmatter.name, s.frontmatter.version
                        ));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "name": s.frontmatter.name,
                        "version": s.frontmatter.version,
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"skills_for_session: {e}\"}}"),
        ),
    }
}

/// `POST /sessions/<id>/skills/clear` — detach every attached
/// skill from a session. Returns 200 + `{cleared, session,
/// detached: <count>}`. Useful for "reset to fall-through-all-
/// active-skills" without pre-listing the attached set. 400
/// on bad uuid.
pub(super) async fn session_clear_skills_response(
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
    let tenant_id = request_tenant_id(req);
    if let Err(resp) = assert_session_belongs_to_tenant(store, sid, raw_sid, &tenant_id).await {
        return resp;
    }
    let skills = SkillStore::new(Arc::clone(store));
    let attached = match skills.skills_for_session(sid).await {
        Ok(rows) => rows,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"skills_for_session: {e}\"}}"),
            );
        }
    };
    let mut detached: u64 = 0;
    for s in attached {
        match skills.detach_from_session(sid, s.id).await {
            Ok(n) => detached += n,
            Err(e) => {
                return (
                    "500 Internal Server Error",
                    format!("{{\"error\":\"detach_from_session: {e}\"}}"),
                );
            }
        }
    }
    (
        "200 OK",
        format!("{{\"cleared\":true,\"session\":\"{raw_sid}\",\"detached\":{detached}}}"),
    )
}

fn parse_skill_field(req: &str) -> Result<String, (&'static str, String)> {
    let parsed: serde_json::Value = serde_json::from_str(body_from_request(req)).map_err(|e| {
        (
            "400 Bad Request",
            format!("{{\"error\":\"parse body: {e}\"}}"),
        )
    })?;
    parsed
        .get("skill")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .ok_or((
            "400 Bad Request",
            "{\"error\":\"missing or empty `skill`\"}".to_string(),
        ))
}

#[cfg(test)]
mod tests {
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
             VALUES (?, ?, 'cluster6', '.', strftime('%s','now'))",
        )
        .bind(workspace.0.as_bytes().as_slice())
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .expect("seed workspace");
        let session = store
            .create_session(tenant, workspace, "m", "p", Some("cluster6"), None)
            .await
            .expect("create session");
        (tenant, session)
    }

    fn req_with_tenant(path: &str, tenant: &TenantId, body: &str) -> String {
        format!(
            "POST {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Tenant-Id: {}\r\n\
             Content-Length: {}\r\n\r\n{body}",
            tenant.0,
            body.len()
        )
    }

    fn skill_doc(name: &str) -> String {
        format!(
            "---\nname: {name}\nversion: \"0.1\"\ndescription: d\n\
             lock: none\nprovenance:\n  source: test\n  hash: sha256-x\n---\nbody"
        )
    }

    #[tokio::test]
    async fn attach_skill_matching_tenant_inserts_session_skill() {
        let store = mem_store().await;
        let (tenant, session) = seed_session(&store).await;
        SkillStore::new(Arc::clone(&store))
            .install(&skill_doc("alpha"), None)
            .await
            .expect("install skill");
        let sid = session.0.to_string();
        let req = req_with_tenant(
            &format!("/sessions/{sid}/skills/attach"),
            &tenant,
            r#"{"skill":"alpha"}"#,
        );

        let (status, body) = session_attach_skill_response(&sid, &req, Some(&store)).await;

        assert!(status.starts_with("200"), "status={status} body={body}");
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM session_skill WHERE session_id = ?")
                .bind(session.0.as_bytes().as_slice())
                .fetch_one(store.pool())
                .await
                .expect("count session_skill");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn attach_skill_cross_tenant_returns_404_and_inserts_nothing() {
        let store = mem_store().await;
        let (_tenant, session) = seed_session(&store).await;
        SkillStore::new(Arc::clone(&store))
            .install(&skill_doc("alpha"), None)
            .await
            .expect("install skill");
        let sid = session.0.to_string();
        let other_tenant = TenantId::new();
        let req = req_with_tenant(
            &format!("/sessions/{sid}/skills/attach"),
            &other_tenant,
            r#"{"skill":"alpha"}"#,
        );

        let (status, _body) = session_attach_skill_response(&sid, &req, Some(&store)).await;

        assert!(status.starts_with("404"), "expected 404, got {status}");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session_skill")
            .fetch_one(store.pool())
            .await
            .expect("count session_skill");
        assert_eq!(count, 0);
    }
}
