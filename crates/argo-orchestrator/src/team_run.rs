//! Supervisor engine — run a [`TeamPlan`] with parallel fan-out +
//! dependency-layer ordering (Supervisor model, DEC-022).
//!
//! [`Supervisor::run_team`] drives a plan to completion:
//!
//! 1. Compute topological layers from `depends_on`
//!    ([`TeamPlan::layers`]).
//! 2. For each layer, spawn every *eligible* step concurrently under a
//!    bounded-concurrency cap (a [`tokio::sync::Semaphore`]), then
//!    **barrier** — await the whole layer before starting the next.
//! 3. A step any of whose dependencies **failed or were skipped** is
//!    itself **skipped** (recorded, never hung) so a failure drops its
//!    dependents instead of deadlocking the run.
//! 4. Each step that actually executes is persisted as an
//!    orchestration-run row (a `provider='orchestrator'` session row
//!    — the *existing* table the `/orchestration` dashboard reads; no
//!    new migration), tenant-scoped, via the injected [`StepExecutor`].
//! 5. The gathered [`StepOutcome`]s are returned in a [`TeamResult`]
//!    for the lead to synthesize.
//!
//! The "how a delegated worker actually does the work" is abstracted
//! behind [`StepExecutor`] so the engine stays vertical-agnostic and
//! testable. The shipped consumer (the `POST
//! /api/v1/orchestration/team-runs` gateway route) wires
//! [`PersistingExecutor`], which writes the child run row + the
//! lead's synthesis row to the orchestrator store. Swapping in a
//! real LLM-backed delegation executor later is additive — it
//! implements the same trait.
//!
//! `current_handoff_id` is threaded into every [`StepContext`] so the
//! whole team run hangs off the lead's active handoff chain and the
//! audit subtree stays coherent (the same property that made Claim 2
//! LIVE).
use std::collections::BTreeMap;
use std::sync::Arc;

use argo_identity::WorkerId;
use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::team_plan::{StepId, TeamPlan};

/// Default ceiling on how many delegations run at once within a layer.
/// Mirrors the cap discipline used elsewhere in the crate; override
/// via [`Supervisor::with_concurrency`].
pub const DEFAULT_TEAM_CONCURRENCY: usize = 8;

/// Per-step context handed to a [`StepExecutor`]. Carries the tenant
/// (every primitive is multi-tenant by construction) and the lead's
/// active handoff id so the child's work joins the same audit chain.
#[derive(Debug, Clone)]
pub struct StepContext {
    /// Tenant the whole team run belongs to.
    pub tenant_id: TenantId,
    /// Lead worker coordinating the run.
    pub lead: WorkerId,
    /// Worker this step delegates to.
    pub to: WorkerId,
    /// Stable step id within the plan.
    pub step_id: StepId,
    /// The delegated task text.
    pub task: String,
    /// The lead's active handoff id, if any — threads the team run
    /// onto the existing handoff chain (`ToolContext.current_handoff_id`).
    pub current_handoff_id: Option<String>,
    /// Unique id for this whole team run, generated once at the top of
    /// [`Supervisor::run_team`] and shared by every step + the lead's
    /// synthesis. Encoded into the persisted row title (`[team:<id>]`)
    /// so the `/orchestration` view can group concurrent team runs into
    /// separate trees from the title prefix alone — the read endpoint
    /// exposes no per-run id column, and this needs no migration.
    pub team_run_id: String,
}

/// Lifecycle of one [`DelegationStep`](crate::DelegationStep) inside a
/// team run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// The delegated worker completed the step.
    Completed,
    /// The executor returned an error for the step.
    Failed,
    /// A dependency failed or was skipped, so this step never ran.
    Skipped,
}

/// Result of one step after the run settles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepOutcome {
    /// Which step.
    pub step_id: StepId,
    /// Terminal status.
    pub status: StepStatus,
    /// Executor output on success; the error string on failure; the
    /// blocking dependency on a skip.
    pub detail: String,
    /// The orchestration-run row id this step persisted, when it ran.
    pub run_id: Option<String>,
}

/// The aggregate result of a team run, for the lead to synthesize.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamResult {
    /// The orchestration-run row id of the lead's synthesis row.
    pub synthesis_run_id: String,
    /// One outcome per step, in plan order.
    pub outcomes: Vec<StepOutcome>,
}

impl TeamResult {
    /// `true` iff every step completed (none failed or skipped).
    #[must_use]
    pub fn all_completed(&self) -> bool {
        self.outcomes
            .iter()
            .all(|o| matches!(o.status, StepStatus::Completed))
    }

    /// Count of completed steps.
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| matches!(o.status, StepStatus::Completed))
            .count()
    }
}

/// Pluggable per-step worker. Implementors persist + execute one
/// delegation and may persist the lead's synthesis row. Kept as a
/// trait so the engine carries no vertical/runtime coupling and tests
/// can inject deterministic executors.
#[async_trait]
pub trait StepExecutor: Send + Sync {
    /// Execute one delegation. Return `Ok((run_id, detail))` where
    /// `run_id` is the persisted orchestration-run row id; an `Err`
    /// marks the step [`StepStatus::Failed`] and drops its dependents.
    async fn execute(&self, ctx: &StepContext) -> ArgoResult<(String, String)>;

    /// Persist the lead's synthesis run row once all steps settle.
    /// Returns its run id. `team_run_id` is the run-wide id shared with
    /// every step so the synthesis row's title groups under the same
    /// tree. The default records nothing and returns a fresh uuid —
    /// production executors override to write the row.
    async fn synthesize(
        &self,
        _plan: &TeamPlan,
        _tenant_id: TenantId,
        _team_run_id: &str,
    ) -> ArgoResult<String> {
        Ok(uuid::Uuid::now_v7().to_string())
    }
}

/// Run-invariant borrows threaded unchanged through every layer of a
/// single [`Supervisor::run_team`] call — the lead's plan, the tenant,
/// the active handoff id, and the run-wide team id. Bundled into one
/// struct so [`Supervisor::run_layer`] stays under the argument-count
/// lint instead of taking each separately.
struct LayerInputs<'a> {
    plan: &'a TeamPlan,
    tenant_id: TenantId,
    current_handoff_id: &'a Option<String>,
    team_run_id: &'a str,
}

/// Drives [`TeamPlan`]s with bounded-concurrency fan-out + dependency
/// layers. Generic over the [`StepExecutor`] so the same engine serves
/// the gateway route and the tests.
pub struct Supervisor<E: StepExecutor> {
    executor: Arc<E>,
    concurrency: usize,
}

impl<E: StepExecutor> Supervisor<E> {
    /// Build a supervisor over an executor with the default cap.
    #[must_use]
    pub fn new(executor: Arc<E>) -> Self {
        Self {
            executor,
            concurrency: DEFAULT_TEAM_CONCURRENCY,
        }
    }

    /// Override the per-layer concurrency cap (clamped to ≥ 1).
    #[must_use]
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    /// Run the plan to completion.
    ///
    /// Layers run in topological order; within a layer, eligible steps
    /// run concurrently (bounded) and the layer barriers before the
    /// next. A step with a failed/skipped dependency is skipped, never
    /// awaited — so failures cannot deadlock the run.
    ///
    /// # Errors
    ///
    /// Propagates [`TeamPlan::layers`] validation errors (cycle,
    /// dup/unknown step id) and any error from the executor's
    /// `synthesize`.
    pub async fn run_team(
        &self,
        plan: &TeamPlan,
        tenant_id: TenantId,
        current_handoff_id: Option<String>,
    ) -> ArgoResult<TeamResult> {
        let layers = plan.layers()?;
        // One id for the whole run, shared by every step + the lead's
        // synthesis. Encoded into each persisted row title so the
        // dashboard groups concurrent team runs into separate trees
        // (uuid v7 is time-ordered → stable, sortable, no migration).
        let team_run_id = uuid::Uuid::now_v7().to_string();
        let steps: BTreeMap<&StepId, &crate::team_plan::DelegationStep> =
            plan.steps.iter().map(|s| (&s.id, s)).collect();
        let mut outcomes: BTreeMap<StepId, StepOutcome> = BTreeMap::new();
        let inputs = LayerInputs {
            plan,
            tenant_id,
            current_handoff_id: &current_handoff_id,
            team_run_id: &team_run_id,
        };

        for layer in &layers {
            let layer_results = self.run_layer(layer, &steps, &outcomes, &inputs).await;
            for outcome in layer_results {
                outcomes.insert(outcome.step_id.clone(), outcome);
            }
        }

        let synthesis_run_id = self.executor.synthesize(plan, tenant_id, &team_run_id).await?;
        // Re-emit outcomes in original plan order for a stable wire shape.
        let ordered = plan
            .steps
            .iter()
            .filter_map(|s| outcomes.remove(&s.id))
            .collect();
        Ok(TeamResult {
            synthesis_run_id,
            outcomes: ordered,
        })
    }

    /// Run one topological layer: skip steps whose deps didn't all
    /// complete, then fan the rest out concurrently under the cap and
    /// barrier. Returns this layer's outcomes.
    async fn run_layer(
        &self,
        layer: &[StepId],
        steps: &BTreeMap<&StepId, &crate::team_plan::DelegationStep>,
        settled: &BTreeMap<StepId, StepOutcome>,
        inputs: &LayerInputs<'_>,
    ) -> Vec<StepOutcome> {
        let mut runnable: Vec<&crate::team_plan::DelegationStep> = Vec::new();
        let mut outcomes: Vec<StepOutcome> = Vec::new();
        for id in layer {
            let Some(step) = steps.get(id) else { continue };
            if let Some(blocker) = first_unmet_dep(&step.depends_on, settled) {
                outcomes.push(skip(id.clone(), &blocker));
            } else {
                runnable.push(step);
            }
        }
        let sem = Arc::new(Semaphore::new(self.concurrency));
        let mut tasks = Vec::with_capacity(runnable.len());
        for step in runnable {
            let ctx = StepContext {
                tenant_id: inputs.tenant_id,
                lead: inputs.plan.lead,
                to: step.to,
                step_id: step.id.clone(),
                task: step.task.clone(),
                current_handoff_id: inputs.current_handoff_id.clone(),
                team_run_id: inputs.team_run_id.to_string(),
            };
            let exec = Arc::clone(&self.executor);
            let sem = Arc::clone(&sem);
            tasks.push(async move {
                // Permit is held for the step's lifetime → bounds the
                // number of concurrent in-flight delegations per layer.
                let _permit = sem.acquire_owned().await;
                let id = ctx.step_id.clone();
                match exec.execute(&ctx).await {
                    Ok((run_id, detail)) => completed(id, run_id, detail),
                    Err(e) => failed(id, &e.to_string()),
                }
            });
        }
        outcomes.extend(futures::future::join_all(tasks).await);
        outcomes
    }
}

/// Return the first dependency that did not [`StepStatus::Completed`]
/// (failed, skipped, or — defensively — absent), else `None`.
fn first_unmet_dep(
    depends_on: &[StepId],
    settled: &BTreeMap<StepId, StepOutcome>,
) -> Option<String> {
    for dep in depends_on {
        match settled.get(dep) {
            Some(o) if matches!(o.status, StepStatus::Completed) => {}
            _ => return Some(dep.to_string()),
        }
    }
    None
}

fn completed(step_id: StepId, run_id: String, detail: String) -> StepOutcome {
    StepOutcome {
        step_id,
        status: StepStatus::Completed,
        detail,
        run_id: Some(run_id),
    }
}

fn failed(step_id: StepId, detail: &str) -> StepOutcome {
    StepOutcome {
        step_id,
        status: StepStatus::Failed,
        detail: detail.to_string(),
        run_id: None,
    }
}

fn skip(step_id: StepId, blocker: &str) -> StepOutcome {
    StepOutcome {
        step_id,
        status: StepStatus::Skipped,
        detail: format!("dependency '{blocker}' did not complete"),
        run_id: None,
    }
}

/// Assert (debug-only) that every plan step appears exactly once in
/// the computed layers — a cheap internal invariant guard reused by
/// tests. Public-in-crate so the test module can call it.
#[cfg(test)]
pub(crate) fn layers_cover_all(plan: &TeamPlan, layers: &[Vec<StepId>]) -> bool {
    let flat: std::collections::BTreeSet<&StepId> = layers.iter().flatten().collect();
    flat.len() == plan.steps.len() && plan.steps.iter().all(|s| flat.contains(&s.id))
}

#[cfg(test)]
#[path = "team_run_tests.rs"]
mod tests;
