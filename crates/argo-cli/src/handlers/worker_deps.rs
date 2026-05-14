//! `argo worker deps add/remove/list/graph/impact` (Stream
//! `claude-worker-dependency-tracking`, V1.1).
//!
//! Wires `argo_store::worker_deps` to the CLI surface declared in
//! `cli_worker_deps_cmds.rs`. Three renderers live here — markdown
//! (terminal-friendly), DOT (graphviz), JSON (machine-readable).

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::worker_deps::{
    self, DepKind, DepRow, Direction, ImpactedWorker,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;

/// `argo worker deps add <a> <b> --kind <kind> [--note ...] [--json]`.
pub async fn add(
    a: &str,
    b: &str,
    kind_token: &str,
    note: Option<&str>,
    json_out: bool,
) -> ArgoResult<()> {
    let kind = DepKind::parse(kind_token)?;
    let pool = open_pool().await?;
    let inserted = worker_deps::add_dependency(&pool, a, b, kind, note).await?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "inserted": inserted,
                "a": a,
                "b": b,
                "kind": kind.as_str(),
                "note": note,
            }))
            .unwrap_or_else(|_| "{}".into())
        );
    } else if inserted {
        println!("ok: {a} → {b} ({})", kind.as_str());
    } else {
        println!(
            "ok: {a} → {b} ({}) — already declared, refreshed timestamp/note",
            kind.as_str()
        );
    }
    Ok(())
}

/// `argo worker deps remove <a> <b> --kind <kind> [--json]`.
pub async fn remove(a: &str, b: &str, kind_token: &str, json_out: bool) -> ArgoResult<()> {
    let kind = DepKind::parse(kind_token)?;
    let pool = open_pool().await?;
    let removed = worker_deps::remove_dependency(&pool, a, b, kind).await?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "removed": removed,
                "a": a,
                "b": b,
                "kind": kind.as_str(),
            }))
            .unwrap_or_else(|_| "{}".into())
        );
    } else if removed > 0 {
        println!("removed: {a} → {b} ({})", kind.as_str());
    } else {
        println!("(no row matched {a} → {b} {})", kind.as_str());
    }
    Ok(())
}

/// `argo worker deps list <id> [--direction in|out|both] [--format ...]`.
pub async fn list(worker_id: &str, direction_token: &str, format: &str) -> ArgoResult<()> {
    let direction = Direction::parse(direction_token)?;
    let pool = open_pool().await?;
    let rows = worker_deps::list_for_worker(&pool, worker_id, direction).await?;
    match format {
        "json" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "worker_id": worker_id,
                    "direction": direction_token,
                    "count": rows.len(),
                    "deps": rows.iter().map(dep_row_json).collect::<Vec<_>>(),
                }))
                .unwrap_or_else(|_| "{}".into())
            );
        }
        "markdown" => {
            print!("{}", render_list_markdown(worker_id, direction, &rows));
        }
        other => {
            return Err(ArgoError::Provider(format!(
                "invalid --format '{other}': expected markdown|json"
            )));
        }
    }
    Ok(())
}

/// `argo worker deps graph [--format markdown|dot|json]`.
pub async fn graph(format: &str) -> ArgoResult<()> {
    let pool = open_pool().await?;
    let rows = worker_deps::list_all(&pool).await?;
    match format {
        "json" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "count": rows.len(),
                    "edges": rows.iter().map(dep_row_json).collect::<Vec<_>>(),
                }))
                .unwrap_or_else(|_| "{}".into())
            );
        }
        "markdown" => {
            print!("{}", render_graph_markdown(&rows));
        }
        "dot" => {
            print!("{}", render_dot(&rows));
        }
        other => {
            return Err(ArgoError::Provider(format!(
                "invalid --format '{other}': expected markdown|dot|json"
            )));
        }
    }
    Ok(())
}

/// `argo worker deps impact <id> [--json]`.
pub async fn impact(worker_id: &str, json_out: bool) -> ArgoResult<()> {
    let pool = open_pool().await?;
    let entries = worker_deps::impact(&pool, worker_id).await?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "seed": worker_id,
                "count": entries.len(),
                "impacted": entries.iter().map(impacted_json).collect::<Vec<_>>(),
            }))
            .unwrap_or_else(|_| "{}".into())
        );
    } else {
        print!("{}", render_impact_markdown(worker_id, &entries));
    }
    Ok(())
}

// ───────────────────────── renderers ─────────────────────────

fn render_list_markdown(worker_id: &str, direction: Direction, rows: &[DepRow]) -> String {
    let header = match direction {
        Direction::Out => format!("# Worker {worker_id} dependencies (out)\n\n"),
        Direction::In => format!("# Workers depending on {worker_id} (in)\n\n"),
        Direction::Both => format!("# Worker {worker_id} dependency edges (both)\n\n"),
    };
    let mut out = header;
    if rows.is_empty() {
        out.push_str("_No dependencies declared._\n");
        return out;
    }
    out.push_str("| kind | a | b | note |\n");
    out.push_str("| ---- | - | - | ---- |\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            row.kind.as_str(),
            row.a_worker_id,
            row.b_worker_id,
            row.note.as_deref().unwrap_or(""),
        ));
    }
    out
}

fn render_graph_markdown(rows: &[DepRow]) -> String {
    let mut out = String::from("# Workspace dependency graph\n\n");
    if rows.is_empty() {
        out.push_str("_No dependencies declared._\n");
        return out;
    }
    out.push_str(&format!("Total edges: **{}**\n\n", rows.len()));
    out.push_str("| kind | a | b | note |\n");
    out.push_str("| ---- | - | - | ---- |\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            row.kind.as_str(),
            row.a_worker_id,
            row.b_worker_id,
            row.note.as_deref().unwrap_or(""),
        ));
    }
    out
}

fn render_dot(rows: &[DepRow]) -> String {
    let mut out = String::from("digraph worker_deps {\n");
    out.push_str("  rankdir=LR;\n");
    out.push_str("  node [shape=box, style=rounded];\n");
    let mut nodes: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for row in rows {
        nodes.insert(row.a_worker_id.as_str());
        nodes.insert(row.b_worker_id.as_str());
    }
    for n in &nodes {
        out.push_str(&format!("  \"{}\";\n", escape_dot(n)));
    }
    for row in rows {
        out.push_str(&format!(
            "  \"{}\" -> \"{}\" [color=\"{}\", label=\"{}\"];\n",
            escape_dot(&row.a_worker_id),
            escape_dot(&row.b_worker_id),
            kind_color(row.kind),
            row.kind.as_str(),
        ));
    }
    out.push_str("}\n");
    out
}

fn render_impact_markdown(seed: &str, entries: &[ImpactedWorker]) -> String {
    let mut out = format!("# Workers impacted by {seed} outage\n\n");
    if entries.is_empty() {
        out.push_str("_No declared dependents._\n");
        return out;
    }
    out.push_str(&format!("Total impacted: **{}**\n\n", entries.len()));
    out.push_str("| depth | worker_id |\n");
    out.push_str("| ----: | --------- |\n");
    for entry in entries {
        out.push_str(&format!("| {} | {} |\n", entry.depth, entry.worker_id));
    }
    out
}

fn kind_color(kind: DepKind) -> &'static str {
    match kind {
        DepKind::Data => "blue",
        DepKind::Approval => "red",
        DepKind::Handoff => "green",
        DepKind::Skill => "purple",
    }
}

fn escape_dot(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

fn dep_row_json(row: &DepRow) -> serde_json::Value {
    json!({
        "a": row.a_worker_id,
        "b": row.b_worker_id,
        "kind": row.kind.as_str(),
        "declared_at": row.declared_at,
        "note": row.note,
    })
}

fn impacted_json(entry: &ImpactedWorker) -> serde_json::Value {
    json!({
        "depth": entry.depth,
        "worker_id": entry.worker_id,
    })
}

async fn open_pool() -> ArgoResult<sqlx::SqlitePool> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    worker_deps::ensure_schema(store.pool()).await?;
    Ok(store.pool().clone())
}

#[cfg(test)]
#[path = "worker_deps_tests.rs"]
mod tests;
