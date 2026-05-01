//! Bulk-list GET handlers — `/skills` and `/memory`. Lifted
//! out of `routes.rs` so the dispatcher file stays under the
//! 500-LOC ceiling.
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /skills[?include-superseded&name-contains&lock]` —
/// active skills by default, all versions when
/// `include-superseded`. Optional `name-contains` substring
/// (case-insensitive) and `lock` (`none|user-protected|
/// content-locked`) filters. Each row: `{id, name, version,
/// lock, hash, created_at, superseded_by?}`.
pub(super) async fn skills_response(
    store: Option<&Arc<SessionStore>>,
    include_superseded: bool,
    name_contains: Option<&str>,
    lock: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let lock_filter = match lock {
        None => None,
        Some("none") => Some(argo_types::Lock::None),
        Some("user-protected") => Some(argo_types::Lock::UserProtected),
        Some("content-locked") => Some(argo_types::Lock::ContentLocked),
        Some(other) => {
            return (
                "400 Bad Request",
                format!(
                    "{{\"error\":\"unknown lock '{other}' — use none|user-protected|content-locked\"}}"
                ),
            );
        }
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    let result = if include_superseded {
        skills.list_all().await
    } else {
        skills.list_active().await
    };
    let needle = name_contains.map(str::to_lowercase);
    match result {
        Ok(rows) => {
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .filter(|s| match needle.as_deref() {
                    Some(n) => s.frontmatter.name.to_lowercase().contains(n),
                    None => true,
                })
                .filter(|s| match lock_filter {
                    Some(want) => s.frontmatter.lock == want,
                    None => true,
                })
                .map(|s| {
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "name": s.frontmatter.name,
                        "version": s.frontmatter.version,
                        "lock": format!("{:?}", s.frontmatter.lock),
                        "hash": hex::encode(s.hash),
                        "superseded_by": s.superseded_by.map(|i| i.0.to_string()),
                        "created_at": s.created_at.to_rfc3339(),
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
            format!("{{\"error\":\"list skills: {e}\"}}"),
        ),
    }
}

/// `GET /memory/<id>/versions` — supersede chain for a memory
/// entry. Returns every entry in the same `(workspace, file)`
/// scope, oldest first. 200 + JSON array, 400 on bad uuid,
/// 404 when the entry doesn't exist.
pub(super) async fn memory_versions_response(
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
    match memory.versions_for_entry(id).await {
        Ok(rows) => {
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "id": e.id.to_string(),
                        "file": format!("{:?}", e.file),
                        "bytes": e.body.len(),
                        "lock": format!("{:?}", e.lock),
                        "hash": hex::encode(e.hash),
                        "superseded_by": e.superseded_by.map(|i| i.to_string()),
                        "learned_at": e.learned_at.to_rfc3339(),
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "404 Not Found",
            format!("{{\"error\":\"versions_for_entry: {e}\"}}"),
        ),
    }
}

/// `GET /memory[?file&since-minutes&lock]` — active memory
/// entries. `file` scopes to one of the four canonical files.
/// `since-minutes` filters by `learned_at`. `lock` filters by
/// the lock state (`none|user-protected|content-locked`).
/// 200 + `[]` when no workspace is registered; 400 on
/// unknown `file` or `lock`.
pub(super) async fn memory_response(
    store: Option<&Arc<SessionStore>>,
    file: Option<String>,
    since_minutes: Option<i64>,
    lock: Option<String>,
) -> (&'static str, String) {
    use argo_memory::{MemoryFile, MemoryStore};
    let Some(store) = store else {
        return store_unavailable();
    };
    let files: Vec<MemoryFile> = match file.as_deref() {
        None => vec![
            MemoryFile::Memory,
            MemoryFile::User,
            MemoryFile::Soul,
            MemoryFile::Agents,
        ],
        Some("memory") | Some("MEMORY.md") => vec![MemoryFile::Memory],
        Some("user") | Some("USER.md") => vec![MemoryFile::User],
        Some("soul") | Some("SOUL.md") => vec![MemoryFile::Soul],
        Some("agents") | Some("AGENTS.md") => vec![MemoryFile::Agents],
        Some(other) => {
            return (
                "400 Bad Request",
                format!(
                    "{{\"error\":\"unknown file '{other}' — use memory|user|soul|agents\"}}"
                ),
            );
        }
    };
    let workspace = match store.first_workspace().await {
        Ok(Some(w)) => w,
        Ok(None) => return ("200 OK", "[]".to_string()),
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"first_workspace: {e}\"}}"),
            );
        }
    };
    let lock_filter = match lock.as_deref() {
        None => None,
        Some("none") => Some(argo_types::Lock::None),
        Some("user-protected") => Some(argo_types::Lock::UserProtected),
        Some("content-locked") => Some(argo_types::Lock::ContentLocked),
        Some(other) => {
            return (
                "400 Bad Request",
                format!(
                    "{{\"error\":\"unknown lock '{other}' — use none|user-protected|content-locked\"}}"
                ),
            );
        }
    };
    let memory = MemoryStore::new(Arc::clone(store));
    let cutoff = since_minutes
        .map(|m| chrono::Utc::now() - chrono::Duration::minutes(m));
    let mut all: Vec<serde_json::Value> = Vec::new();
    for f in files {
        match memory.list_active(workspace, f).await {
            Ok(entries) => {
                for e in entries {
                    if let Some(c) = cutoff {
                        if e.learned_at < c {
                            continue;
                        }
                    }
                    if let Some(want) = lock_filter {
                        if e.lock != want {
                            continue;
                        }
                    }
                    all.push(serde_json::json!({
                        "id": e.id.to_string(),
                        "file": format!("{:?}", e.file),
                        "bytes": e.body.len(),
                        "lock": format!("{:?}", e.lock),
                        "hash": hex::encode(e.hash),
                        "learned_at": e.learned_at.to_rfc3339(),
                    }));
                }
            }
            Err(e) => {
                return (
                    "500 Internal Server Error",
                    format!("{{\"error\":\"list {f:?}: {e}\"}}"),
                );
            }
        }
    }
    (
        "200 OK",
        serde_json::to_string(&all).unwrap_or_else(|_| "[]".to_string()),
    )
}
