//! `argo-cron` — cron expressions + tenant-aware scheduler (§22).
//!
//! Three pieces:
//!
//! - **[`CronExpr`]** — five-field POSIX-ish cron expression
//!   parser + match against a [`chrono::DateTime<chrono::Utc>`].
//!   Day-of-month / day-of-week follow standard cron's "either
//!   field matches" semantics when both are non-wildcard.
//! - **[`CronJob`]** + **[`JobStore`]** — `(schedule, prompt,
//!   target)` plus `last_fired_at` bookkeeping; tenant-isolated by
//!   `(tenant_id, name)`. The shipped [`InMemoryJobStore`] is used
//!   in tests + the walking skeleton; production stores plug into
//!   `argo-store` or the workspace's `cron/jobs/` directory.
//! - **[`Scheduler`]** — stateless `tick(now)` that filters due
//!   jobs and dispatches them to a [`JobRunner`]. The surrounding
//!   daemon (`argo-cli`'s daemon mode + `argo-gateway`) calls
//!   `tick` once per minute under a file lock so multiple gateway
//!   processes never double-fire a job.
//!
//! ```
//! use argo_cron::{CronExpr, CronJob, CronTarget, InMemoryJobStore, JobStore};
//! use argo_types::TenantId;
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! let schedule = CronExpr::parse("0 8 * * 1-5").unwrap();
//! let job = CronJob::new(
//!     TenantId::new(),
//!     "morning-summary",
//!     schedule,
//!     "Summarize my unread Slack DMs.",
//!     CronTarget { platform: "telegram".into(), chat: "abc".into() },
//! );
//! let store = InMemoryJobStore::new();
//! store.save(&job).await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod expr;
mod job;
mod scheduler;
mod system_registry;
mod target;
mod tick_lock;

#[cfg(test)]
mod tests;

pub use expr::{CronExpr, CronParseError};
pub use job::{CronJob, CronTarget, InMemoryJobStore, JobStore};
pub use scheduler::{JobRunner, Scheduler, TickReport};
pub use system_registry::{
    SystemJobContext, SystemJobHandler, SystemJobOutcome, SystemJobRegistry,
    SystemJobRegistryBuilder,
};
pub use target::{
    system_job_name, system_target, CANONICAL_SYSTEM_JOBS, SYSTEM_JOB_AB_PROMOTION,
    SYSTEM_JOB_DREAMS_CURATION, SYSTEM_JOB_FORGE_OPTIMIZE, SYSTEM_JOB_SKILL_QUALITY_RECOMPUTE,
    SYSTEM_PLATFORM,
};
pub use tick_lock::{TickLock, TickLockError};
