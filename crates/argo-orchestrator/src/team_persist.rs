//! [`PersistingExecutor`] — the production [`StepExecutor`] that
//! records each delegation + the lead's synthesis as an
//! orchestration-run row.
//!
//! Each step and the synthesis are persisted as `provider =
//! 'orchestrator'` `session` rows — the **existing** table the
//! `/orchestration` dashboard
//! (`crates/argo-cli/.../routes_orchestration.rs`) already selects.
//! **No new migration, no new table** (parallel-build-safe): we reuse
//! the same schema [`crate::OrchestratorStore::insert_session`]
//! writes, so a team run's children show up in the existing runs list
//! the moment they land.
//!
//! What this executor does **not** do yet: actually invoke a worker's
//! model/turn to produce the delegated output. That is the additive
//! follow-up — a `RuntimeStepExecutor` implementing the same
//! [`StepExecutor`] trait would call the agent runner. Today the
//! executor persists the run row and returns a deterministic
//! acknowledgement string, so the orchestration topology (fan-out +
//! dependency layers + tenant scoping + audit-coherent rows) is real
//! and observable end-to-end without faking a model response. This is
//! the honest seam, not theater: every row it writes is a genuine,
//! tenant-scoped orchestration-run row.
use std::sync::Arc;

use argo_types::{ArgoResult, TenantId};
use async_trait::async_trait;

use crate::persist::OrchestratorStore;
use crate::session::Session;
use crate::team_plan::TeamPlan;
use crate::team_run::{StepContext, StepExecutor};

/// Persists team-run children + the lead synthesis as orchestrator
/// session rows. Holds an [`OrchestratorStore`] (already tenant-aware).
///
/// The engine fans delegations out genuinely concurrently within a
/// layer, but SQLite is a **single-writer** store: concurrent
/// `INSERT` (+ the idempotent schema bring-up
/// [`OrchestratorStore::insert_session`] runs) racing on one
/// connection pool can otherwise surface as transient
/// "database is locked" errors. We serialize *only the row-write*
/// behind an async mutex — the delegations themselves still run in
/// parallel, and a future LLM-backed executor's expensive model call
/// stays fully concurrent; only the brief persistence is ordered,
/// matching the database's actual concurrency model.
pub struct PersistingExecutor {
    store: Arc<OrchestratorStore>,
    write_lock: tokio::sync::Mutex<()>,
}

impl PersistingExecutor {
    /// Build over an orchestrator store.
    #[must_use]
    pub fn new(store: Arc<OrchestratorStore>) -> Self {
        Self {
            store,
            write_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Insert one `provider='orchestrator'` session row under the
    /// tenant, recording the delegated worker as a participant, and
    /// return its run id. Shared by per-step + synthesis writes; the
    /// write is serialized (single-writer SQLite) so concurrent
    /// fan-out cannot self-contend on the lock.
    async fn record_run(
        &self,
        tenant_id: TenantId,
        title: String,
        participant: Option<argo_identity::WorkerId>,
    ) -> ArgoResult<String> {
        let mut session = Session::new(tenant_id, title);
        if let Some(w) = participant {
            session.add_participant(w);
        }
        let run_id = session.id.to_string();
        let _guard = self.write_lock.lock().await;
        self.store.insert_session(&session).await?;
        Ok(run_id)
    }
}

#[async_trait]
impl StepExecutor for PersistingExecutor {
    async fn execute(&self, ctx: &StepContext) -> ArgoResult<(String, String)> {
        // The child run row is tenant-scoped (Session::new threads
        // tenant_id) and titled with the run-wide team id + delegated
        // task so it reads cleanly in the /orchestration list and the
        // dashboard can group concurrent team runs into separate trees
        // from the `[team:<id>]` prefix alone (no per-run id column).
        let title = format!("[team:{}] {} → {}", ctx.team_run_id, ctx.step_id, ctx.task);
        let run_id = self
            .record_run(ctx.tenant_id, title, Some(ctx.to))
            .await?;
        // Reference the active handoff chain in the acknowledgement so
        // the lead's synthesis can see the run hung off it.
        let handoff = ctx
            .current_handoff_id
            .as_deref()
            .unwrap_or("(no active handoff)");
        let detail = format!(
            "delegated step '{}' to worker {} (handoff {})",
            ctx.step_id, ctx.to, handoff
        );
        Ok((run_id, detail))
    }

    async fn synthesize(
        &self,
        plan: &TeamPlan,
        tenant_id: TenantId,
        team_run_id: &str,
    ) -> ArgoResult<String> {
        let title = format!("[team:{team_run_id}] lead synthesis — {}", plan.title);
        self.record_run(tenant_id, title, Some(plan.lead)).await
    }
}

#[cfg(test)]
#[path = "team_persist_tests.rs"]
mod tests;
