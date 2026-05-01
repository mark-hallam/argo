//! Bulk-list GET handlers — `/skills` and `/memory`. Lifted
//! out of `routes.rs` so the dispatcher file stays under the
//! 500-LOC ceiling.
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /skills[?include-superseded=true]` — active skills by
/// default, all versions when the flag is set. Each row carries
/// `{id, name, version, lock, hash, created_at, superseded_by?}`.
pub(super) async fn skills_response(
    store: Option<&Arc<SessionStore>>,
    include_superseded: bool,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    let result = if include_superseded {
        skills.list_all().await
    } else {
        skills.list_active().await
    };
    match result {
        Ok(rows) => {
            let arr: Vec<serde_json::Value> = rows
                .iter()
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

/// `GET /memory` — every active memory entry across the four
/// canonical files (MEMORY / USER / SOUL / AGENTS) for the
/// first registered workspace. 200 + `[]` when no workspace
/// is registered.
pub(super) async fn memory_response(
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    use argo_memory::{MemoryFile, MemoryStore};
    let Some(store) = store else {
        return store_unavailable();
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
    let memory = MemoryStore::new(Arc::clone(store));
    let mut all: Vec<serde_json::Value> = Vec::new();
    for f in [
        MemoryFile::Memory,
        MemoryFile::User,
        MemoryFile::Soul,
        MemoryFile::Agents,
    ] {
        match memory.list_active(workspace, f).await {
            Ok(entries) => {
                for e in entries {
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
