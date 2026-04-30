//! Scheduler tick — picks due jobs and dispatches them to a
//! [`JobRunner`].
//!
//! The loop is intentionally simple: every minute the surrounding
//! daemon calls [`Scheduler::tick`] with the current time. The
//! scheduler asks the [`JobStore`] for all jobs, filters by
//! [`CronJob::is_due`], hands each due job to the runner, and (on
//! success) records the fire timestamp so the same minute doesn't
//! fire twice.
//!
//! Multi-tenant safety: tenants are isolated by the `(tenant_id,
//! name)` uniqueness contract on [`JobStore`]. Schedulers can run
//! one-per-tenant or one global; the file-lock wrapper that lives
//! with the daemon is the only thing that needs to coordinate.
use crate::job::{CronJob, JobStore};
use argo_types::ArgoResult;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// What runs when a job fires.
///
/// Real impls spawn an `argo-core::Agent` against the prompt and
/// hand the resulting message to a `argo-gateway` adapter for
/// delivery to `target.platform` / `target.chat`. Tests just
/// record what was asked of them.
#[async_trait]
pub trait JobRunner: Send + Sync {
    /// Execute the job. Returning `Ok(())` causes the scheduler to
    /// mark the job fired; returning an error leaves the cursor
    /// untouched so the next tick will retry.
    async fn run(&self, job: &CronJob) -> ArgoResult<()>;
}

/// Outcome of a single tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickReport {
    /// Job ids the scheduler attempted to fire.
    pub attempted: Vec<uuid::Uuid>,
    /// Job ids that fired successfully.
    pub fired: Vec<uuid::Uuid>,
    /// Job ids whose runner returned an error.
    pub errors: Vec<uuid::Uuid>,
}

/// Stateless scheduler — all persistent state lives in the
/// [`JobStore`].
#[derive(Debug, Default, Clone, Copy)]
pub struct Scheduler;

impl Scheduler {
    /// Run one tick: fetch jobs, filter due ones, dispatch.
    ///
    /// Returns the per-job outcome so the surrounding daemon can
    /// log + alert. The runner is awaited sequentially per tick;
    /// concurrent dispatch is the daemon's call (jobs may have
    /// per-tenant fairness or rate limits).
    pub async fn tick(
        self,
        store: &dyn JobStore,
        runner: &dyn JobRunner,
        now: DateTime<Utc>,
    ) -> ArgoResult<TickReport> {
        let jobs = store.list_all().await?;
        let mut report = TickReport {
            attempted: Vec::new(),
            fired: Vec::new(),
            errors: Vec::new(),
        };
        for j in jobs {
            if !j.is_due(now) {
                continue;
            }
            report.attempted.push(j.id);
            match runner.run(&j).await {
                Ok(()) => {
                    store.record_fire(j.id, now).await?;
                    report.fired.push(j.id);
                }
                Err(_) => report.errors.push(j.id),
            }
        }
        Ok(report)
    }
}
