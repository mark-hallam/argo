//! Read-side meta endpoints: `GET /providers` (workspace
//! provider manifests), `GET /tools` (registered in-core tools),
//! `GET /search?q=…` (FTS5 search across the message corpus),
//! `GET /workspace/artifacts` (recorded marketplace pushes).
//! Lives in its own file to keep `routes.rs` under the 500-LOC
//! budget and to scope the I/O (filesystem walk for providers,
//! registry build for tools) away from the dispatch shape.
use argo_provider::ProviderManifest;
use argo_store::SessionStore;
use argo_tool::{builtin_tools, ToolRegistry};
use argo_types::SessionId;
use std::path::PathBuf;
use std::sync::Arc;

/// `GET /providers` — list every parseable manifest under
/// `<cwd>/manifests/providers/*.toml` as a JSON array of
/// `{id, family, base_url, models: [{id, ctx_window?,
/// price_per_1k_in?, price_per_1k_out?}]}` objects. Invalid
/// manifests are reported with `{path, error}` rows so a typo
/// is visible without crashing the whole list.
pub(super) async fn providers_response() -> (&'static str, String) {
    let Ok(cwd) = std::env::current_dir() else {
        return (
            "500 Internal Server Error",
            "{\"error\":\"cwd unavailable\"}".to_string(),
        );
    };
    let dir = cwd.join("manifests").join("providers");
    let mut rows: Vec<serde_json::Value> = Vec::new();
    if !dir.exists() {
        return (
            "200 OK",
            serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
        );
    }
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect(),
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"read_dir: {e}\"}}"),
            );
        }
    };
    paths.sort();
    for p in paths {
        match std::fs::read_to_string(&p).and_then(|body| {
            ProviderManifest::from_toml_str(&body).map_err(std::io::Error::other)
        }) {
            Ok(m) => {
                let models: Vec<serde_json::Value> = m
                    .models
                    .iter()
                    .map(|spec| {
                        serde_json::json!({
                            "id": spec.id,
                            "context_window": spec.context_window,
                            "price_per_1k_in": spec.price_per_1k_in,
                            "price_per_1k_out": spec.price_per_1k_out,
                            "tier": spec.tier,
                        })
                    })
                    .collect();
                rows.push(serde_json::json!({
                    "id": m.provider.id,
                    "family": format!("{:?}", m.provider.family),
                    "base_url": m.provider.base_url,
                    "models": models,
                }));
            }
            Err(e) => rows.push(serde_json::json!({
                "path": p.display().to_string(),
                "error": e.to_string(),
            })),
        }
    }
    (
        "200 OK",
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
    )
}

/// `GET /tools` — list every registered in-core tool with its
/// JSON-schema parameter shape, exactly as the agent loop will
/// hand it to a provider. Useful for a UI / docs surface that
/// wants to introspect what an Argo agent can do without scraping
/// `argo provider show`.
pub(super) async fn tools_response() -> (&'static str, String) {
    let Ok(cwd) = std::env::current_dir() else {
        return (
            "500 Internal Server Error",
            "{\"error\":\"cwd unavailable\"}".to_string(),
        );
    };
    let mut reg = ToolRegistry::new();
    for t in builtin_tools(cwd) {
        reg.register(t);
    }
    let rows: Vec<serde_json::Value> = reg
        .schemas()
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "description": s.description,
                "parameters": s.parameters,
            })
        })
        .collect();
    (
        "200 OK",
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
    )
}

/// `GET /search?q=<query>[&limit=N][&session=<uuid>]` — FTS5
/// search across the message corpus. Mirrors `argo session search
/// <query>` so external clients (web UI, scripts) can reuse the
/// same query syntax.
///
/// Each row is `{session_id, seq, role, snippet}`. Returns 503
/// when the store is unavailable, 400 when `q` is missing or
/// `session` is not a valid UUID.
pub(super) async fn search_response(
    query: Option<&str>,
    limit: u32,
    session: Option<&str>,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let Some(q) = query else {
        return (
            "400 Bad Request",
            "{\"error\":\"missing q parameter\"}".to_string(),
        );
    };
    if q.is_empty() {
        return (
            "400 Bad Request",
            "{\"error\":\"empty q parameter\"}".to_string(),
        );
    }
    let result = match session {
        None => store.search_messages(q, limit).await,
        Some(s) => match uuid::Uuid::parse_str(s) {
            Ok(u) => store.search_messages_in_session(SessionId(u), q, limit).await,
            Err(e) => {
                return (
                    "400 Bad Request",
                    format!("{{\"error\":\"invalid session id: {e}\"}}"),
                );
            }
        },
    };
    match result {
        Ok(hits) => {
            let rows: Vec<serde_json::Value> = hits
                .into_iter()
                .map(|h| {
                    serde_json::json!({
                        "session_id": h.session_id.0.to_string(),
                        "seq": h.seq,
                        "role": h.role,
                        "snippet": h.snippet,
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"search: {e}\"}}"),
        ),
    }
}

/// `GET /workspace/artifacts[?limit=N]` — list recorded
/// marketplace pushes from the workspace's `marketplace_artifact`
/// table, newest first. Defaults to 25 rows. Returns 503 if the
/// store is unavailable or 200 + an empty array if no workspace
/// is registered yet.
pub(super) async fn artifacts_response(
    limit: u32,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
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
    match store.list_marketplace_artifacts(workspace, limit).await {
        Ok(rows) => (
            "200 OK",
            serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list_marketplace_artifacts: {e}\"}}"),
        ),
    }
}

/// `GET /forge/runs[?limit=N]` — list recorded forge optimization
/// runs from the local DB (the same rows `argo forge runs list`
/// surfaces). Defaults to 50 rows. Returns 503 when the store is
/// unavailable; 200 + an empty array on a clean DB.
pub(super) async fn forge_runs_response(
    limit: u32,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match store.list_forge_runs(limit).await {
        Ok(rows) => {
            let arr: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "id": hex::encode(&r.id),
                        "skill_name": r.skill_name,
                        "started_at": r.started_at,
                        "ended_at": r.ended_at,
                        "base_version": r.base_version,
                        "candidate_version": r.candidate_version,
                        "iterations": r.iterations,
                        "spend_usd_micros": r.spend_usd_micros,
                        "final_score": r.final_score,
                        "status": r.status,
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
            format!("{{\"error\":\"list_forge_runs: {e}\"}}"),
        ),
    }
}

/// `GET /memory/<id>` — fetch a memory entry by id (mirrors
/// `argo memory view <id>`). Returns 404 when no entry matches,
/// 400 when the id isn't a valid UUID.
pub(super) async fn memory_show_response(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(raw_id) {
        Ok(u) => u,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid memory entry id: {e}\"}}"),
            );
        }
    };
    let memory = argo_memory::MemoryStore::new(Arc::clone(store));
    match memory.get(id).await {
        Ok(e) => {
            let body = serde_json::json!({
                "id": e.id.to_string(),
                "file": format!("{:?}", e.file),
                "body": e.body,
                "lock": format!("{:?}", e.lock),
                "hash": hex::encode(e.hash),
                "learned_at": e.learned_at.to_rfc3339(),
            });
            (
                "200 OK",
                serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(argo_types::ArgoError::Schema(_)) => (
            "404 Not Found",
            format!("{{\"error\":\"no memory entry with id {raw_id}\"}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"memory_show: {e}\"}}"),
        ),
    }
}

/// `GET /skills/<name>` — fetch the active skill for a name
/// (mirrors `argo skill view <name>`). Returns 404 if no active
/// row matches; the body includes the skill body text + lock +
/// frontmatter for clients that want the full document.
/// `GET /skills/<name>/versions` — list every row matching
/// `<name>` (active + superseded) as a JSON array sorted by
/// `created_at` ASC. Useful for visualising the supersede
/// chain after a `forge optimize` or `skill refine` cycle.
pub(super) async fn skill_versions_response(
    name: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    match skills.versions_for_name(name).await {
        Ok(rows) if rows.is_empty() => (
            "404 Not Found",
            format!("{{\"error\":\"no skill named '{name}'\"}}"),
        ),
        Ok(rows) => {
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "id": s.id.0.to_string(),
                        "version": s.frontmatter.version,
                        "hash": hex::encode(s.hash),
                        "lock": format!("{:?}", s.frontmatter.lock),
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
            format!("{{\"error\":\"versions_for_name: {e}\"}}"),
        ),
    }
}

pub(super) async fn skill_show_response(
    name: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    match skills.find_active_by_name(name).await {
        Ok(Some(s)) => {
            let body = serde_json::json!({
                "id": s.id.0.to_string(),
                "name": s.frontmatter.name,
                "version": s.frontmatter.version,
                "description": s.frontmatter.description,
                "lock": format!("{:?}", s.frontmatter.lock),
                "hash": hex::encode(s.hash),
                "body": s.body,
                "created_at": s.created_at.to_rfc3339(),
            });
            (
                "200 OK",
                serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Ok(None) => (
            "404 Not Found",
            format!("{{\"error\":\"no active skill named '{name}'\"}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"skill_show: {e}\"}}"),
        ),
    }
}

/// `GET /forge/runs/<id>` — fetch one forge_run row by id (mirrors
/// `argo forge runs show <id>`). Returns 404 when no row matches,
/// 503 when the store is unavailable, 400 when the id isn't a
/// valid UUID.
pub(super) async fn forge_run_show_response(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(raw_id) {
        Ok(u) => u,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid forge run id: {e}\"}}"),
            );
        }
    };
    match store.get_forge_run(id).await {
        Ok(Some(r)) => {
            let body = serde_json::json!({
                "id": hex::encode(&r.id),
                "skill_name": r.skill_name,
                "started_at": r.started_at,
                "ended_at": r.ended_at,
                "base_version": r.base_version,
                "candidate_version": r.candidate_version,
                "iterations": r.iterations,
                "spend_usd_micros": r.spend_usd_micros,
                "final_score": r.final_score,
                "status": r.status,
            });
            (
                "200 OK",
                serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Ok(None) => (
            "404 Not Found",
            format!("{{\"error\":\"no forge run with id {raw_id}\"}}"),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"get_forge_run: {e}\"}}"),
        ),
    }
}

/// `GET /routes` — discovery endpoint listing every route the
/// gateway dispatches, with a one-line description per row.
/// Lets a UI / docs surface enumerate the API without scraping
/// the help banner. Static — no DB or filesystem I/O — so it's
/// always fast and never 503.
pub(super) use super::routes_catalog::routes_response;

/// `GET /costs` — cost ceilings from `argo.toml`. No DB hit.
pub(super) fn costs_response() -> (&'static str, String) {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"cwd: {e}\"}}"),
            );
        }
    };
    let cfg = argo_config::ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    let ceilings = cfg.budget.to_cost_ceilings();
    let payload = serde_json::json!({
        "per_session_usd_micros": ceilings.per_session_usd_micros,
        "per_day_usd_micros":     ceilings.per_day_usd_micros,
        "per_month_usd_micros":   ceilings.per_month_usd_micros,
        "notify_at_pct":          ceilings.notify_at_pct,
    });
    (
        "200 OK",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// `GET /db/stats` — per-table row counts for the workspace
/// store. Mirrors the `db status` block of `argo doctor`.
pub(super) async fn db_stats_response(
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match store.db_stats().await {
        Ok(stats) => (
            "200 OK",
            serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string()),
        ),
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"db_stats: {e}\"}}"),
        ),
    }
}
