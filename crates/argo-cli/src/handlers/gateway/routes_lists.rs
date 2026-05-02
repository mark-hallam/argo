//! Bulk-list GET handlers — `/skills` and `/memory`. Lifted
//! out of `routes.rs` so the dispatcher file stays under the
//! 500-LOC ceiling.
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /skills[?include-superseded&name-contains&lock&body-contains]` —
/// active skills by default, all versions when
/// `include-superseded`. Optional substring filters
/// (case-insensitive) on `name-contains` (frontmatter.name) and
/// `body-contains` (markdown body), plus a `lock`
/// (`none|user-protected|content-locked`) filter. Each row:
/// `{id, name, version, lock, hash, created_at, superseded_by?}`.
#[allow(clippy::too_many_arguments)]
pub(super) async fn skills_response(
    store: Option<&Arc<SessionStore>>,
    include_superseded: bool,
    name_contains: Option<&str>,
    lock: Option<&str>,
    body_contains: Option<&str>,
    limit: Option<usize>,
    format: Option<&str>,
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
    let body_needle = body_contains.map(str::to_lowercase);
    match result {
        Ok(rows) => {
            let filtered: Vec<&argo_skill::Skill> = rows
                .iter()
                .filter(|s| match needle.as_deref() {
                    Some(n) => s.frontmatter.name.to_lowercase().contains(n),
                    None => true,
                })
                .filter(|s| match body_needle.as_deref() {
                    Some(n) => s.body.to_lowercase().contains(n),
                    None => true,
                })
                .filter(|s| match lock_filter {
                    Some(want) => s.frontmatter.lock == want,
                    None => true,
                })
                .collect();
            let truncated: Vec<&argo_skill::Skill> = match limit {
                Some(n) => filtered.iter().take(n).copied().collect(),
                None => filtered,
            };
            if matches!(format, Some("markdown")) {
                let n = truncated.len();
                let mut out = format!(
                    "# Skills ({n} match{})\n\n",
                    if n == 1 { "" } else { "es" }
                );
                if !truncated.is_empty() {
                    out.push_str("| name | version | lock | hash |\n");
                    out.push_str("| --- | --- | --- | --- |\n");
                    for s in &truncated {
                        let hash_short: String = hex::encode(s.hash).chars().take(12).collect();
                        out.push_str(&format!(
                            "| `{}` | `{}` | {:?} | `{hash_short}…` |\n",
                            s.frontmatter.name, s.frontmatter.version, s.frontmatter.lock,
                        ));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = truncated
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

/// `GET /skills/count[?include-superseded=true]` — count active
/// skills (or all versions). Mirrors the CLI's `argo skill
/// count` (#430).
pub(super) async fn skills_count_response(
    include_superseded: bool,
    store: Option<&Arc<SessionStore>>,
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
            let payload = serde_json::json!({
                "count": rows.len(),
                "include_superseded": include_superseded,
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list skills: {e}\"}}"),
        ),
    }
}

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
                format!(
                    "{{\"error\":\"unknown file '{other}' — use memory|user|soul|agents\"}}"
                ),
            );
        }
    };
    let workspace = match store.first_workspace().await {
        Ok(Some(w)) => w,
        Ok(None) => {
            return (
                "200 OK",
                format!("{{\"count\":0,\"file\":\"{}\"}}", file.as_deref().unwrap_or("all")),
            );
        }
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"first_workspace: {e}\"}}"),
            );
        }
    };
    let memory = MemoryStore::new(Arc::clone(store));
    let cutoff = since_minutes
        .map(|m| chrono::Utc::now() - chrono::Duration::minutes(m));
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

/// `GET /memory[?file&since-minutes&lock]` — active memory
/// entries. `file` scopes to one of the four canonical files.
/// `since-minutes` filters by `learned_at`. `lock` filters by
/// the lock state (`none|user-protected|content-locked`).
/// `body_contains` keeps only entries whose body contains the
/// given substring (case-insensitive). 200 + `[]` when no
/// workspace is registered; 400 on unknown `file` or `lock`.
#[allow(clippy::too_many_arguments)]
pub(super) async fn memory_response(
    store: Option<&Arc<SessionStore>>,
    file: Option<String>,
    since_minutes: Option<i64>,
    lock: Option<String>,
    body_contains: Option<String>,
    limit: Option<usize>,
    format: Option<&str>,
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
    let needle = body_contains.as_deref().map(str::to_lowercase);
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
