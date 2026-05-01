//! Admin-flavoured POST handlers — lock / unlock endpoints for
//! both memory entries and skills. Lives in its own module so
//! `routes_post_writes.rs` (which already hosts skill install,
//! memory append, and the forge POSTs) stays under the 500-LOC
//! ceiling.
//!
//! All four endpoints pass `PrincipalOverride::Force` to the
//! underlying store API — the gateway acts as the user, not as
//! an agent. A future tier of RBAC tokens may narrow this.
use super::{body_from_request, store_unavailable};
use argo_skill::{PrincipalOverride as SkillPrincipal, SkillStore};
use argo_store::SessionStore;
use argo_types::Lock;
use std::sync::Arc;

/// `POST /memory/<id>/lock` — apply a lock to a memory entry.
/// Body shape: `{"lock":"user-protected"|"content-locked"}`
/// (defaults to `user-protected` when the body is empty / lock
/// missing). Returns 200 + `{locked, id, lock}`. 400 on invalid
/// id / unknown lock label.
pub(super) async fn memory_lock_response(
    raw_id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(raw_id) {
        Ok(u) => u,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid memory id: {e}\"}}"),
            );
        }
    };
    let lock = match parse_lock_from_body(req, Lock::UserProtected) {
        Ok(Lock::None) => {
            return (
                "400 Bad Request",
                "{\"error\":\"`lock=none` not allowed on /lock — use /unlock instead\"}"
                    .to_string(),
            );
        }
        Ok(l) => l,
        Err(e) => return e,
    };
    let memory = argo_memory::MemoryStore::new(Arc::clone(store));
    match memory
        .set_lock(id, lock, argo_memory::PrincipalOverride::Force)
        .await
    {
        Ok(()) => (
            "200 OK",
            format!(
                "{{\"locked\":true,\"id\":\"{raw_id}\",\"lock\":\"{}\"}}",
                lock_label(lock)
            ),
        ),
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"set_lock: {e}\"}}"),
        ),
    }
}

/// `POST /memory/<id>/forget` — revert a memory entry to the
/// previous head (or no-op when this is already the head).
/// Mirrors `argo memory forget <id>` which is itself a thin
/// alias for `revert`. Returns 200 + the new entry's id; 400
/// on bad uuid or unknown entry.
pub(super) async fn memory_forget_response(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(raw_id) {
        Ok(u) => u,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid memory id: {e}\"}}"),
            );
        }
    };
    let memory = argo_memory::MemoryStore::new(Arc::clone(store));
    match memory.revert(id, argo_memory::PrincipalOverride::Force).await {
        Ok(entry) => (
            "200 OK",
            format!(
                "{{\"forgotten\":true,\"id\":\"{raw_id}\",\"new_head\":\"{}\"}}",
                entry.id
            ),
        ),
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"forget: {e}\"}}"),
        ),
    }
}

/// `POST /memory/<id>/unlock` — clear the lock. Returns 200 +
/// `{unlocked, id}`. 400 on invalid id; underlying delete is
/// idempotent (no-op when no row exists).
pub(super) async fn memory_unlock_response(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(raw_id) {
        Ok(u) => u,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid memory id: {e}\"}}"),
            );
        }
    };
    let memory = argo_memory::MemoryStore::new(Arc::clone(store));
    match memory
        .set_lock(id, Lock::None, argo_memory::PrincipalOverride::Force)
        .await
    {
        Ok(()) => (
            "200 OK",
            format!("{{\"unlocked\":true,\"id\":\"{raw_id}\"}}"),
        ),
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"unlock: {e}\"}}"),
        ),
    }
}

/// `POST /skills/<name>/lock` — apply a lock to the active
/// version of a named skill. Body shape mirrors memory's lock.
pub(super) async fn skill_lock_response(
    name: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let lock = match parse_lock_from_body(req, Lock::UserProtected) {
        Ok(Lock::None) => {
            return (
                "400 Bad Request",
                "{\"error\":\"`lock=none` not allowed on /lock — use /unlock instead\"}"
                    .to_string(),
            );
        }
        Ok(l) => l,
        Err(e) => return e,
    };
    let skills = SkillStore::new(Arc::clone(store));
    let skill = match skills.find_active_by_name(name).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no active skill named '{name}'\"}}"),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"find_active_by_name: {e}\"}}"),
            );
        }
    };
    match skills.set_lock(skill.id, lock, SkillPrincipal::Force).await {
        Ok(()) => (
            "200 OK",
            format!(
                "{{\"locked\":true,\"name\":\"{name}\",\"skill_id\":\"{}\",\"lock\":\"{}\"}}",
                skill.id.0,
                lock_label(lock)
            ),
        ),
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"set_lock: {e}\"}}"),
        ),
    }
}

/// `POST /skills/<name>/unlock` — clear the lock on the
/// currently-active version of `<name>`. Idempotent.
pub(super) async fn skill_unlock_response(
    name: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let skills = SkillStore::new(Arc::clone(store));
    let skill = match skills.find_active_by_name(name).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                "404 Not Found",
                format!("{{\"error\":\"no active skill named '{name}'\"}}"),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"find_active_by_name: {e}\"}}"),
            );
        }
    };
    match skills
        .set_lock(skill.id, Lock::None, SkillPrincipal::Force)
        .await
    {
        Ok(()) => (
            "200 OK",
            format!(
                "{{\"unlocked\":true,\"name\":\"{name}\",\"skill_id\":\"{}\"}}",
                skill.id.0
            ),
        ),
        Err(e) => (
            "400 Bad Request",
            format!("{{\"error\":\"unlock: {e}\"}}"),
        ),
    }
}

fn lock_label(lock: Lock) -> &'static str {
    match lock {
        Lock::None => "none",
        Lock::UserProtected => "user-protected",
        Lock::ContentLocked => "content-locked",
    }
}

/// Parse `{"lock":"<label>"}` from the request body. Empty body
/// or missing field → `default`.
fn parse_lock_from_body(req: &str, default: Lock) -> Result<Lock, (&'static str, String)> {
    let body = body_from_request(req);
    if body.trim().is_empty() {
        return Ok(default);
    }
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| ("400 Bad Request", format!("{{\"error\":\"parse JSON body: {e}\"}}")))?;
    match v.get("lock").and_then(|x| x.as_str()) {
        None => Ok(default),
        Some("none") => Ok(Lock::None),
        Some("user-protected") => Ok(Lock::UserProtected),
        Some("content-locked") => Ok(Lock::ContentLocked),
        Some(other) => Err((
            "400 Bad Request",
            format!(
                "{{\"error\":\"unknown lock '{other}' — use \
                 user-protected|content-locked|none\"}}"
            ),
        )),
    }
}
