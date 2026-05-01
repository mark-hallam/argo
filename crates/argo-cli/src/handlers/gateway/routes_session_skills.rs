//! Per-session skill attach / detach / list handlers + the
//! session-delete endpoint. Lives in its own module so
//! `routes_post_admin.rs` (memory + skill lock/unlock/forget/
//! edit) stays under the 500-LOC ceiling.
use super::{body_from_request, store_unavailable};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

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

/// `GET /sessions/<id>/skills` — list the skills attached to
/// the session. Empty array means the runner falls back to all
/// active skills for this session.
pub(super) async fn session_skills_list_response(
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
    let skills = SkillStore::new(Arc::clone(store));
    match skills.skills_for_session(sid).await {
        Ok(rows) => {
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
        format!(
            "{{\"cleared\":true,\"session\":\"{raw_sid}\",\"detached\":{detached}}}"
        ),
    )
}

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

fn parse_skill_field(req: &str) -> Result<String, (&'static str, String)> {
    let parsed: serde_json::Value = serde_json::from_str(body_from_request(req))
        .map_err(|e| ("400 Bad Request", format!("{{\"error\":\"parse body: {e}\"}}")))?;
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
