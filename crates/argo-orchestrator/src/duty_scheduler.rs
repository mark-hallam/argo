//! Worker duty scheduler — the autonomous loop
//! (Stream `claude-worker-duty-scheduler`, V1.3).
//!
//! Workers carry a list of duties (cron + entry_action + enabled +
//! paused_until). The [`DutyScheduler`] ticks once per minute,
//! walks every active worker's duties, and fires any whose cron
//! window has elapsed since `last_fired_at`. Each fire creates a
//! [`crate::Session`] attributed to the duty via the
//! `duty_fired_session` link table; the worker's
//! [`argo_identity::AutoApprovalPolicy`] gate is honored before
//! the session is created.
//!
//! Implementation is split across three sibling files:
//!   * `duty_scheduler.rs` — record type + scheduler loop (this file)
//!   * `duty_scheduler_gate.rs` — `FireDecision`, `DutyGate`, gates
//!   * `duty_scheduler_store.rs` — SQLite CRUD over `role_duty`
//!
//! ```
//! use argo_orchestrator::duty_scheduler::FireDecision;
//! assert_eq!(FireDecision::AutoApprove.label(), "auto_approved");
//! ```
use crate::{OrchestratorStore, Session, SessionId};
use argo_identity::WorkerId;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, TimeZone, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

pub use crate::duty_scheduler_gate::{ClosedGate, DutyGate, FireDecision, OpenGate};
pub use crate::duty_scheduler_store::DutyStore;

/// Persisted duty row (mirror of the `role_duty` table).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DutyRecord {
    /// UUIDv7-stringified id.
    pub id: String,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Owning worker.
    pub worker_id: WorkerId,
    /// Cron expression (5- or 6-field, parsed by the `cron` crate).
    pub cron: String,
    /// Handler name in the worker action vocabulary.
    pub entry_action: String,
    /// Human-readable description.
    pub description: String,
    /// Disabled duties never fire.
    pub enabled: bool,
    /// Paused-until — scheduler skips while set and `> now`.
    pub paused_until: Option<DateTime<Utc>>,
    /// Last successful fire (per-window idempotency origin).
    pub last_fired_at: Option<DateTime<Utc>>,
    /// Created-at (used as cron evaluation origin when never fired).
    pub created_at: DateTime<Utc>,
}

impl DutyRecord {
    /// Fresh duty with a UUIDv7 id and `created_at = now`. Time
    /// is truncated to micros so the database round-trip (which
    /// stores `timestamp_micros`) is exact.
    #[must_use]
    pub fn new(
        tenant_id: TenantId,
        worker_id: WorkerId,
        cron: impl Into<String>,
        entry_action: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::now_v7().to_string(),
            tenant_id,
            worker_id,
            cron: cron.into(),
            entry_action: entry_action.into(),
            description: description.into(),
            enabled: true,
            paused_until: None,
            last_fired_at: None,
            created_at: trunc_to_micros(Utc::now()),
        }
    }

    /// `true` iff the cron window has elapsed since the last fire
    /// origin (`last_fired_at` if set, else `created_at`).
    pub fn should_fire(&self, now: DateTime<Utc>) -> ArgoResult<bool> {
        if !self.enabled {
            return Ok(false);
        }
        if matches!(self.paused_until, Some(until) if until > now) {
            return Ok(false);
        }
        let schedule = Schedule::from_str(&self.cron)
            .map_err(|e| ArgoError::Schema(format!("duty.cron '{}': {e}", self.cron)))?;
        let origin = self.last_fired_at.unwrap_or(self.created_at);
        match schedule.after(&origin).next() {
            Some(next) => Ok(next <= now),
            None => Ok(false),
        }
    }
}

/// Outcome of a single tick.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TickReport {
    /// Total duties evaluated.
    pub evaluated: usize,
    /// Duty ids whose cron matched and the gate auto-approved.
    pub fired: Vec<String>,
    /// Duty ids whose cron matched but the gate denied.
    pub denied_by_gate: Vec<String>,
}

/// Combines a [`DutyStore`] with an [`OrchestratorStore`] and a
/// [`DutyGate`] to produce the autonomous-loop tick.
pub struct DutyScheduler {
    duties: Arc<DutyStore>,
    orchestrator: Arc<OrchestratorStore>,
    gate: Arc<dyn DutyGate>,
}

impl DutyScheduler {
    /// Build a scheduler around the three injected dependencies.
    #[must_use]
    pub fn new(
        duties: Arc<DutyStore>,
        orchestrator: Arc<OrchestratorStore>,
        gate: Arc<dyn DutyGate>,
    ) -> Self {
        Self { duties, orchestrator, gate }
    }

    /// Walk every active duty and fire those whose cron window has
    /// elapsed since `last_fired_at` (or `created_at` when never
    /// fired). Returns the per-tick report.
    pub async fn tick(&self, now: DateTime<Utc>) -> ArgoResult<TickReport> {
        let mut report = TickReport::default();
        let active = self.duties.list_active(now).await?;
        report.evaluated = active.len();
        for duty in active {
            if !duty.should_fire(now)? {
                continue;
            }
            match self.gate.evaluate(&duty).await? {
                FireDecision::AutoApprove => {
                    let session_id = self.spawn_session(&duty, now).await?;
                    self.duties
                        .link_fire(&duty.id, &session_id, now, FireDecision::AutoApprove)
                        .await?;
                    self.duties.mark_fired(&duty.id, now).await?;
                    report.fired.push(duty.id);
                }
                other => {
                    self.duties
                        .link_fire(
                            &duty.id,
                            &SessionId::from_string("00000000-0000-0000-0000-000000000000".into()),
                            now,
                            other,
                        )
                        .await
                        .ok();
                    self.duties.mark_fired(&duty.id, now).await?;
                    report.denied_by_gate.push(duty.id);
                }
            }
        }
        Ok(report)
    }

    /// Manually fire one duty bypassing cron + gate (the
    /// `argo duty fire-now` surface).
    pub async fn fire_now(&self, duty_id: &str) -> ArgoResult<SessionId> {
        let now = Utc::now();
        let duty = self
            .duties
            .get(duty_id)
            .await?
            .ok_or_else(|| ArgoError::Schema(format!("duty '{duty_id}' not found")))?;
        let session_id = self.spawn_session(&duty, now).await?;
        self.duties
            .link_fire(&duty.id, &session_id, now, FireDecision::ManualFire)
            .await?;
        self.duties.mark_fired(&duty.id, now).await?;
        Ok(session_id)
    }

    async fn spawn_session(&self, duty: &DutyRecord, now: DateTime<Utc>) -> ArgoResult<SessionId> {
        let mut session = Session::new(
            duty.tenant_id,
            format!("duty:{}@{}", duty.entry_action, now.format("%Y-%m-%dT%H:%MZ")),
        );
        session.add_participant(duty.worker_id);
        self.orchestrator.insert_session(&session).await?;
        Ok(session.id)
    }
}

/// Spawn a tokio task that calls [`DutyScheduler::tick`] on the
/// given interval, swallowing per-tick errors via `tracing::error!`
/// so a flaky tick never kills the loop. Gateway boot binds this
/// to the [`OrchestratorStore`] + [`DutyStore`] it owns.
pub fn spawn_duty_scheduler(
    scheduler: Arc<DutyScheduler>,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let now = Utc::now();
            if let Err(err) = scheduler.tick(now).await {
                tracing::error!(target: "argo_orchestrator::duty_scheduler",
                    error = %err, "duty scheduler tick failed");
            }
        }
    })
}

fn trunc_to_micros(d: DateTime<Utc>) -> DateTime<Utc> {
    Utc.timestamp_micros(d.timestamp_micros())
        .single()
        .unwrap_or(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_store::SessionStore;
    use chrono::Duration as CDur;

    async fn open_stores() -> (Arc<DutyStore>, Arc<OrchestratorStore>) {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let duties = Arc::new(DutyStore::new(store.clone()));
        duties.ensure_schema().await.unwrap();
        let orch = Arc::new(OrchestratorStore::new(store));
        orch.ensure_schema().await.unwrap();
        (duties, orch)
    }

    fn record(cron: &str) -> DutyRecord {
        DutyRecord::new(TenantId::new(), WorkerId::new(), cron, "ping", "test duty")
    }

    #[tokio::test]
    async fn disabled_duty_never_fires() {
        let mut d = record("0 * * * * *");
        d.enabled = false;
        assert!(!d.should_fire(Utc::now()).unwrap());
    }

    #[tokio::test]
    async fn paused_duty_skipped_until_window_passes() {
        let mut d = record("0 * * * * *");
        let now = Utc::now();
        d.paused_until = Some(now + CDur::hours(1));
        assert!(!d.should_fire(now).unwrap());
        d.paused_until = Some(now - CDur::hours(1));
        assert!(!d.should_fire(now).unwrap());
        assert!(d.should_fire(now + CDur::minutes(2)).unwrap());
    }

    #[tokio::test]
    async fn cron_window_idempotent_via_last_fired_at() {
        let mut d = record("0 * * * * *");
        let base = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
        d.created_at = base;
        assert!(d.should_fire(base + CDur::minutes(2)).unwrap());
        d.last_fired_at = Some(base + CDur::minutes(2));
        assert!(!d.should_fire(base + CDur::minutes(2)).unwrap());
        assert!(d.should_fire(base + CDur::minutes(3)).unwrap());
    }

    #[tokio::test]
    async fn invalid_cron_surfaces_schema_error() {
        let d = record("not a cron");
        assert!(matches!(
            d.should_fire(Utc::now()),
            Err(ArgoError::Schema(_))
        ));
    }

    #[tokio::test]
    async fn fire_now_creates_session_attributed_to_duty() {
        let (duties, orch) = open_stores().await;
        let scheduler = DutyScheduler::new(duties.clone(), orch, Arc::new(OpenGate));
        let d = record("0 0 * * * *");
        duties.insert(&d).await.unwrap();
        let session_id = scheduler.fire_now(&d.id).await.unwrap();
        let attribution = duties.sessions_for_duty(&d.id).await.unwrap();
        assert_eq!(attribution.len(), 1);
        assert_eq!(attribution[0].0, session_id);
        assert_eq!(attribution[0].2, "manual_fire");
    }

    #[tokio::test]
    async fn tick_skips_paused_and_disabled_duties() {
        let (duties, orch) = open_stores().await;
        let scheduler = DutyScheduler::new(duties.clone(), orch, Arc::new(OpenGate));
        let mut paused = record("* * * * * *");
        paused.paused_until = Some(Utc::now() + CDur::hours(1));
        let mut disabled = record("* * * * * *");
        disabled.enabled = false;
        duties.insert(&paused).await.unwrap();
        duties.insert(&disabled).await.unwrap();
        let report = scheduler.tick(Utc::now()).await.unwrap();
        assert_eq!(report.evaluated, 0);
        assert!(report.fired.is_empty());
    }

    #[tokio::test]
    async fn duty_round_trips_through_store() {
        let (duties, _) = open_stores().await;
        let d = record("0 * * * * *");
        duties.insert(&d).await.unwrap();
        assert_eq!(duties.get(&d.id).await.unwrap().unwrap(), d);
    }

    #[tokio::test]
    async fn list_for_tenant_isolates_other_tenants() {
        let (duties, _) = open_stores().await;
        let a = TenantId::new();
        let b = TenantId::new();
        duties.insert(&DutyRecord::new(a, WorkerId::new(), "0 * * * * *", "ping", "")).await.unwrap();
        duties.insert(&DutyRecord::new(b, WorkerId::new(), "0 * * * * *", "ping", "")).await.unwrap();
        assert_eq!(duties.list_for_tenant(a).await.unwrap().len(), 1);
        assert_eq!(duties.list_for_tenant(b).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn pause_then_resume_round_trips() {
        let (duties, _) = open_stores().await;
        let d = record("0 * * * * *");
        duties.insert(&d).await.unwrap();
        duties.set_paused(&d.id, Some(Utc::now() + CDur::hours(2))).await.unwrap();
        assert!(duties.get(&d.id).await.unwrap().unwrap().paused_until.is_some());
        duties.set_paused(&d.id, None).await.unwrap();
        assert!(duties.get(&d.id).await.unwrap().unwrap().paused_until.is_none());
    }
}
