//! Worker dependency graph + cycle detection
//! (Stream `claude-worker-dependency-tracking`, V1.1).
//!
//! Real org delegations have dependencies: the AR clerk depends on the
//! accountant for tax-coded ledger pulls; the recruiter-screener depends
//! on the calendaring worker for slot booking. `handoff_routing` tracks
//! runtime weights; this module records *declared intent* — the
//! structural lens the operator reaches for when a worker breaks.
//!
//! The semantics are: row `(a, b, kind)` means **`a` depends on `b`** in
//! the relation `kind`. So:
//!
//! * `list --direction out` for `X` returns rows where `a = X`
//!   (`X`'s dependencies — what `X` needs).
//! * `list --direction in` for `X` returns rows where `b = X`
//!   (the workers depending on `X` — *X*'s dependents).
//! * `impact X` walks the in-edge graph from `X`: every worker that
//!   directly or transitively depends on `X`. The brief calls this
//!   the "downstream impact" of `X`'s outage.
//!
//! Cycles within one `kind` are rejected at insert time via 3-color
//! DFS over same-kind edges. Cross-kind cycles are allowed: data and
//! approval can ping-pong, that's a real org pattern.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use argo_types::{ArgoError, ArgoResult};

#[path = "worker_deps_graph.rs"]
mod graph;

const WORKER_DEPENDENCY_SCHEMA: &str = include_str!("../migrations/98__worker_dependency.sql");

/// Hard cap on edges considered during cycle detection. Mirrors the
/// brief's "10k edges to avoid pathological DFS"; a real workspace
/// shouldn't approach this.
pub const CYCLE_DETECTION_EDGE_CAP: usize = 10_000;

/// Hard cap on impact-walk depth. Mirrors the brief's "depth-limited
/// to 8 levels"; deeper organisations almost certainly have a cycle
/// or pathological declared-dependency sprawl that should be reviewed.
pub const IMPACT_MAX_DEPTH: u32 = 8;

/// Kind of dependency between two workers. Stored as the lowercase
/// token on `worker_dependency.kind`. Future variants land additively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepKind {
    /// `a` consumes data produced by `b` (e.g. AR clerk → accountant).
    Data,
    /// `a` requires `b`'s approval for some action class.
    Approval,
    /// `a` hands off ongoing work items to `b`.
    Handoff,
    /// `a` relies on a skill `b` provides (and only `b` provides).
    Skill,
}

impl DepKind {
    /// Parse the operator-facing token used by `--kind <kind>`.
    pub fn parse(s: &str) -> ArgoResult<Self> {
        match s {
            "data" => Ok(Self::Data),
            "approval" => Ok(Self::Approval),
            "handoff" => Ok(Self::Handoff),
            "skill" => Ok(Self::Skill),
            _ => Err(ArgoError::Provider(format!(
                "invalid --kind '{s}': expected data|approval|handoff|skill"
            ))),
        }
    }

    /// String form persisted into `worker_dependency.kind`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Approval => "approval",
            Self::Handoff => "handoff",
            Self::Skill => "skill",
        }
    }

    /// Every variant — used by the workspace graph dump that fans out
    /// across kinds. Update if the enum gains a variant.
    pub const ALL: [Self; 4] = [Self::Data, Self::Approval, Self::Handoff, Self::Skill];
}

/// One persisted dependency row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepRow {
    /// The dependent (`a` depends on `b`).
    pub a_worker_id: String,
    /// The dependency (`a` depends on `b`).
    pub b_worker_id: String,
    /// Relation kind.
    pub kind: DepKind,
    /// Micros since epoch when the row was inserted.
    pub declared_at: i64,
    /// Optional operator note.
    pub note: Option<String>,
}

/// One node visited by an impact walk, paired with its hop distance
/// from the seed worker. Sorted by `(depth, worker_id)` so the CLI
/// renders deterministically.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ImpactedWorker {
    /// Hop distance (≥ 1; the seed itself is excluded).
    pub depth: u32,
    /// Worker id of the impacted dependent.
    pub worker_id: String,
}

/// Direction filter for `list_for_worker`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Outbound: rows where `a = worker` (the worker's dependencies).
    Out,
    /// Inbound: rows where `b = worker` (workers that depend on this one).
    In,
    /// Union of `Out` and `In`.
    Both,
}

impl Direction {
    /// Parse the `--direction` token. Defaults are caller's job.
    pub fn parse(s: &str) -> ArgoResult<Self> {
        match s {
            "out" => Ok(Self::Out),
            "in" => Ok(Self::In),
            "both" => Ok(Self::Both),
            _ => Err(ArgoError::Provider(format!(
                "invalid --direction '{s}': expected in|out|both"
            ))),
        }
    }
}

/// Apply the dep schema idempotently. Safe to call from every entry point.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    for sql in crate::migrations::split_sql_statements(WORKER_DEPENDENCY_SCHEMA) {
        sqlx::query(&sql)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("worker_dependency schema: {e}")))?;
    }
    Ok(())
}

/// Insert one dependency row after rejecting same-kind cycles. Self-edges
/// (`a == b`) are also rejected — a worker can't depend on itself.
/// Returns `Ok(true)` on first insert, `Ok(false)` if the (a, b, kind)
/// row already exists (idempotent re-add updates `note` + `declared_at`).
pub async fn add_dependency(
    pool: &SqlitePool,
    a: &str,
    b: &str,
    kind: DepKind,
    note: Option<&str>,
) -> ArgoResult<bool> {
    if a.is_empty() || b.is_empty() {
        return Err(ArgoError::Provider(
            "worker_deps: worker ids must not be empty".into(),
        ));
    }
    if a == b {
        return Err(ArgoError::Provider(format!(
            "worker_deps: self-dependency rejected (a == b == {a})"
        )));
    }
    ensure_schema(pool).await?;
    if graph::would_create_cycle(pool, a, b, kind).await? {
        let path = graph::cycle_path(pool, a, b, kind).await?;
        let rendered = path.join("→");
        return Err(ArgoError::Provider(format!("cycle detected: {rendered}")));
    }
    let now = Utc::now().timestamp_micros();
    let res = sqlx::query(
        "INSERT INTO worker_dependency (a_worker_id, b_worker_id, kind, declared_at, note)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(a_worker_id, b_worker_id, kind)
         DO UPDATE SET declared_at = excluded.declared_at, note = excluded.note",
    )
    .bind(a)
    .bind(b)
    .bind(kind.as_str())
    .bind(now)
    .bind(note)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_dependency insert: {e}")))?;
    Ok(res.rows_affected() > 0 && res.last_insert_rowid() != 0)
}

/// Remove one dependency row by `(a, b, kind)`. Returns the rows-affected
/// count so callers can distinguish "removed 1" from "no such row".
pub async fn remove_dependency(
    pool: &SqlitePool,
    a: &str,
    b: &str,
    kind: DepKind,
) -> ArgoResult<u64> {
    ensure_schema(pool).await?;
    let res = sqlx::query(
        "DELETE FROM worker_dependency
         WHERE a_worker_id = ? AND b_worker_id = ? AND kind = ?",
    )
    .bind(a)
    .bind(b)
    .bind(kind.as_str())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_dependency delete: {e}")))?;
    Ok(res.rows_affected())
}

/// List dependency rows touching `worker` in the chosen direction.
/// Sorted by `(kind, counterparty, declared_at)` for deterministic output.
pub async fn list_for_worker(
    pool: &SqlitePool,
    worker: &str,
    direction: Direction,
) -> ArgoResult<Vec<DepRow>> {
    ensure_schema(pool).await?;
    let mut rows = match direction {
        Direction::Out => graph::fetch_rows(pool, "a_worker_id", worker).await?,
        Direction::In => graph::fetch_rows(pool, "b_worker_id", worker).await?,
        Direction::Both => {
            let mut out = graph::fetch_rows(pool, "a_worker_id", worker).await?;
            out.extend(graph::fetch_rows(pool, "b_worker_id", worker).await?);
            out
        }
    };
    // De-dup `Both` results: a self-edge is rejected at insert time
    // so the union is already disjoint, but defend anyway.
    let mut seen = BTreeSet::new();
    rows.retain(|r| seen.insert((r.a_worker_id.clone(), r.b_worker_id.clone(), r.kind)));
    rows.sort_by(|x, y| {
        x.kind
            .cmp(&y.kind)
            .then_with(|| x.a_worker_id.cmp(&y.a_worker_id))
            .then_with(|| x.b_worker_id.cmp(&y.b_worker_id))
            .then_with(|| x.declared_at.cmp(&y.declared_at))
    });
    Ok(rows)
}

/// Dump the entire dependency graph for the workspace, sorted as in
/// `list_for_worker`. Used by `argo worker deps graph`.
pub async fn list_all(pool: &SqlitePool) -> ArgoResult<Vec<DepRow>> {
    ensure_schema(pool).await?;
    let rs = sqlx::query(
        "SELECT a_worker_id, b_worker_id, kind, declared_at, note
         FROM worker_dependency
         ORDER BY kind ASC, a_worker_id ASC, b_worker_id ASC, declared_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_dependency list_all: {e}")))?;
    rs.into_iter().map(graph::row_into_dep).collect()
}

/// Transitive closure of workers that depend on `seed`, walking
/// inbound edges across all kinds. Sorted by `(depth, worker_id)`.
/// Capped at [`IMPACT_MAX_DEPTH`] hops. The seed itself is excluded.
pub async fn impact(pool: &SqlitePool, seed: &str) -> ArgoResult<Vec<ImpactedWorker>> {
    ensure_schema(pool).await?;
    let inbound = graph::inbound_adjacency(pool).await?;
    let mut visited: BTreeMap<String, u32> = BTreeMap::new();
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    queue.push_back((seed.to_string(), 0));
    visited.insert(seed.to_string(), 0);
    while let Some((node, depth)) = queue.pop_front() {
        if depth >= IMPACT_MAX_DEPTH {
            continue;
        }
        let Some(dependents) = inbound.get(&node) else {
            continue;
        };
        for next in dependents {
            let next_depth = depth + 1;
            let entry = visited.entry(next.clone()).or_insert(u32::MAX);
            if *entry > next_depth {
                *entry = next_depth;
                queue.push_back((next.clone(), next_depth));
            }
        }
    }
    visited.remove(seed);
    let mut out: Vec<ImpactedWorker> = visited
        .into_iter()
        .map(|(worker_id, depth)| ImpactedWorker { depth, worker_id })
        .collect();
    out.sort();
    Ok(out)
}

#[cfg(test)]
#[path = "worker_deps_tests.rs"]
mod tests;
