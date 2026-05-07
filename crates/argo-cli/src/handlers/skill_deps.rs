//! `argo skill deps <name>` — dependency-graph view.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{build_graph, build_reverse_graph, DepEdge, DepGraph, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Top-level entry. Loads the workspace store, runs forward or
/// reverse graph build, and emits markdown or JSON.
pub async fn run(name: &str, json: bool, reverse: bool, max_depth: u32) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(db_path(&home)).await?);
    store.migrate().await?;
    let skills = SkillStore::new(store);
    let graph = if reverse {
        build_reverse_graph(&skills, name, max_depth).await?
    } else {
        build_graph(&skills, name, max_depth).await?
    };
    if json {
        let body = serde_json::to_string(&graph)
            .map_err(|e| ArgoError::Provider(format!("serialize graph: {e}")))?;
        println!("{body}");
    } else {
        print_markdown(&graph, reverse);
    }
    Ok(())
}

/// Render `graph` as an indented `└─` / `├─` markdown tree
/// rooted at `graph.root`. In reverse mode the root is the
/// target — children are skills that point at it.
fn print_markdown(graph: &DepGraph, reverse: bool) {
    let title = if reverse {
        format!("# Reverse dependencies of `{}`", graph.root)
    } else {
        format!("# Dependencies of `{}`", graph.root)
    };
    println!("{title}");
    println!();
    if graph.edges.is_empty() {
        println!("(no {} dependencies)", if reverse { "reverse" } else { "forward" });
        return;
    }
    let mut children: BTreeMap<&str, Vec<&DepEdge>> = BTreeMap::new();
    for e in &graph.edges {
        let parent = if reverse { e.to.as_str() } else { e.from.as_str() };
        children.entry(parent).or_default().push(e);
    }
    let cycles: std::collections::HashSet<&str> =
        graph.cycles.iter().map(String::as_str).collect();
    let mut visited = std::collections::HashSet::<String>::new();
    println!("- `{}`", graph.root);
    walk(
        &graph.root,
        &children,
        &cycles,
        &mut visited,
        "",
        reverse,
    );
    if !graph.cycles.is_empty() {
        println!();
        println!("*depth reached: {}*", graph.depth);
        println!("*cycles detected at: {}*", graph.cycles.join(", "));
    } else {
        println!();
        println!("*depth reached: {}*", graph.depth);
    }
}

/// Recursive renderer that prints children of `parent` with
/// `prefix` as the current indent. Tracks `visited` to stop
/// printing past cycles (the cycle annotation already fired
/// in the parent's listing).
fn walk(
    parent: &str,
    children: &BTreeMap<&str, Vec<&DepEdge>>,
    cycles: &std::collections::HashSet<&str>,
    visited: &mut std::collections::HashSet<String>,
    prefix: &str,
    reverse: bool,
) {
    let Some(rows) = children.get(parent) else {
        return;
    };
    let last_idx = rows.len().saturating_sub(1);
    for (i, edge) in rows.iter().enumerate() {
        let is_last = i == last_idx;
        let target = if reverse { edge.from.as_str() } else { edge.to.as_str() };
        let branch = if is_last { "└─" } else { "├─" };
        let cycle_tag = if cycles.contains(target) {
            " (cycle)"
        } else {
            ""
        };
        println!(
            "{prefix}{branch} `{target}` [{}]{}",
            edge.kind.as_str(),
            cycle_tag
        );
        if !visited.insert(target.to_string()) {
            continue;
        }
        if cycles.contains(target) {
            continue;
        }
        let next_prefix = if is_last {
            format!("{prefix}   ")
        } else {
            format!("{prefix}│  ")
        };
        walk(target, children, cycles, visited, &next_prefix, reverse);
    }
}
