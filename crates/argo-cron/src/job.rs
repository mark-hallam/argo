//! Cron job + the storage seam.
//!
//! A [`CronJob`] is a `(schedule, prompt, target)` triple plus the
//! enabled flag and `last_fired_at` bookkeeping the scheduler uses
//! to suppress double-fires inside the same minute. Persistence is
//! abstracted via the [`JobStore`] trait — a real impl writes JSON
//! files under `<workspace>/cron/jobs/`; the shipped
//! [`InMemoryJobStore`] is used in tests + the walking skeleton.
use crate::expr::CronExpr;
use argo_types::TenantId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Where a cron job's output should be delivered after the agent
/// runs the prompt. The string fields are intentionally opaque to
/// this crate — the gateway adapter (Slack/Telegram/Discord/email)
/// owns the format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CronTarget {
    /// Adapter name — e.g. `"telegram"`, `"slack"`, `"discord"`,
    /// `"matrix"`, `"email"`.
    pub platform: String,
    /// Adapter-specific destination — channel id, chat id, mailbox.
    pub chat: String,
}

/// A scheduled job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    /// Stable id (UUIDv7).
    pub id: Uuid,
    /// Owning tenant.
    pub tenant_id: TenantId,
    /// Job name (unique within tenant).
    pub name: String,
    /// Cron schedule.
    pub schedule: CronExpr,
    /// Prompt the agent runs when the job fires.
    pub prompt: String,
    /// Where to deliver the output.
    pub target: CronTarget,
    /// `false` suppresses firing without deleting the job.
    pub enabled: bool,
    /// Last successful fire — used to dedupe within a one-minute
    /// match window.
    #[serde(default)]
    pub last_fired_at: Option<DateTime<Utc>>,
}

impl CronJob {
    /// Build a fresh job.
    #[must_use]
    pub fn new(
        tenant_id: TenantId,
        name: impl Into<String>,
        schedule: CronExpr,
        prompt: impl Into<String>,
        target: CronTarget,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            tenant_id,
            name: name.into(),
            schedule,
            prompt: prompt.into(),
            target,
            enabled: true,
            last_fired_at: None,
        }
    }

    /// `true` when `now` matches the schedule, the job is enabled,
    /// and we haven't already fired it within the same minute.
    #[must_use]
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled {
            return false;
        }
        if !self.schedule.matches(now) {
            return false;
        }
        if let Some(last) = self.last_fired_at {
            if same_minute(last, now) {
                return false;
            }
        }
        true
    }
}

fn same_minute(a: DateTime<Utc>, b: DateTime<Utc>) -> bool {
    a.format("%Y-%m-%dT%H:%M").to_string() == b.format("%Y-%m-%dT%H:%M").to_string()
}

/// Job-store seam.
///
/// Listing returns a snapshot — implementations are expected to
/// return jobs in stable order so tests on `Vec<CronJob>` are
/// deterministic. Writes are last-writer-wins on `(tenant, name)`.
#[async_trait]
pub trait JobStore: Send + Sync {
    /// All jobs across all tenants (used by [`crate::Scheduler::tick`]).
    async fn list_all(&self) -> argo_types::ArgoResult<Vec<CronJob>>;
    /// Persist a new or updated job.
    async fn save(&self, job: &CronJob) -> argo_types::ArgoResult<()>;
    /// Update the `last_fired_at` cursor (called once a job fires).
    async fn record_fire(
        &self,
        job_id: Uuid,
        fired_at: DateTime<Utc>,
    ) -> argo_types::ArgoResult<()>;
    /// Remove a job by id.
    async fn delete(&self, job_id: Uuid) -> argo_types::ArgoResult<()>;
}

/// Test-only in-memory job store.
#[derive(Debug, Default, Clone)]
pub struct InMemoryJobStore {
    inner: Arc<RwLock<Vec<CronJob>>>,
}

impl InMemoryJobStore {
    /// Empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl JobStore for InMemoryJobStore {
    async fn list_all(&self) -> argo_types::ArgoResult<Vec<CronJob>> {
        let mut v = self.inner.read().clone();
        v.sort_by(|a, b| a.tenant_id.0.cmp(&b.tenant_id.0).then(a.name.cmp(&b.name)));
        Ok(v)
    }

    async fn save(&self, job: &CronJob) -> argo_types::ArgoResult<()> {
        let mut g = self.inner.write();
        if let Some(slot) = g
            .iter_mut()
            .find(|j| j.tenant_id == job.tenant_id && j.name == job.name)
        {
            *slot = job.clone();
        } else {
            g.push(job.clone());
        }
        Ok(())
    }

    async fn record_fire(
        &self,
        job_id: Uuid,
        fired_at: DateTime<Utc>,
    ) -> argo_types::ArgoResult<()> {
        let mut g = self.inner.write();
        if let Some(j) = g.iter_mut().find(|j| j.id == job_id) {
            j.last_fired_at = Some(fired_at);
        }
        Ok(())
    }

    async fn delete(&self, job_id: Uuid) -> argo_types::ArgoResult<()> {
        let mut g = self.inner.write();
        g.retain(|j| j.id != job_id);
        Ok(())
    }
}
