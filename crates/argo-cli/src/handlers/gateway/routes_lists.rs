//! Bulk-list GET handlers — `/skills` and `/memory`. Lifted
//! out of `routes.rs` so the dispatcher file stays under the
//! 500-LOC ceiling.
//!
//! The `/memory` family handlers live in the
//! `routes_lists_memory` sibling, re-exported below so existing
//! call sites (`super::routes_lists::{…}`) keep working.
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) use super::routes_lists_memory::{
    memory_count_response, memory_response, memory_versions_response,
};

/// `GET /skills[?include-superseded&name-contains&lock&body-contains]` —
/// active skills by default, all versions when
/// `include-superseded`. Optional substring filters
/// (case-insensitive) on `name-contains` (frontmatter.name) and
/// `body-contains` (markdown body), plus a `lock`
/// (`none|user-protected|content-locked`) filter. Each row:
/// `{id, name, version, lock, hash, created_at, superseded_by?}`.
#[allow(clippy::too_many_arguments)]
pub(super) async fn skills_response(
    req: &str,
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
    // WS1 tenant-isolation: the active-skill list is scoped to the
    // caller's tenant (JWT for a signed-in user, cwd/demo tenant for
    // anonymous — which is exactly the tenant the demo seed installs
    // under, so the demo `/skills` page is unaffected). The
    // `?include-superseded=true` version-history path (`list_all`) is
    // out of this brief's scope and stays on the permissive seam.
    let result = if include_superseded {
        skills.list_all().await
    } else {
        let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
            Ok(t) => t,
            Err(e) => {
                return (
                    "500 Internal Server Error",
                    format!("{{\"error\":\"resolve tenant: {e}\"}}"),
                );
            }
        };
        skills.list_active_for_tenant(&tenant.to_string()).await
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
                let mut out = format!("# Skills ({n} match{})\n\n", if n == 1 { "" } else { "es" });
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
    req: &str,
    include_superseded: bool,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    // WS1 tenant-isolation: count only the caller's tenant's active
    // skills (mirrors `skills_response`). `include-superseded` stays
    // on the permissive `list_all` seam (out of brief scope).
    let result = if include_superseded {
        skills.list_all().await
    } else {
        let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
            Ok(t) => t,
            Err(e) => {
                return (
                    "500 Internal Server Error",
                    format!("{{\"error\":\"resolve tenant: {e}\"}}"),
                );
            }
        };
        skills.list_active_for_tenant(&tenant.to_string()).await
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
