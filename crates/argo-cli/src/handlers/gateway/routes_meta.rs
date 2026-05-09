//! Read-side meta endpoints: `GET /providers` (workspace
//! provider manifests), `GET /tools` (registered in-core tools),
//! `GET /search?q=…` (FTS5 search across the message corpus),
//! `GET /workspace/artifacts` (recorded marketplace pushes).
//! Lives in its own file to keep `routes.rs` under the 500-LOC
//! budget and to scope the I/O (filesystem walk for providers,
//! registry build for tools) away from the dispatch shape.
use argo_store::SessionStore;
use std::sync::Arc;

// `tool_show_response`, `tools_response`, and `tools_count_response`
// live in `super::routes_meta_tools` — re-exported below so call
// sites in `routes_dispatch_get` stay unchanged. Lifted out to
// keep this file under the 500-LOC budget.
pub(super) use super::routes_meta_tools::{
    tool_show_response, tools_count_response, tools_response,
};

/// `GET /search` + `GET /search/count` live in the sibling
/// `routes_search` module; this re-export keeps the call sites
/// in `routes.rs` unchanged. Lifted out so this file stays
/// under the 500-LOC budget as the catalog grows.
pub(super) use super::routes_search::search_response;

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

/// `GET /db/stats[?format=markdown]` — per-table row counts
/// for the workspace store. Mirrors the `db status` block of
/// `argo doctor`. JSON default; `?format=markdown` returns a
/// 2-column paste-able table sorted by row count desc.
pub(super) async fn db_stats_response(
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match store.db_stats().await {
        Ok(stats) => {
            if matches!(format, Some("markdown")) {
                let mut rows: Vec<(&str, i64)> = vec![
                    ("session", stats.sessions),
                    ("message", stats.messages),
                    ("tool_call", stats.tool_calls),
                    ("audit_record", stats.audit_rows),
                    ("routing_decision", stats.routing_decisions),
                    ("judge_record", stats.judge_records),
                    ("memory_entry", stats.memory_entries),
                    ("skill", stats.skills),
                    ("forge_run", stats.forge_runs),
                    ("spend_ledger", stats.spend_rows),
                ];
                rows.sort_by_key(|r| std::cmp::Reverse(r.1));
                let total: i64 = rows.iter().map(|(_, n)| *n).sum();
                let mut out = format!("# DB stats ({total} rows across 10 tables)\n\n");
                out.push_str("| table | rows |\n");
                out.push_str("| --- | ---: |\n");
                for (t, n) in &rows {
                    out.push_str(&format!("| `{t}` | {n} |\n"));
                }
                return ("200 OK", out);
            }
            (
                "200 OK",
                serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"db_stats: {e}\"}}"),
        ),
    }
}

/// `GET /db/version[?format=markdown]` — current SQLite
/// schema version (the migration counter). Returns `{version}`
/// JSON or a single-line markdown header. Useful for ops
/// dashboards verifying that a deployment has the latest
/// schema applied.
pub(super) async fn db_version_response(
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    match store.schema_version().await {
        Ok(v) => {
            if matches!(format, Some("markdown")) {
                return (
                    "200 OK",
                    format!("# DB schema version `{v}`\n"),
                );
            }
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({ "version": v }))
                    .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"schema_version: {e}\"}}"),
        ),
    }
}

/// `GET /db/integrity[?format=markdown]` — runs
/// `PRAGMA integrity_check` against the workspace store. SQLite
/// reports a single `"ok"` row when the database is healthy
/// and one or more diagnostic rows otherwise. Returns
/// `{ok: bool, check: [<row>, ...]}` so callers can branch on
/// the boolean and surface the raw rows when something is off.
pub(super) async fn db_integrity_response(
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let pool = store.pool();
    let rows: Result<Vec<(String,)>, _> = sqlx::query_as("PRAGMA integrity_check")
        .fetch_all(pool)
        .await;
    match rows {
        Ok(rows) => {
            let lines: Vec<String> = rows.into_iter().map(|(s,)| s).collect();
            let ok = lines.len() == 1 && lines.first().is_some_and(|s| s == "ok");
            if matches!(format, Some("markdown")) {
                let mut out = format!(
                    "# DB integrity ({})\n\n",
                    if ok { "ok" } else { "FAIL" }
                );
                if !ok {
                    out.push_str("## diagnostics\n\n");
                    for l in &lines {
                        let safe = l.replace('|', "\\|");
                        out.push_str(&format!("- `{safe}`\n"));
                    }
                }
                return ("200 OK", out);
            }
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({
                    "ok": ok,
                    "check": lines,
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"integrity_check: {e}\"}}"),
        ),
    }
}
