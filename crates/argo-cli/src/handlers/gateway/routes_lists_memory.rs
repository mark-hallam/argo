//! `/memory` family bulk-list handlers (count, versions, list).
//! Lifted out of `routes_lists.rs` to stay under the 500-LOC
//! ceiling; re-exported from `routes_lists` so call sites are
//! unchanged.
use super::store_unavailable;
use crate::home::cwd_workspace;
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /memory/count[?file=…&since-minutes=N]` — count of
/// active memory entries after filters. Mirrors the CLI's
/// `argo memory count`.
pub(super) async fn memory_count_response(
    file: Option<String>,
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
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
                format!("{{\"error\":\"unknown file '{other}' — use memory|user|soul|agents\"}}"),
            );
        }
    };
    // #718: cwd-bound workspace (matches every writer's path).
    let workspace = match cwd_workspace(store).await {
        Ok(w) => w,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd_workspace: {e}\"}}"),
            );
        }
    };
    let memory = MemoryStore::new(Arc::clone(store));
    let cutoff = since_minutes.map(|m| chrono::Utc::now() - chrono::Duration::minutes(m));
    let mut total = 0_usize;
    for f in files {
        if let Ok(entries) = memory.list_active(workspace, f).await {
            if let Some(c) = cutoff {
                total += entries.iter().filter(|e| e.learned_at >= c).count();
            } else {
                total += entries.len();
            }
        }
    }
    let payload = serde_json::json!({
        "count": total,
        "file": file.as_deref().unwrap_or("all"),
    });
    (
        "200 OK",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// `GET /memory/<id>/versions` — supersede chain for a memory
/// entry. Returns every entry in the same `(workspace, file)`
/// scope, oldest first. 200 + JSON array, 400 on bad uuid,
/// 404 when the entry doesn't exist.
pub(super) async fn memory_versions_response(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
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
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                let active = rows
                    .iter()
                    .find(|e| e.superseded_by.is_none())
                    .map(|e| e.id.to_string())
                    .unwrap_or_else(|| "(none)".into());
                let mut out = format!(
                    "# Memory entry `{raw_id}` — {n} version{} (active: `{active}`)\n\n",
                    if n == 1 { "" } else { "s" }
                );
                out.push_str("| id | status | file | bytes | learned_at |\n");
                out.push_str("| --- | :---: | --- | ---: | --- |\n");
                for e in &rows {
                    let status = if e.superseded_by.is_some() {
                        "superseded"
                    } else {
                        "**active**"
                    };
                    out.push_str(&format!(
                        "| `{}` | {status} | `{:?}` | {} | {} |\n",
                        e.id,
                        e.file,
                        e.body.len(),
                        e.learned_at.to_rfc3339(),
                    ));
                }
                return ("200 OK", out);
            }
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

/// `GET /memory[?file&since-minutes&lock&body-contains]` —
/// active memory entries. `file` scopes to one of four canonical
/// files; `since-minutes` filters by `learned_at`; `lock` filters
/// by lock state (`none|user-protected|content-locked`);
/// `body_contains` does a case-insensitive substring match.
/// 200 + `[]` if no workspace; 400 on unknown `file` / `lock`.
#[allow(clippy::too_many_arguments)]
pub(super) async fn memory_response(
    store: Option<&Arc<SessionStore>>,
    file: Option<String>,
    since_minutes: Option<i64>,
    lock: Option<String>,
    body_contains: Option<String>,
    limit: Option<usize>,
    format: Option<&str>,
    vertical: Option<String>,
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
                format!("{{\"error\":\"unknown file '{other}' — use memory|user|soul|agents\"}}"),
            );
        }
    };
    // #718: cwd-bound workspace (matches every writer's path).
    let workspace = match cwd_workspace(store).await {
        Ok(w) => w,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd_workspace: {e}\"}}"),
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
    let cutoff = since_minutes.map(|m| chrono::Utc::now() - chrono::Duration::minutes(m));
    let needle = body_contains.as_deref().map(str::to_lowercase);
    let mut all: Vec<serde_json::Value> = Vec::new();
    for f in files {
        // #687 — vertical-aware lookup mirrors the CLI #686
        // `argo memory list --vertical`. Default-deny: legacy
        // NULL-vertical rows excluded when ?vertical= is set.
        let entries_result = match vertical.as_deref() {
            Some(vid) => memory.list_for_vertical(workspace, f, vid).await,
            None => memory.list_active(workspace, f).await,
        };
        match entries_result {
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
                    if let Some(n) = needle.as_deref() {
                        if !e.body.to_lowercase().contains(n) {
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
                        "vertical_id": e.vertical_id,
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
    // Sort newest-first across files, then truncate to `limit`.
    // Mirrors the CLI's --sort-by=learned-at + --limit semantics.
    all.sort_by(|a, b| {
        let la = a.get("learned_at").and_then(|s| s.as_str()).unwrap_or("");
        let lb = b.get("learned_at").and_then(|s| s.as_str()).unwrap_or("");
        lb.cmp(la)
    });
    if let Some(n) = limit {
        all.truncate(n);
    }
    if matches!(format, Some("markdown")) {
        let n = all.len();
        let mut out = format!(
            "# Memory entries ({n} match{})\n\n",
            if n == 1 { "" } else { "es" }
        );
        if !all.is_empty() {
            out.push_str("| file | id | bytes | lock |\n");
            out.push_str("| --- | --- | ---: | --- |\n");
            for e in &all {
                let file = e.get("file").and_then(|s| s.as_str()).unwrap_or("?");
                let id = e.get("id").and_then(|s| s.as_str()).unwrap_or("?");
                let bytes = e.get("bytes").and_then(|n| n.as_u64()).unwrap_or(0);
                let lock = e.get("lock").and_then(|s| s.as_str()).unwrap_or("?");
                out.push_str(&format!("| {file} | `{id}` | {bytes} | {lock} |\n"));
            }
        }
        return ("200 OK", out);
    }
    (
        "200 OK",
        serde_json::to_string(&all).unwrap_or_else(|_| "[]".to_string()),
    )
}
