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
//! What this executor itself does **not** do: invoke a worker's
//! model/turn to produce the delegated output. It persists the run row
//! and returns a deterministic acknowledgement string, so the
//! orchestration topology (fan-out + dependency layers + tenant scoping
//! + audit-coherent rows) is real and observable end-to-end without
//! faking a model response. This is the honest seam, not theater: every
//! row it writes is a genuine, tenant-scoped orchestration-run row.
//!
//! The real-model follow-up (DEC-023 P3) is the gateway-layer
//! `RuntimeStepExecutor` (`crates/argo-cli/.../routes_orchestration_team_llm.rs`),
//! which **composes** this executor: it reuses [`PersistingExecutor::record_step_run`]
//! (so the same tenant-scoped, audit-coherent row + `[team:<id>]` title
//! still land — topology unchanged) and replaces only the deterministic
//! `detail` with genuine model output, falling back to
//! [`PersistingExecutor::deterministic_detail`] on any provider error.
//! The persistence half is owned here precisely so the model-backed
//! variant cannot drift the row shape.
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

    /// Persist the per-step child run row for `ctx` and return its run
    /// id — the **exact** tenant-scoped, `[team:<id>]`-titled,
    /// participant-tagged row [`StepExecutor::execute`] writes, factored
    /// out so a model-backed executor can reuse the persistence half
    /// verbatim (topology + audit chain unchanged) and supply its own
    /// `detail`. Returns only the run id; the caller pairs it with
    /// either real model output or [`Self::deterministic_detail`].
    ///
    /// # Errors
    ///
    /// Propagates any [`OrchestratorStore::insert_session`] error.
    pub async fn record_step_run(&self, ctx: &StepContext) -> ArgoResult<String> {
        // The child run row is tenant-scoped (Session::new threads
        // tenant_id) and titled with the run-wide team id + delegated
        // task so it reads cleanly in the /orchestration list and the
        // dashboard can group concurrent team runs into separate trees
        // from the `[team:<id>]` prefix alone (no per-run id column).
        let title = format!("[team:{}] {} → {}", ctx.team_run_id, ctx.step_id, ctx.task);
        self.record_run(ctx.tenant_id, title, Some(ctx.to)).await
    }

    /// The deterministic per-step acknowledgement `detail` string. This
    /// is the honest placeholder a non-model run returns *and* the
    /// fallback a model-backed executor emits when the provider errors —
    /// keeping a single source of truth for the "no real output" detail
    /// so a fallback reads identically to a deterministic run (no
    /// theater either way).
    #[must_use]
    pub fn deterministic_detail(ctx: &StepContext) -> String {
        // Reference the active handoff chain in the acknowledgement so
        // the lead's synthesis can see the run hung off it.
        let handoff = ctx
            .current_handoff_id
            .as_deref()
            .unwrap_or("(no active handoff)");
        format!(
            "delegated step '{}' to worker {} (handoff {})",
            ctx.step_id, ctx.to, handoff
        )
    }
}

#[async_trait]
impl StepExecutor for PersistingExecutor {
    async fn execute(&self, ctx: &StepContext) -> ArgoResult<(String, String)> {
        let run_id = self.record_step_run(ctx).await?;
        Ok((run_id, Self::deterministic_detail(ctx)))
    }

    async fn synthesize(
        &self,
        plan: &TeamPlan,
        tenant_id: TenantId,
        team_run_id: &str,
    ) -> ArgoResult<String> {
        self.record_synthesis_run(plan, tenant_id, team_run_id).await
    }
}

impl PersistingExecutor {
    /// Persist the lead's synthesis run row (the `[team:<id>] lead
    /// synthesis — <title>` row tagged with the lead worker) and return
    /// its run id — the persistence half of [`StepExecutor::synthesize`],
    /// factored out so a model-backed executor reuses the exact row
    /// without re-deriving the title or duplicating the write. The
    /// synthesis *text* (deterministic vs. model-generated) is surfaced
    /// in the step outcomes, not this row's title.
    ///
    /// # Errors
    ///
    /// Propagates any [`OrchestratorStore::insert_session`] error.
    pub async fn record_synthesis_run(
        &self,
        plan: &TeamPlan,
        tenant_id: TenantId,
        team_run_id: &str,
    ) -> ArgoResult<String> {
        self.record_synthesis_run_with_summary(plan, tenant_id, team_run_id, None)
            .await
    }

    /// Like [`Self::record_synthesis_run`] but appends an optional
    /// one-line `summary` headline to the synthesis row title (after the
    /// canonical `[team:<id>] lead synthesis — <plan title>` markers, so
    /// the `/orchestration` SPA still parses the run-id prefix + the
    /// `lead synthesis` body marker — both checks are prefix/`includes`,
    /// not exact-match). Lets a model-backed executor surface the real
    /// lead synthesis in the persisted row through the SAME single,
    /// tenant-scoped insert — no title UPDATE, no migration, no schema
    /// change. `None` reproduces [`Self::record_synthesis_run`] exactly.
    ///
    /// The caller bounds the summary length; this method only joins.
    ///
    /// # Errors
    ///
    /// Propagates any [`OrchestratorStore::insert_session`] error.
    pub async fn record_synthesis_run_with_summary(
        &self,
        plan: &TeamPlan,
        tenant_id: TenantId,
        team_run_id: &str,
        summary: Option<&str>,
    ) -> ArgoResult<String> {
        let base = format!("[team:{team_run_id}] lead synthesis — {}", plan.title);
        let title = match summary.map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => format!("{base} · {s}"),
            None => base,
        };
        self.record_run(tenant_id, title, Some(plan.lead)).await
    }
}

#[cfg(test)]
#[path = "team_persist_tests.rs"]
mod tests;
