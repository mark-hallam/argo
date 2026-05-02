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

/// `GET /tools/<name>[?format=markdown]` — single tool by name.
/// Returns 200 + `{name, description, parameters}` or 404 when
/// no tool by that name is registered. With
/// `?format=markdown`, returns a `# <name>` doc with the
/// description + pretty-printed JSON-schema parameters block.
pub(super) async fn tool_show_response(
    raw_name: &str,
    format: Option<&str>,
) -> (&'static str, String) {
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
            if matches!(format, Some("markdown")) {
                let mut out = format!("# {}\n\n{}\n\n## Parameters\n\n", s.name, s.description);
                let pretty = serde_json::to_string_pretty(&s.parameters)
                    .unwrap_or_else(|_| "{}".to_string());
                out.push_str("```json\n");
                out.push_str(&pretty);
                out.push_str("\n```\n");
                return ("200 OK", out);
            }
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

/// `GET /tools[?name-contains=<sub>&format=markdown]` — list
/// every registered in-core tool with its JSON-schema parameter
/// shape. Optional `name-contains` filter (case-insensitive)
/// lets clients find tools by pattern. With `format=markdown`,
/// the response is a markdown table — paste-able into
/// READMEs / runbooks.
pub(super) async fn tools_response(
    name_contains: Option<&str>,
    format: Option<&str>,
) -> (&'static str, String) {
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
    let mut schemas: Vec<argo_tool::ToolSchema> = reg
        .schemas()
        .into_iter()
        .filter(|s| match needle.as_deref() {
            Some(n) => s.name.to_lowercase().contains(n),
            None => true,
        })
        .collect();
    schemas.sort_by(|a, b| a.name.cmp(&b.name));
    if matches!(format, Some("markdown")) {
        let mut out = format!("# Tools ({})\n\n", schemas.len());
        if !schemas.is_empty() {
            out.push_str("| name | description |\n");
            out.push_str("| --- | --- |\n");
            for s in &schemas {
                let desc = s
                    .description
                    .lines()
                    .next()
                    .unwrap_or("")
                    .replace('|', "\\|");
                out.push_str(&format!("| `{}` | {desc} |\n", s.name));
            }
        }
        return ("200 OK", out);
    }
    let rows: Vec<serde_json::Value> = schemas
        .iter()
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

/// `GET /search?q=<query>[&limit=N][&session=<uuid>][&role=<r>]` —
/// FTS5 search across the message corpus. Mirrors
/// `argo session search <query>` so external clients (web UI,
/// scripts) can reuse the same query syntax.
///
/// Each row is `{session_id, seq, role, snippet}`. With
/// `?role=user|assistant|tool|system` the result is filtered
/// to that role. Returns 503 when the store is unavailable,
/// 400 when `q` is missing, `session` is not a valid UUID, or
/// `role` is not one of the four canonical labels.
#[allow(clippy::too_many_arguments)]
pub(super) async fn search_response(
    query: Option<&str>,
    limit: u32,
    session: Option<&str>,
    role: Option<&str>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
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
    if let Some(want) = role {
        if !matches!(want, "user" | "assistant" | "tool" | "system") {
            return (
                "400 Bad Request",
                format!(
                    "{{\"error\":\"unknown role '{want}' — use user|assistant|tool|system\"}}"
                ),
            );
        }
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
            let filtered: Vec<argo_store::MessageSearchHit> = hits
                .into_iter()
                .filter(|h| match role {
                    Some(want) => h.role == want,
                    None => true,
                })
                .collect();
            if matches!(format, Some("markdown")) {
                let n = filtered.len();
                let mut out = format!(
                    "# Search results for `{q}` ({n} match{})\n\n",
                    if n == 1 { "" } else { "es" }
                );
                if !filtered.is_empty() {
                    out.push_str("| session_id | seq | role | snippet |\n");
                    out.push_str("| --- | ---: | --- | --- |\n");
                    for h in &filtered {
                        let snippet = h.snippet.replace('|', "\\|");
                        out.push_str(&format!(
                            "| `{}` | {} | `{}` | {snippet} |\n",
                            h.session_id.0, h.seq, h.role
                        ));
                    }
                }
                return ("200 OK", out);
            }
            let rows: Vec<serde_json::Value> = filtered
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

// `forge_runs_response` lives in `super::routes_forge` to keep
// this file under the 500-LOC budget.
pub(super) use super::routes_forge::forge_runs_response;

/// `GET /memory/<id>` — fetch a memory entry by id (mirrors
/// `argo memory view <id>`). Returns 404 when no entry matches,
/// 400 when the id isn't a valid UUID.
/// `GET /memory/<id>[?format=markdown]` — single memory entry
/// with body. Default JSON shape; with `?format=markdown`,
/// returns a header (`# Memory entry <id>`) + metadata bullets
/// + the raw body — paste-able as standalone documentation.
pub(super) async fn memory_show_response_with_format(
    raw_id: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
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
            if matches!(format, Some("markdown")) {
                let mut out = format!("# Memory entry `{}`\n\n", e.id);
                out.push_str(&format!("- **File**: `{:?}`\n", e.file));
                out.push_str(&format!("- **Lock**: `{:?}`\n", e.lock));
                out.push_str(&format!(
                    "- **Hash**: `sha256-{}`\n",
                    hex::encode(e.hash)
                ));
                out.push_str(&format!(
                    "- **Learned at**: {}\n",
                    e.learned_at.to_rfc3339()
                ));
                out.push_str("\n---\n\n");
                out.push_str(&e.body);
                if !e.body.ends_with('\n') {
                    out.push('\n');
                }
                return ("200 OK", out);
            }
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

/// `GET /skills/<name>` + `GET /skills/<name>/versions` —
/// re-exported from the sibling `routes_skills_show` module
/// (lifted out of this file to keep it under the 500-LOC
/// budget). The previous `pub(super) async fn ...` definitions
/// lived here directly until the catalog grew large enough to
/// push the file over.
pub(super) use super::routes_skills_show::{
    skill_show_response_with_format, skill_versions_response,
};

// `forge_run_show_response` lives in `super::routes_forge` to
// keep this file under the 500-LOC budget.

/// `GET /routes` — discovery endpoint listing every route the
/// gateway dispatches, with a one-line description per row.
/// Lets a UI / docs surface enumerate the API without scraping
/// the help banner. Static — no DB or filesystem I/O — so it's
/// always fast and never 503.
pub(super) use super::routes_catalog::routes_response;

/// `GET /costs[?format=markdown]` — cost ceilings from
/// `argo.toml`. No DB hit. Markdown returns a 4-bullet doc.
pub(super) fn costs_response(format: Option<&str>) -> (&'static str, String) {
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
    if matches!(format, Some("markdown")) {
        let mut out = String::from("# Cost ceilings\n\n");
        for (label, val) in [
            ("per-session", ceilings.per_session_usd_micros),
            ("per-day", ceilings.per_day_usd_micros),
            ("per-month", ceilings.per_month_usd_micros),
        ] {
            match val {
                Some(v) => out.push_str(&format!("- **{label}**: ${:.2}\n", v as f64 / 1_000_000.0)),
                None => out.push_str(&format!("- **{label}**: _(unlimited)_\n")),
            }
        }
        out.push_str(&format!(
            "- **notify_at_pct**: {}\n",
            ceilings
                .notify_at_pct
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
        return ("200 OK", out);
    }
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
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match store.load_global_spend().await {
        Ok(snap) => {
            if matches!(format, Some("markdown")) {
                let mut out = String::from("# Spend roll-up\n\n");
                out.push_str(&format!(
                    "- **Today**: ${:.4}\n",
                    snap.today as f64 / 1_000_000.0
                ));
                out.push_str(&format!(
                    "- **This month**: ${:.4}\n",
                    snap.this_month as f64 / 1_000_000.0
                ));
                out.push_str(&format!(
                    "- **All time**: ${:.4}\n",
                    snap.all_time as f64 / 1_000_000.0
                ));
                return ("200 OK", out);
            }
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
