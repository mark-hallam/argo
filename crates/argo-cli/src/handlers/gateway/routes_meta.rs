//! Read-side meta endpoints: `GET /providers` (workspace
//! provider manifests), `GET /tools` (registered in-core tools),
//! `GET /search?q=…` (FTS5 search across the message corpus),
//! `GET /workspace/artifacts` (recorded marketplace pushes).
//! Lives in its own file to keep `routes.rs` under the 500-LOC
//! budget and to scope the I/O (filesystem walk for providers,
//! registry build for tools) away from the dispatch shape.
use argo_store::SessionStore;
use argo_tool::{builtin_tools, ToolRegistry};
use argo_types::SessionId;
use std::sync::Arc;

/// `GET /tools/<name>` — single tool by name. Returns 200 +
/// `{name, description, parameters}` or 404 when no tool by
/// that name is registered.
pub(super) async fn tool_show_response(raw_name: &str) -> (&'static str, String) {
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
    match reg.schemas().into_iter().find(|s| s.name == raw_name) {
        Some(s) => {
            let payload = serde_json::json!({
                "name": s.name,
                "description": s.description,
                "parameters": s.parameters,
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        None => (
            "404 Not Found",
            format!("{{\"error\":\"no tool named '{raw_name}'\"}}"),
        ),
    }
}

/// `GET /tools[?name-contains=<sub>]` — list every registered
/// in-core tool with its JSON-schema parameter shape. Optional
/// `name-contains` filter (case-insensitive) lets clients find
/// tools by pattern.
pub(super) async fn tools_response(name_contains: Option<&str>) -> (&'static str, String) {
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
    let needle = name_contains.map(str::to_lowercase);
    let rows: Vec<serde_json::Value> = reg
        .schemas()
        .into_iter()
        .filter(|s| match needle.as_deref() {
            Some(n) => s.name.to_lowercase().contains(n),
            None => true,
        })
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

/// `GET /forge/runs[?limit&since-minutes&status&score-min&skill&has-candidate]`
/// — list recorded forge optimization runs. All flags AND'd
/// together. `has-candidate=true` keeps only rows whose
/// `candidate_version` is non-NULL (i.e. produced at least
/// one iteration's worth of mutation). `score-min` and
/// `score-max` define an inclusive band on `final_score`;
/// rows with `NULL` final_score never satisfy either.
/// `iterations-min` and `iterations-max` define an inclusive
/// band on the iteration count (a fresh run has 0).
#[allow(clippy::too_many_arguments)]
pub(super) async fn forge_runs_response(
    limit: u32,
    since_minutes: Option<i64>,
    status: Option<&str>,
    score_min: Option<f64>,
    score_max: Option<f64>,
    skill: Option<&str>,
    has_candidate: Option<bool>,
    iterations_min: Option<i64>,
    iterations_max: Option<i64>,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match store.list_forge_runs(limit).await {
        Ok(rows) => {
            let cutoff_micros = since_minutes.map(|m| {
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros()
            });
            let arr: Vec<serde_json::Value> = rows
                .into_iter()
                .filter(|r| match cutoff_micros {
                    Some(c) => r.started_at >= c,
                    None => true,
                })
                .filter(|r| match status {
                    Some(want) => r.status == want,
                    None => true,
                })
                .filter(|r| match score_min {
                    Some(threshold) => r.final_score.is_some_and(|s| s >= threshold),
                    None => true,
                })
                .filter(|r| match score_max {
                    Some(threshold) => r.final_score.is_some_and(|s| s <= threshold),
                    None => true,
                })
                .filter(|r| match skill {
                    Some(want) => r.skill_name == want,
                    None => true,
                })
                .filter(|r| match has_candidate {
                    Some(true) => r.candidate_version.is_some(),
                    Some(false) => r.candidate_version.is_none(),
                    None => true,
                })
                .filter(|r| match iterations_min {
                    Some(min) => r.iterations >= min,
                    None => true,
                })
                .filter(|r| match iterations_max {
                    Some(max) => r.iterations <= max,
                    None => true,
                })
                .map(|r| {
                    // Mirror forge_run_show_response so list +
                    // single-row views agree on the elapsed math:
                    // running rows count from now, ended rows
                    // count from ended_at - started_at.
                    let now_micros = chrono::Utc::now().timestamp_micros();
                    let end_micros = r.ended_at.unwrap_or(now_micros);
                    let elapsed_seconds = ((end_micros - r.started_at) / 1_000_000).max(0);
                    serde_json::json!({
                        "id": hex::encode(&r.id),
                        "skill_name": r.skill_name,
                        "started_at": r.started_at,
                        "ended_at": r.ended_at,
                        "elapsed_seconds": elapsed_seconds,
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
            // For running rows: time-since-start; for ended
            // rows: ended_at - started_at. Both in whole seconds.
            let now_micros = chrono::Utc::now().timestamp_micros();
            let end_micros = r.ended_at.unwrap_or(now_micros);
            let elapsed_seconds = ((end_micros - r.started_at) / 1_000_000).max(0);
            let body = serde_json::json!({
                "id": hex::encode(&r.id),
                "skill_name": r.skill_name,
                "started_at": r.started_at,
                "ended_at": r.ended_at,
                "elapsed_seconds": elapsed_seconds,
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

/// `GET /costs/total` — actual spend roll-up across every
/// tenant: today (UTC), month-to-date, all-time. Mirrors the
/// "spent" block of `argo costs show`. Returned as
/// `{today, this_month, all_time}` in usd_micros.
pub(super) async fn costs_total_response(
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match store.load_global_spend().await {
        Ok(snap) => {
            let payload = serde_json::json!({
                "today":       snap.today,
                "this_month":  snap.this_month,
                "all_time":    snap.all_time,
            });
            (
                "200 OK",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"load_global_spend: {e}\"}}"),
        ),
    }
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
