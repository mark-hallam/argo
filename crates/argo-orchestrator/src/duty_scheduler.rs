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
//! Files touched by this brief:
//!   * `crates/argo-store/migrations/28__role_duties.sql` — schema
//!   * `crates/argo-identity/src/types.rs` — `Duty` struct extension
//!   * `crates/argo-orchestrator/src/duty_scheduler.rs` — this file
//!   * `crates/argo-orchestrator/src/lib.rs` — re-exports + spawn
//!   * `crates/argo-cli/src/handlers/duty_*.rs` — 5 CLI surfaces
//!
//! ```
//! use argo_orchestrator::duty_scheduler::FireDecision;
//! assert_eq!(FireDecision::AutoApprove.label(), "auto_approved");
//! ```
use crate::{OrchestratorStore, Session, SessionId};
use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const ROLE_DUTY_SCHEMA: &str =
    include_str!("../../argo-store/migrations/28__role_duties.sql");

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

/// Decision rendered by the AutoApprovalPolicy gate when a duty
/// fire is evaluated. Mirrors the `decision` column in
/// `duty_fired_session`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FireDecision {
    /// Gate allowed the fire to spawn a session.
    AutoApprove,
    /// Gate denied (worker policy, budget, time-window, etc.).
    DeniedGate,
    /// Duty was paused at fire time.
    DeniedPaused,
    /// Duty was disabled at fire time.
    DeniedDisabled,
    /// Manual `argo duty fire-now` invocation (bypasses cron).
    ManualFire,
}

impl FireDecision {
    /// Stable label written to the link table.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::AutoApprove => "auto_approved",
            Self::DeniedGate => "denied_gate",
            Self::DeniedPaused => "denied_paused",
            Self::DeniedDisabled => "denied_disabled",
            Self::ManualFire => "manual_fire",
        }
    }
}

/// Approval gate consulted before a duty fire becomes a Session.
/// Production callers wire this to the H-1 evaluator over the
/// worker's resolved `AutoApprovalPolicy`; tests use a stub.
#[async_trait]
pub trait DutyGate: Send + Sync {
    /// Decide whether the duty's pending fire should proceed.
    async fn evaluate(&self, duty: &DutyRecord) -> ArgoResult<FireDecision>;
}

/// `DutyGate` that auto-approves every fire — gateway-default
/// when no per-worker policy is configured.
pub struct OpenGate;

#[async_trait]
impl DutyGate for OpenGate {
    async fn evaluate(&self, _duty: &DutyRecord) -> ArgoResult<FireDecision> {
        Ok(FireDecision::AutoApprove)
    }
}

/// `DutyGate` that denies every fire — useful in tests for proving
/// the gate-honored acceptance criterion.
pub struct ClosedGate;

#[async_trait]
impl DutyGate for ClosedGate {
    async fn evaluate(&self, _duty: &DutyRecord) -> ArgoResult<FireDecision> {
        Ok(FireDecision::DeniedGate)
    }
}

/// SQLite-backed CRUD over the `role_duty` and `duty_fired_session`
/// tables.
pub struct DutyStore {
    store: Arc<SessionStore>,
}

impl DutyStore {
    /// Build over an open, migrated [`SessionStore`].
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Idempotently create the `role_duty` + `duty_fired_session`
    /// tables.
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        for stmt in ROLE_DUTY_SCHEMA
            .split_terminator(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            sqlx::query(stmt)
                .execute(self.store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("role_duty schema: {e}")))?;
        }
        Ok(())
    }

    /// Insert a fresh duty.
    pub async fn insert(&self, duty: &DutyRecord) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query(
            "INSERT INTO role_duty (id, tenant_id, worker_id, cron, entry_action, \
             description, enabled, paused_until, last_fired_at, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&duty.id)
        .bind(duty.tenant_id.0.as_bytes().as_slice())
        .bind(duty.worker_id.to_string())
        .bind(&duty.cron)
        .bind(&duty.entry_action)
        .bind(&duty.description)
        .bind(i64::from(duty.enabled))
        .bind(duty.paused_until.map(|d| d.timestamp_micros()))
        .bind(duty.last_fired_at.map(|d| d.timestamp_micros()))
        .bind(duty.created_at.timestamp_micros())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("insert duty: {e}")))?;
        Ok(())
    }

    /// Look up one duty by id.
    pub async fn get(&self, id: &str) -> ArgoResult<Option<DutyRecord>> {
        self.ensure_schema().await?;
        let row = sqlx::query(
            "SELECT id, tenant_id, worker_id, cron, entry_action, description, \
             enabled, paused_until, last_fired_at, created_at \
             FROM role_duty WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("find duty: {e}")))?;
        row.map(row_to_duty).transpose()
    }

    /// List duties scoped to a tenant, oldest first.
    pub async fn list_for_tenant(&self, tenant: TenantId) -> ArgoResult<Vec<DutyRecord>> {
        self.ensure_schema().await?;
        let rows = sqlx::query(
            "SELECT id, tenant_id, worker_id, cron, entry_action, description, \
             enabled, paused_until, last_fired_at, created_at \
             FROM role_duty WHERE tenant_id = ? ORDER BY created_at ASC, id ASC",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list duties: {e}")))?;
        rows.into_iter().map(row_to_duty).collect()
    }

    /// List all enabled, non-paused duties across every tenant —
    /// the per-tick scheduler input.
    pub async fn list_active(&self, now: DateTime<Utc>) -> ArgoResult<Vec<DutyRecord>> {
        self.ensure_schema().await?;
        let now_micros = now.timestamp_micros();
        let rows = sqlx::query(
            "SELECT id, tenant_id, worker_id, cron, entry_action, description, \
             enabled, paused_until, last_fired_at, created_at \
             FROM role_duty WHERE enabled = 1 \
             AND (paused_until IS NULL OR paused_until <= ?) \
             ORDER BY created_at ASC, id ASC",
        )
        .bind(now_micros)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list active duties: {e}")))?;
        rows.into_iter().map(row_to_duty).collect()
    }

    /// Set `paused_until` on a duty. Pass `None` to clear (resume).
    pub async fn set_paused(&self, id: &str, until: Option<DateTime<Utc>>) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query("UPDATE role_duty SET paused_until = ? WHERE id = ?")
            .bind(until.map(|d| d.timestamp_micros()))
            .bind(id)
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("set paused_until: {e}")))?;
        Ok(())
    }

    /// Update `last_fired_at`. Called after a successful fire.
    pub async fn mark_fired(&self, id: &str, fired_at: DateTime<Utc>) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query("UPDATE role_duty SET last_fired_at = ? WHERE id = ?")
            .bind(fired_at.timestamp_micros())
            .bind(id)
            .execute(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("mark fired: {e}")))?;
        Ok(())
    }

    /// Record the (duty -> session) attribution row.
    pub async fn link_fire(
        &self,
        duty_id: &str,
        session_id: &SessionId,
        fired_at: DateTime<Utc>,
        decision: FireDecision,
    ) -> ArgoResult<()> {
        self.ensure_schema().await?;
        sqlx::query(
            "INSERT OR IGNORE INTO duty_fired_session \
             (duty_id, session_id, fired_at, decision) VALUES (?, ?, ?, ?)",
        )
        .bind(duty_id)
        .bind(session_id.as_str())
        .bind(fired_at.timestamp_micros())
        .bind(decision.label())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("link fire: {e}")))?;
        Ok(())
    }

    /// All sessions ever spawned by one duty, newest first.
    pub async fn sessions_for_duty(&self, duty_id: &str) -> ArgoResult<Vec<(SessionId, DateTime<Utc>, String)>> {
        self.ensure_schema().await?;
        let rows = sqlx::query(
            "SELECT session_id, fired_at, decision FROM duty_fired_session \
             WHERE duty_id = ? ORDER BY fired_at DESC",
        )
        .bind(duty_id)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("sessions_for_duty: {e}")))?;
        rows.into_iter()
            .map(|row| {
                let sid: String = row.try_get("session_id").map_err(map_provider)?;
                let fired_micros: i64 = row.try_get("fired_at").map_err(map_provider)?;
                let decision: String = row.try_get("decision").map_err(map_provider)?;
                let fired_at = micros_to_utc(fired_micros, "fired_at")?;
                Ok((SessionId::from_string(sid), fired_at, decision))
            })
            .collect()
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

fn row_to_duty(row: sqlx::sqlite::SqliteRow) -> ArgoResult<DutyRecord> {
    let tenant_blob: Vec<u8> = row.try_get("tenant_id").map_err(map_provider)?;
    let tenant_id = TenantId(
        Uuid::from_slice(&tenant_blob).map_err(|e| ArgoError::Schema(format!("tenant: {e}")))?,
    );
    let worker_str: String = row.try_get("worker_id").map_err(map_provider)?;
    let worker_id =
        WorkerId::from_str(&worker_str).map_err(|e| ArgoError::Schema(format!("worker: {e}")))?;
    let enabled_int: i64 = row.try_get("enabled").map_err(map_provider)?;
    let paused: Option<i64> = row.try_get("paused_until").map_err(map_provider)?;
    let last_fired: Option<i64> = row.try_get("last_fired_at").map_err(map_provider)?;
    let created: i64 = row.try_get("created_at").map_err(map_provider)?;
    Ok(DutyRecord {
        id: row.try_get("id").map_err(map_provider)?,
        tenant_id,
        worker_id,
        cron: row.try_get("cron").map_err(map_provider)?,
        entry_action: row.try_get("entry_action").map_err(map_provider)?,
        description: row.try_get("description").map_err(map_provider)?,
        enabled: enabled_int != 0,
        paused_until: paused
            .map(|v| micros_to_utc(v, "paused_until"))
            .transpose()?,
        last_fired_at: last_fired
            .map(|v| micros_to_utc(v, "last_fired_at"))
            .transpose()?,
        created_at: micros_to_utc(created, "created_at")?,
    })
}

fn map_provider(e: sqlx::Error) -> ArgoError {
    ArgoError::Provider(format!("duty row: {e}"))
}

fn micros_to_utc(v: i64, field: &'static str) -> ArgoResult<DateTime<Utc>> {
    Utc.timestamp_micros(v)
        .single()
        .ok_or_else(|| ArgoError::Schema(format!("{field} out of range")))
}

fn trunc_to_micros(d: DateTime<Utc>) -> DateTime<Utc> {
    Utc.timestamp_micros(d.timestamp_micros())
        .single()
        .unwrap_or(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as CDur;

    async fn open_store() -> (Arc<DutyStore>, Arc<OrchestratorStore>) {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let duties = Arc::new(DutyStore::new(store.clone()));
        duties.ensure_schema().await.unwrap();
        let orch = Arc::new(OrchestratorStore::new(store));
        orch.ensure_schema().await.unwrap();
        (duties, orch)
    }

    fn record(tenant: TenantId, worker: WorkerId, cron: &str) -> DutyRecord {
        DutyRecord::new(tenant, worker, cron, "ping", "test duty")
    }

    #[tokio::test]
    async fn duty_round_trips_through_store() {
        let (duties, _) = open_store().await;
        let tenant = TenantId::new();
        let worker = WorkerId::new();
        let d = record(tenant, worker, "0 * * * * *");
        duties.insert(&d).await.unwrap();
        let loaded = duties.get(&d.id).await.unwrap().unwrap();
        assert_eq!(loaded, d);
    }

    #[tokio::test]
    async fn list_for_tenant_isolates_other_tenants() {
        let (duties, _) = open_store().await;
        let a = TenantId::new();
        let b = TenantId::new();
        duties.insert(&record(a, WorkerId::new(), "0 * * * * *")).await.unwrap();
        duties.insert(&record(b, WorkerId::new(), "0 * * * * *")).await.unwrap();
        assert_eq!(duties.list_for_tenant(a).await.unwrap().len(), 1);
        assert_eq!(duties.list_for_tenant(b).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn disabled_duty_never_fires() {
        let mut d = record(TenantId::new(), WorkerId::new(), "0 * * * * *");
        d.enabled = false;
        assert!(!d.should_fire(Utc::now()).unwrap());
    }

    #[tokio::test]
    async fn paused_duty_skipped_until_window_passes() {
        let mut d = record(TenantId::new(), WorkerId::new(), "0 * * * * *");
        let now = Utc::now();
        d.paused_until = Some(now + CDur::hours(1));
        assert!(!d.should_fire(now).unwrap());
        d.paused_until = Some(now - CDur::hours(1));
        // After the pause window, normal cron rules apply — the
        // `created_at = now` origin means the next tick is +1min.
        assert!(!d.should_fire(now).unwrap());
        assert!(d.should_fire(now + CDur::minutes(2)).unwrap());
    }

    #[tokio::test]
    async fn cron_window_idempotent_via_last_fired_at() {
        let mut d = record(TenantId::new(), WorkerId::new(), "0 * * * * *");
        let base = Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap();
        d.created_at = base;
        // First minute boundary after created_at = 12:01:00.
        assert!(d.should_fire(base + CDur::minutes(2)).unwrap());
        d.last_fired_at = Some(base + CDur::minutes(2));
        // Same tick should not re-fire.
        assert!(!d.should_fire(base + CDur::minutes(2)).unwrap());
        // Next minute boundary at 12:03 should fire.
        assert!(d.should_fire(base + CDur::minutes(3)).unwrap());
    }

    #[tokio::test]
    async fn invalid_cron_surfaces_schema_error() {
        let d = record(TenantId::new(), WorkerId::new(), "not a cron");
        assert!(matches!(
            d.should_fire(Utc::now()),
            Err(ArgoError::Schema(_))
        ));
    }

    #[tokio::test]
    async fn open_gate_auto_approves() {
        let g = OpenGate;
        let d = record(TenantId::new(), WorkerId::new(), "0 * * * * *");
        assert_eq!(g.evaluate(&d).await.unwrap(), FireDecision::AutoApprove);
    }

    #[tokio::test]
    async fn closed_gate_denies() {
        let g = ClosedGate;
        let d = record(TenantId::new(), WorkerId::new(), "0 * * * * *");
        assert_eq!(g.evaluate(&d).await.unwrap(), FireDecision::DeniedGate);
    }

    #[tokio::test]
    async fn pause_then_resume_round_trips() {
        let (duties, _) = open_store().await;
        let d = record(TenantId::new(), WorkerId::new(), "0 * * * * *");
        duties.insert(&d).await.unwrap();
        let until = Utc::now() + CDur::hours(2);
        duties.set_paused(&d.id, Some(until)).await.unwrap();
        let paused = duties.get(&d.id).await.unwrap().unwrap();
        assert!(paused.paused_until.is_some());
        duties.set_paused(&d.id, None).await.unwrap();
        let resumed = duties.get(&d.id).await.unwrap().unwrap();
        assert!(resumed.paused_until.is_none());
    }

    #[tokio::test]
    async fn link_fire_records_attribution_row() {
        let (duties, _) = open_store().await;
        let d = record(TenantId::new(), WorkerId::new(), "0 * * * * *");
        duties.insert(&d).await.unwrap();
        let session = SessionId::new();
        let now = Utc::now();
        duties
            .link_fire(&d.id, &session, now, FireDecision::AutoApprove)
            .await
            .unwrap();
        let rows = duties.sessions_for_duty(&d.id).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].2, "auto_approved");
    }

    #[tokio::test]
    async fn fire_decision_labels_are_stable() {
        assert_eq!(FireDecision::AutoApprove.label(), "auto_approved");
        assert_eq!(FireDecision::DeniedGate.label(), "denied_gate");
        assert_eq!(FireDecision::ManualFire.label(), "manual_fire");
    }

    #[tokio::test]
    async fn fire_now_creates_session_attributed_to_duty() {
        let (duties, orch) = open_store().await;
        let scheduler = DutyScheduler::new(duties.clone(), orch.clone(), Arc::new(OpenGate));
        let d = record(TenantId::new(), WorkerId::new(), "0 0 * * * *");
        duties.insert(&d).await.unwrap();
        let session_id = scheduler.fire_now(&d.id).await.unwrap();
        let attribution = duties.sessions_for_duty(&d.id).await.unwrap();
        assert_eq!(attribution.len(), 1);
        assert_eq!(attribution[0].0, session_id);
        assert_eq!(attribution[0].2, "manual_fire");
    }

    #[tokio::test]
    async fn tick_skips_paused_and_disabled_duties() {
        let (duties, orch) = open_store().await;
        let scheduler = DutyScheduler::new(duties.clone(), orch, Arc::new(OpenGate));
        let mut paused = record(TenantId::new(), WorkerId::new(), "* * * * * *");
        paused.paused_until = Some(Utc::now() + CDur::hours(1));
        let mut disabled = record(TenantId::new(), WorkerId::new(), "* * * * * *");
        disabled.enabled = false;
        duties.insert(&paused).await.unwrap();
        duties.insert(&disabled).await.unwrap();
        let report = scheduler.tick(Utc::now()).await.unwrap();
        assert_eq!(report.evaluated, 0); // both filtered by list_active
        assert!(report.fired.is_empty());
    }
}
