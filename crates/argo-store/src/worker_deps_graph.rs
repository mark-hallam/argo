//! Graph internals for [`super`] (`worker_deps`): cycle detection,
//! reachability, path recovery, and the sqlx-backed adjacency loaders.
//!
//! Split out of `worker_deps.rs` (the 500-LOC ceiling); the parent
//! module owns the public surface and keeps these helpers private via
//! `pub(super)`. Pure graph algorithms (`reachable`, `path_between`)
//! sit alongside the DB-bound loaders so the cycle-detection cluster
//! stays cohesive.

use sqlx::{Row, SqlitePool};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use argo_types::{ArgoError, ArgoResult};

use super::{DepKind, DepRow, CYCLE_DETECTION_EDGE_CAP};

pub(super) async fn fetch_rows(
    pool: &SqlitePool,
    column: &str,
    value: &str,
) -> ArgoResult<Vec<DepRow>> {
    let sql = format!(
        "SELECT a_worker_id, b_worker_id, kind, declared_at, note
         FROM worker_dependency
         WHERE {column} = ?
         ORDER BY kind ASC, a_worker_id ASC, b_worker_id ASC, declared_at ASC"
    );
    let rs = sqlx::query(&sql)
        .bind(value)
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("worker_dependency fetch: {e}")))?;
    rs.into_iter().map(row_into_dep).collect()
}

pub(super) fn row_into_dep(row: sqlx::sqlite::SqliteRow) -> ArgoResult<DepRow> {
    let kind: String = row.get("kind");
    Ok(DepRow {
        a_worker_id: row.get("a_worker_id"),
        b_worker_id: row.get("b_worker_id"),
        kind: DepKind::parse(&kind)?,
        declared_at: row.get("declared_at"),
        note: row.get("note"),
    })
}

/// Forward (out-edge) adjacency for one `kind`, used by cycle detection.
async fn forward_adjacency(
    pool: &SqlitePool,
    kind: DepKind,
) -> ArgoResult<BTreeMap<String, Vec<String>>> {
    let rs = sqlx::query(
        "SELECT a_worker_id, b_worker_id FROM worker_dependency
         WHERE kind = ?
         ORDER BY a_worker_id ASC, b_worker_id ASC
         LIMIT ?",
    )
    .bind(kind.as_str())
    .bind(CYCLE_DETECTION_EDGE_CAP as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_dependency adjacency: {e}")))?;
    let mut adj: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in rs {
        let a: String = r.get("a_worker_id");
        let b: String = r.get("b_worker_id");
        adj.entry(a).or_default().push(b);
    }
    Ok(adj)
}

/// Inbound (in-edge) adjacency across all kinds. Keyed on `b_worker_id`,
/// values are the set of `a` ids that declared a dependency on `b`.
/// Used by `impact`.
pub(super) async fn inbound_adjacency(
    pool: &SqlitePool,
) -> ArgoResult<BTreeMap<String, BTreeSet<String>>> {
    let rs = sqlx::query(
        "SELECT a_worker_id, b_worker_id FROM worker_dependency
         ORDER BY b_worker_id ASC, a_worker_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_dependency inbound: {e}")))?;
    let mut adj: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in rs {
        let a: String = r.get("a_worker_id");
        let b: String = r.get("b_worker_id");
        adj.entry(b).or_default().insert(a);
    }
    Ok(adj)
}

/// `true` if inserting `a → b` of kind `kind` would close a cycle in
/// the same-kind subgraph. Equivalent to: is there already a path
/// `b → … → a` along same-kind edges?
pub(super) async fn would_create_cycle(
    pool: &SqlitePool,
    a: &str,
    b: &str,
    kind: DepKind,
) -> ArgoResult<bool> {
    if a == b {
        return Ok(true);
    }
    let adj = forward_adjacency(pool, kind).await?;
    Ok(reachable(&adj, b, a))
}

/// Render the cycle path the operator's edge would close, in the form
/// `a→b→…→a`. Used for the human-readable error message. Falls back to
/// `a→b→a` when the same-kind subgraph is empty (e.g. self-edge guard).
pub(super) async fn cycle_path(
    pool: &SqlitePool,
    a: &str,
    b: &str,
    kind: DepKind,
) -> ArgoResult<Vec<String>> {
    let adj = forward_adjacency(pool, kind).await?;
    let inner = path_between(&adj, b, a).unwrap_or_else(|| vec![b.to_string(), a.to_string()]);
    let mut full = Vec::with_capacity(inner.len() + 1);
    full.push(a.to_string());
    full.extend(inner);
    Ok(full)
}

/// Standard reachability over a forward adjacency map (DFS).
fn reachable(adj: &BTreeMap<String, Vec<String>>, from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    let mut stack: Vec<&str> = vec![from];
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    seen.insert(from);
    while let Some(node) = stack.pop() {
        let Some(nexts) = adj.get(node) else {
            continue;
        };
        for next in nexts {
            if next == to {
                return true;
            }
            if seen.insert(next.as_str()) {
                stack.push(next.as_str());
            }
        }
    }
    false
}

/// Recover the path `from → … → to` over the forward adjacency, if any.
/// Returns `None` when no path exists. Used for the cycle-error message.
fn path_between(adj: &BTreeMap<String, Vec<String>>, from: &str, to: &str) -> Option<Vec<String>> {
    let mut parents: BTreeMap<String, String> = BTreeMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(from.to_string());
    let mut found = from == to;
    while let Some(node) = queue.pop_front() {
        if node == to {
            found = true;
            break;
        }
        let Some(nexts) = adj.get(&node) else {
            continue;
        };
        for next in nexts {
            if !parents.contains_key(next) && next != from {
                parents.insert(next.clone(), node.clone());
                if next == to {
                    found = true;
                    queue.clear();
                    break;
                }
                queue.push_back(next.clone());
            }
        }
    }
    if !found {
        return None;
    }
    let mut path = vec![to.to_string()];
    let mut cursor = to.to_string();
    while cursor != from {
        let parent = parents.get(&cursor)?.clone();
        path.push(parent.clone());
        cursor = parent;
    }
    path.reverse();
    Some(path)
}
