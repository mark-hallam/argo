//! DAG primitives — Workflow / TaskNode / Edge + topology
//! validator. The shape is serde-friendly so the YAML parser
//! lands by deriving Deserialize on these types.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use thiserror::Error;

/// Stable identifier for a workflow run. Caller-supplied so the
/// CLI can quote it back; defaults to a UUIDv7.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(pub String);

impl WorkflowId {
    /// Fresh UUIDv7-backed id.
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7().to_string())
    }

    /// Borrow the raw id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WorkflowId {
    fn default() -> Self {
        Self::new()
    }
}

/// Trigger that gates an edge. v1 ships `success` only;
/// `always` and `failure` follow once the executor learns to
/// branch on outcome.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// Edge fires when `from_task` succeeds.
    #[default]
    Success,
    /// Edge fires regardless of `from_task` outcome.
    Always,
    /// Edge fires only when `from_task` failed (compensation).
    Failure,
}

/// One node in the workflow DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskNode {
    /// Unique within the workflow.
    pub id: String,
    /// Worker id or persona-match string. The runner resolves the
    /// concrete worker.
    #[serde(default)]
    pub worker: String,
    /// Optional persona-match expression (e.g. `role=support`).
    /// Used when `worker` is empty.
    #[serde(default)]
    pub persona: Option<String>,
    /// Free-form key→value inputs. String values may carry
    /// `${tasks.<id>.out.<key>}` references resolved at run time.
    #[serde(default)]
    pub inputs: BTreeMap<String, serde_json::Value>,
    /// Optional output schema — informational; the runner is the
    /// source of truth on what gets emitted.
    #[serde(default)]
    pub outputs: BTreeMap<String, String>,
    /// Explicit dependency list. Equivalent to declaring an
    /// `edges:` entry with `Success` trigger; the parser folds
    /// `depends_on` into the edge list during validation.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Optional whole-workflow idempotency key (Stream
    /// `claude-checkpoint-resume`). When set, the executor
    /// looks the key up in the checkpoint store before
    /// invoking the runner — a hit reuses the cached output
    /// without re-executing the task. Mainly for skipping
    /// expensive LLM calls on retry. Distinct task ids may
    /// share a key (e.g. "summarise-2026-Q1-deals").
    #[serde(default)]
    pub cache_key: Option<String>,
}

/// One edge between tasks. The parser may synthesise edges from
/// `depends_on` shortcuts during validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    /// Upstream task id.
    pub from_task: String,
    /// Downstream task id.
    pub to_task: String,
    /// What triggers the downstream task. Defaults to `Success`.
    #[serde(default, rename = "on")]
    pub trigger: Trigger,
}

/// Top-level workflow document. Mirrors the YAML schema 1:1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workflow {
    /// Stable id (caller-supplied or generated).
    #[serde(default)]
    pub id: String,
    /// Optional human-readable description.
    #[serde(default)]
    pub description: String,
    /// All tasks in the DAG (declaration order is informational —
    /// scheduling order comes from topo sort).
    pub tasks: Vec<TaskNode>,
    /// Explicit edges. Combined with `depends_on` shortcuts during
    /// validation.
    #[serde(default)]
    pub edges: Vec<Edge>,
}

/// Topology / shape errors surfaced by [`Workflow::validate`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    /// Workflow has no tasks.
    #[error("workflow must declare at least one task")]
    Empty,
    /// Two tasks share the same id.
    #[error("duplicate task id '{0}'")]
    DuplicateTaskId(String),
    /// An edge references a task id that doesn't exist.
    #[error("edge references unknown task '{0}'")]
    UnknownTaskInEdge(String),
    /// `depends_on` list references a non-declared task.
    #[error("task '{from}' depends_on unknown task '{to}'")]
    UnknownTaskInDependsOn {
        /// The task whose `depends_on` was bad.
        from: String,
        /// The missing dependency.
        to: String,
    },
    /// DAG has a cycle.
    #[error("cycle detected: {0}")]
    Cycle(String),
}

impl Workflow {
    /// Validate that the workflow is non-empty, every task id is
    /// unique, every edge endpoint is declared, and there are no
    /// cycles. Folds `depends_on` shortcuts into the edge graph.
    ///
    /// Returns the canonicalised edge list (original `edges` plus
    /// any synthetic ones from `depends_on`) so callers don't have
    /// to merge twice.
    pub fn validate(&self) -> Result<Vec<Edge>, ValidationError> {
        if self.tasks.is_empty() {
            return Err(ValidationError::Empty);
        }
        let mut ids = HashSet::new();
        for t in &self.tasks {
            if !ids.insert(t.id.clone()) {
                return Err(ValidationError::DuplicateTaskId(t.id.clone()));
            }
        }

        // Materialise the full edge list: explicit edges + folded
        // depends_on shortcuts.
        let mut edges: Vec<Edge> = self.edges.clone();
        for t in &self.tasks {
            for dep in &t.depends_on {
                if !ids.contains(dep) {
                    return Err(ValidationError::UnknownTaskInDependsOn {
                        from: t.id.clone(),
                        to: dep.clone(),
                    });
                }
                edges.push(Edge {
                    from_task: dep.clone(),
                    to_task: t.id.clone(),
                    trigger: Trigger::Success,
                });
            }
        }
        for e in &edges {
            if !ids.contains(&e.from_task) {
                return Err(ValidationError::UnknownTaskInEdge(e.from_task.clone()));
            }
            if !ids.contains(&e.to_task) {
                return Err(ValidationError::UnknownTaskInEdge(e.to_task.clone()));
            }
        }

        // Cycle check via Kahn's algorithm. Collect indegree per
        // task; pop ready (indegree 0); decrement neighbours;
        // anything left after the pass is part of a cycle.
        let mut indegree: BTreeMap<&str, usize> =
            self.tasks.iter().map(|t| (t.id.as_str(), 0)).collect();
        let mut adj: BTreeMap<&str, Vec<&str>> =
            self.tasks.iter().map(|t| (t.id.as_str(), Vec::new())).collect();
        for e in &edges {
            *indegree.entry(e.to_task.as_str()).or_insert(0) += 1;
            adj.entry(e.from_task.as_str())
                .or_default()
                .push(e.to_task.as_str());
        }
        let mut ready: Vec<&str> = indegree
            .iter()
            .filter_map(|(id, &d)| (d == 0).then_some(*id))
            .collect();
        ready.sort();
        let mut removed: BTreeSet<&str> = BTreeSet::new();
        while let Some(id) = ready.pop() {
            if !removed.insert(id) {
                continue;
            }
            for nxt in adj.get(id).cloned().unwrap_or_default() {
                if let Some(d) = indegree.get_mut(nxt) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        ready.push(nxt);
                    }
                }
            }
        }
        if removed.len() != self.tasks.len() {
            let unresolved: Vec<&str> = self
                .tasks
                .iter()
                .map(|t| t.id.as_str())
                .filter(|id| !removed.contains(*id))
                .collect();
            return Err(ValidationError::Cycle(unresolved.join(" → ")));
        }

        Ok(edges)
    }

    /// Topological order: tasks listed earliest first that have
    /// all their dependencies satisfied. Errors when the DAG has
    /// a cycle (mirrors [`Workflow::validate`]).
    pub fn topo_order(&self) -> Result<Vec<&TaskNode>, ValidationError> {
        let edges = self.validate()?;
        let mut indegree: BTreeMap<&str, usize> =
            self.tasks.iter().map(|t| (t.id.as_str(), 0)).collect();
        let mut adj: BTreeMap<&str, Vec<&str>> =
            self.tasks.iter().map(|t| (t.id.as_str(), Vec::new())).collect();
        for e in &edges {
            *indegree.entry(e.to_task.as_str()).or_insert(0) += 1;
            adj.entry(e.from_task.as_str())
                .or_default()
                .push(e.to_task.as_str());
        }
        // Pop in declaration order (deterministic).
        let mut order = Vec::with_capacity(self.tasks.len());
        let mut visited: BTreeSet<&str> = BTreeSet::new();
        loop {
            let mut emitted = false;
            for t in &self.tasks {
                if visited.contains(t.id.as_str()) {
                    continue;
                }
                if indegree.get(t.id.as_str()).copied().unwrap_or(0) > 0 {
                    continue;
                }
                visited.insert(t.id.as_str());
                order.push(t);
                emitted = true;
                for nxt in adj.get(t.id.as_str()).cloned().unwrap_or_default() {
                    if let Some(d) = indegree.get_mut(nxt) {
                        *d = d.saturating_sub(1);
                    }
                }
            }
            if !emitted {
                break;
            }
        }
        if order.len() != self.tasks.len() {
            // Topology check already passed; if we get here the
            // graph desynced — surface as a cycle for safety.
            return Err(ValidationError::Cycle("topo_order desynced".into()));
        }
        Ok(order)
    }

    /// Lookup a task by id.
    #[must_use]
    pub fn task(&self, id: &str) -> Option<&TaskNode> {
        self.tasks.iter().find(|t| t.id == id)
    }
}

