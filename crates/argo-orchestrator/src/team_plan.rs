//! Supervisor team-plan types + topological layering.
//!
//! A [`TeamPlan`] is a *lead* worker plus a set of
//! [`DelegationStep`]s the lead fans out. Each step names the worker
//! it delegates to (`to`), the `task`, and an optional `depends_on`
//! list referencing earlier step ids. The engine
//! ([`crate::team_run::Supervisor`]) turns `depends_on` into
//! **topological layers**: every step in layer *N* may run only after
//! all of its dependencies (which live in layers `< N`) have settled.
//!
//! This file owns the *data shape* + the pure, side-effect-free layer
//! computation so it is unit-testable without a store, a tokio
//! runtime, or any I/O. The concurrent execution lives in the sibling
//! [`crate::team_run`] (Supervisor model — DEC-022).
//!
//! ```
//! use argo_orchestrator::{TeamPlan, DelegationStep, StepId};
//! use argo_identity::WorkerId;
//!
//! let lead = WorkerId::new();
//! let a = StepId::new("a");
//! let b = StepId::new("b");
//! let plan = TeamPlan::new(lead, "research", vec![
//!     DelegationStep::new(a.clone(), WorkerId::new(), "gather"),
//!     DelegationStep::new(b.clone(), WorkerId::new(), "synthesize")
//!         .with_depends_on(vec![a.clone()]),
//! ]);
//! let layers = plan.layers().expect("acyclic");
//! assert_eq!(layers.len(), 2); // a in layer 0, b in layer 1
//! ```
use std::collections::{BTreeMap, BTreeSet};

use argo_identity::WorkerId;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};

/// Stable identifier for one [`DelegationStep`] inside a [`TeamPlan`].
///
/// Authored as an opaque string so callers can use meaningful labels
/// (`"gather-pricing"`) and so `depends_on` edges read cleanly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StepId(String);

impl StepId {
    /// Wrap a caller-supplied id.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Borrow the raw id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StepId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One sub-delegation the lead fans out to a named worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationStep {
    /// Stable id, unique within the plan.
    pub id: StepId,
    /// Worker the lead delegates this step to.
    pub to: WorkerId,
    /// What the delegated worker is asked to do.
    pub task: String,
    /// Ids of steps that must settle before this one may run. Empty
    /// means the step is eligible in the first layer.
    pub depends_on: Vec<StepId>,
}

impl DelegationStep {
    /// Build a dependency-free step.
    #[must_use]
    pub fn new(id: StepId, to: WorkerId, task: impl Into<String>) -> Self {
        Self {
            id,
            to,
            task: task.into(),
            depends_on: Vec::new(),
        }
    }

    /// Attach a `depends_on` edge list (builder style).
    #[must_use]
    pub fn with_depends_on(mut self, deps: Vec<StepId>) -> Self {
        self.depends_on = deps;
        self
    }
}

/// A lead worker coordinating a set of fan-out delegations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamPlan {
    /// The coordinating ("supervisor") worker.
    pub lead: WorkerId,
    /// Short human-facing label for the team run (operator-visible).
    pub title: String,
    /// The delegations to fan out, in author order.
    pub steps: Vec<DelegationStep>,
}

impl TeamPlan {
    /// Construct a plan.
    #[must_use]
    pub fn new(lead: WorkerId, title: impl Into<String>, steps: Vec<DelegationStep>) -> Self {
        Self {
            lead,
            title: title.into(),
            steps,
        }
    }

    /// Validate the plan: every `depends_on` id must reference a step
    /// in the plan, step ids must be unique, and the dependency graph
    /// must be acyclic. Returns the topological **layers** — each
    /// inner vec is a set of step ids whose dependencies are all in
    /// earlier layers, so the engine can run a whole layer in parallel
    /// and barrier between layers.
    ///
    /// # Errors
    ///
    /// - [`ArgoError::Invariant`] if a step id is duplicated, a
    ///   `depends_on` references an unknown id, or a cycle exists.
    pub fn layers(&self) -> ArgoResult<Vec<Vec<StepId>>> {
        let ids: BTreeSet<&StepId> = self.steps.iter().map(|s| &s.id).collect();
        if ids.len() != self.steps.len() {
            return Err(ArgoError::Invariant(
                "team plan has duplicate step ids".into(),
            ));
        }
        // remaining-deps count per step + reverse adjacency for the
        // Kahn-style layered sweep.
        let mut pending: BTreeMap<StepId, usize> = BTreeMap::new();
        let mut dependents: BTreeMap<StepId, Vec<StepId>> = BTreeMap::new();
        for step in &self.steps {
            pending.insert(step.id.clone(), step.depends_on.len());
            for dep in &step.depends_on {
                if !ids.contains(dep) {
                    return Err(ArgoError::Invariant(format!(
                        "step '{}' depends on unknown step '{}'",
                        step.id, dep
                    )));
                }
                dependents
                    .entry(dep.clone())
                    .or_default()
                    .push(step.id.clone());
            }
        }
        layered_sweep(pending, &dependents, self.steps.len())
    }
}

/// Kahn-style layered topological sort. Each iteration drains all
/// currently-ready (zero remaining-dep) nodes into one layer, then
/// decrements their dependents. A leftover node means a cycle.
fn layered_sweep(
    mut pending: BTreeMap<StepId, usize>,
    dependents: &BTreeMap<StepId, Vec<StepId>>,
    total: usize,
) -> ArgoResult<Vec<Vec<StepId>>> {
    let mut layers: Vec<Vec<StepId>> = Vec::new();
    let mut placed = 0usize;
    while placed < total {
        let ready: Vec<StepId> = pending
            .iter()
            .filter(|&(_, &n)| n == 0)
            .map(|(id, _)| id.clone())
            .collect();
        if ready.is_empty() {
            return Err(ArgoError::Invariant(
                "team plan dependency graph has a cycle".into(),
            ));
        }
        for id in &ready {
            pending.remove(id);
            if let Some(children) = dependents.get(id) {
                for child in children {
                    if let Some(n) = pending.get_mut(child) {
                        *n = n.saturating_sub(1);
                    }
                }
            }
        }
        placed += ready.len();
        layers.push(ready);
    }
    Ok(layers)
}

#[cfg(test)]
#[path = "team_plan_tests.rs"]
mod tests;
