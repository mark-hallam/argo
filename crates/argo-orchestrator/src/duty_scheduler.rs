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
use argo_audit::{AuditKind, AuditSink};
use argo_identity::WorkerId;
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

pub use crate::duty_scheduler_gate::{
    gate_from_config, ClosedGate, DutyGate, FireDecision, GateConfig, OpenGate, QuietHoursGate,
    ScheduleGate,
};
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
    /// Per-duty fire-gate selection. Defaults to
    /// [`GateConfig::Open`], which delegates to the scheduler-level
    /// gate so legacy rows preserve today's behavior.
    #[serde(default)]
    pub gate: GateConfig,
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
            gate: GateConfig::default(),
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
pub struct DutyTickReport {
    /// Total duties evaluated.
    pub evaluated: usize,
    /// Duty ids whose cron matched and the gate auto-approved.
    pub fired: Vec<String>,
    /// Duty ids whose cron matched but the gate denied.
    pub denied_by_gate: Vec<String>,
    /// Duty ids whose cron matched + auto-approved + spawned a
    /// session BUT whose injected [`DutyAgentRunner::run_turn`]
    /// returned `Err`. Pairs with the [`AuditKind::WorkerDutyFailed`]
    /// audit row written for the same fire — silent runner failures
    /// were the worst failure mode pre-fix; this counter makes them
    /// visible in the per-tick report.
    pub run_failed: Vec<String>,
}

/// Behaviour the [`DutyScheduler`] needs from the agent runtime
/// when a duty's cron window elapses and the gate auto-approves.
///
/// Defined here so [`argo-orchestrator`] doesn't depend on
/// [`argo-agent-runner`] (the existing dep direction is
/// runner → orchestrator). Production wires a thin adapter that
/// calls `argo_agent_runner::run_turn`; tests inject mocks.
/// [`NoopDutyRunner`] is the default — it always succeeds and is
/// used by every constructor that doesn't explicitly opt into a
/// runner via [`DutyScheduler::with_agent_runner`].
#[async_trait]
pub trait DutyAgentRunner: Send + Sync {
    /// Drive one agent turn for the just-spawned duty session.
    ///
    /// Returning `Ok(())` ⇒ the scheduler emits a
    /// [`AuditKind::WorkerDutyFired`] row. Returning `Err` ⇒ the
    /// scheduler emits a [`AuditKind::WorkerDutyFailed`] row, logs
    /// at `tracing::error` level, and keeps ticking the next duty
    /// — a single runner failure must never kill the loop.
    async fn run_turn(&self, ctx: DutyRunContext) -> ArgoResult<()>;
}

/// Context handed to [`DutyAgentRunner::run_turn`] per duty fire.
#[derive(Debug, Clone)]
pub struct DutyRunContext {
    /// Tenant that owns the duty + worker + session.
    pub tenant_id: TenantId,
    /// Worker the duty belongs to — the agent loop must run AS
    /// this worker (system prompt, capabilities, memory).
    pub worker_id: WorkerId,
    /// Duty row id — included so the runner can look up the
    /// duty's entry_action / description / cron history if needed.
    pub duty_id: String,
    /// Session row the scheduler just spawned for this fire. The
    /// runner appends messages to this session, not a fresh one.
    pub session_id: SessionId,
    /// Cron expression that fired (informational; audit payload).
    pub cron_spec: String,
    /// Handler name from the worker action vocabulary — the
    /// canonical "what to do this tick" string.
    pub entry_action: String,
    /// Tick instant (UTC) — every audit row written for this
    /// fire stamps the same value so a single fire is one row in
    /// `worker_duty.fired` (or `worker_duty.failed`) by timestamp.
    pub fired_at: DateTime<Utc>,
}

/// Default [`DutyAgentRunner`] — does nothing, always succeeds.
///
/// Wired by every constructor that doesn't call
/// [`DutyScheduler::with_agent_runner`]. Lets every existing
/// callsite (CLI handlers, gateway boot's `ClosedGate` path,
/// integration tests) keep its current behaviour: spawn the
/// session row + write the attribution row, no agent invocation.
/// The production gateway swaps in a real adapter that calls
/// `argo_agent_runner::run_turn` via a follow-up wire-up brief.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopDutyRunner;

#[async_trait]
impl DutyAgentRunner for NoopDutyRunner {
    async fn run_turn(&self, _ctx: DutyRunContext) -> ArgoResult<()> {
        Ok(())
    }
}

/// Combines a [`DutyStore`] with an [`OrchestratorStore`] and a
/// [`DutyGate`] to produce the autonomous-loop tick.
pub struct DutyScheduler {
    duties: Arc<DutyStore>,
    orchestrator: Arc<OrchestratorStore>,
    gate: Arc<dyn DutyGate>,
    runner: Arc<dyn DutyAgentRunner>,
    audit_sink: Option<Arc<AuditSink>>,
}

impl DutyScheduler {
    /// Build a scheduler around the three injected dependencies.
    ///
    /// The agent runner defaults to [`NoopDutyRunner`] and the
    /// audit sink defaults to `None` — opt into the real wire via
    /// [`Self::with_agent_runner`] / [`Self::with_audit_sink`] so
    /// every existing callsite keeps compiling unchanged.
    #[must_use]
    pub fn new(
        duties: Arc<DutyStore>,
        orchestrator: Arc<OrchestratorStore>,
        gate: Arc<dyn DutyGate>,
    ) -> Self {
        Self {
            duties,
            orchestrator,
            gate,
            runner: Arc::new(NoopDutyRunner),
            audit_sink: None,
        }
    }

    /// Inject a [`DutyAgentRunner`] — gateway boot calls this with
    /// an adapter that invokes `argo_agent_runner::run_turn`. The
    /// returned scheduler is otherwise unchanged.
    #[must_use]
    pub fn with_agent_runner(mut self, runner: Arc<dyn DutyAgentRunner>) -> Self {
        self.runner = runner;
        self
    }

    /// Inject an [`AuditSink`] so each fire emits a
    /// [`AuditKind::WorkerDutyFired`] (or
    /// [`AuditKind::WorkerDutyFailed`]) row. Without a sink the
    /// scheduler keeps today's behaviour: session row + attribution
    /// only. The sink is wired by the gateway-boot brief alongside
    /// the runner injection.
    #[must_use]
    pub fn with_audit_sink(mut self, sink: Arc<AuditSink>) -> Self {
        self.audit_sink = Some(sink);
        self
    }

    /// Walk every active duty and fire those whose cron window has
    /// elapsed since `last_fired_at` (or `created_at` when never
    /// fired). Returns the per-tick report.
    ///
    /// Each fire is filtered through the duty's per-row gate
    /// resolved from [`DutyRecord::gate`]. [`GateConfig::Open`]
    /// (the default for legacy rows) delegates to the
    /// scheduler-level gate so today's behavior is preserved. The
    /// other variants override per-duty.
    pub async fn tick(&self, now: DateTime<Utc>) -> ArgoResult<DutyTickReport> {
        let mut report = DutyTickReport::default();
        let active = self.duties.list_active(now).await?;
        report.evaluated = active.len();
        for duty in active {
            if !duty.should_fire(now)? {
                continue;
            }
            let per_duty: Option<Box<dyn DutyGate>> = match &duty.gate {
                GateConfig::Open => None,
                other => Some(gate_from_config(other)?),
            };
            let gate: &dyn DutyGate = match per_duty.as_deref() {
                Some(g) => g,
                None => self.gate.as_ref(),
            };
            let tenant = duty.tenant_id;
            match gate.evaluate(&duty).await? {
                FireDecision::AutoApprove => {
                    // Pre-allocate the session in memory so the runner
                    // gets a stable session_id BEFORE the atomic write.
                    // Persistence happens later, inside the same
                    // transaction as the audit row.
                    let session = self.build_session(&duty, now);
                    let session_id = session.id.clone();
                    let ctx = DutyRunContext {
                        tenant_id: duty.tenant_id,
                        worker_id: duty.worker_id,
                        duty_id: duty.id.clone(),
                        session_id: session_id.clone(),
                        cron_spec: duty.cron.clone(),
                        entry_action: duty.entry_action.clone(),
                        fired_at: now,
                    };
                    let run_outcome = self.runner.run_turn(ctx.clone()).await;
                    if let Err(ref err) = run_outcome {
                        tracing::error!(
                            target: "argo_orchestrator::duty_scheduler",
                            tenant_id = %duty.tenant_id.0,
                            worker_id = %duty.worker_id,
                            duty_id = %duty.id,
                            session_id = %session_id.as_str(),
                            error = %err,
                            "duty agent runner returned Err — emitting WorkerDutyFailed; scheduler continues"
                        );
                    }
                    // Atomic: session + audit either both commit or
                    // neither. If the audit write fails, the session
                    // row is rolled back and the duty stays unfired —
                    // the next tick retries.
                    let commit_outcome = self
                        .commit_session_and_audit(
                            &session,
                            &ctx,
                            run_outcome.as_ref().err(),
                        )
                        .await;
                    if let Err(err) = commit_outcome {
                        tracing::error!(
                            target: "argo_orchestrator::duty_scheduler",
                            tenant_id = %duty.tenant_id.0,
                            worker_id = %duty.worker_id,
                            duty_id = %duty.id,
                            error = %err,
                            "atomic session+audit commit failed; duty not marked fired — scheduler retries next tick"
                        );
                        continue;
                    }
                    self.duties
                        .link_fire(
                            tenant,
                            &duty.id,
                            &session_id,
                            now,
                            FireDecision::AutoApprove,
                        )
                        .await?;
                    self.duties.mark_fired(tenant, &duty.id, now).await?;
                    if run_outcome.is_err() {
                        report.run_failed.push(duty.id.clone());
                    }
                    report.fired.push(duty.id);
                }
                other => {
                    self.duties
                        .link_fire(
                            tenant,
                            &duty.id,
                            &SessionId::from_string("00000000-0000-0000-0000-000000000000".into()),
                            now,
                            other,
                        )
                        .await
                        .ok();
                    self.duties.mark_fired(tenant, &duty.id, now).await?;
                    report.denied_by_gate.push(duty.id);
                }
            }
        }
        Ok(report)
    }

    /// Manually fire one duty bypassing cron + gate (the
    /// `argo duty fire-now` surface). `tenant` scopes every
    /// underlying `DutyStore` lookup; cross-tenant calls surface as
    /// a byte-identical "not found" `ArgoError::Schema`.
    pub async fn fire_now(&self, tenant: TenantId, duty_id: &str) -> ArgoResult<SessionId> {
        let now = Utc::now();
        let duty = self
            .duties
            .get(tenant, duty_id)
            .await?
            .ok_or_else(|| ArgoError::Schema(format!("duty '{duty_id}' not found")))?;
        let session = self.build_session(&duty, now);
        let session_id = session.id.clone();
        self.orchestrator.insert_session(&session).await?;
        self.duties
            .link_fire(tenant, &duty.id, &session_id, now, FireDecision::ManualFire)
            .await?;
        self.duties.mark_fired(tenant, &duty.id, now).await?;
        Ok(session_id)
    }

    /// Atomically persist session + audit in one sqlx tx; with no
    /// sink configured, falls back to a standalone session insert.
    async fn commit_session_and_audit(
        &self,
        session: &Session,
        ctx: &DutyRunContext,
        err: Option<&ArgoError>,
    ) -> ArgoResult<()> {
        let Some(sink) = self.audit_sink.as_ref() else {
            // No audit sink ⇒ preserve the pre-fix behavior: session
            // is inserted standalone, no audit row written.
            self.orchestrator.insert_session(session).await?;
            return Ok(());
        };
        let (kind, payload) = build_audit_payload(ctx, err);
        let session_uuid = Uuid::parse_str(ctx.session_id.as_str()).map_err(|parse_err| {
            ArgoError::Schema(format!(
                "duty session id not parseable as UUID '{}': {parse_err}",
                ctx.session_id.as_str()
            ))
        })?;
        let audit_session = argo_types::SessionId(session_uuid);
        let mut tx = self
            .orchestrator
            .pool()
            .begin()
            .await
            .map_err(|e| ArgoError::Provider(format!("duty scheduler tx begin: {e}")))?;
        self.orchestrator
            .insert_session_in_tx(&mut tx, session)
            .await?;
        // Stamp the duty's tenant on the audit row — the injected
        // sink is shared across tenants for the scheduler's lifetime.
        let tenant = ctx.tenant_id.0.to_string();
        let post = sink
            .write_in_tx_for_tenant(&mut tx, Some(audit_session), kind, payload, None, &tenant)
            .await?;
        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("duty scheduler tx commit: {e}")))?;
        if let Err(e) = sink.after_commit_fanout(post).await {
            tracing::warn!(
                target: "argo_orchestrator::duty_scheduler",
                tenant_id = %ctx.tenant_id.0,
                duty_id = %ctx.duty_id,
                error = %e,
                "duty audit post-commit fanout failed (session + audit already committed)"
            );
        }
        Ok(())
    }

    fn build_session(&self, duty: &DutyRecord, now: DateTime<Utc>) -> Session {
        let mut session = Session::new(
            duty.tenant_id,
            format!(
                "duty:{}@{}",
                duty.entry_action,
                now.format("%Y-%m-%dT%H:%MZ")
            ),
        );
        session.add_participant(duty.worker_id);
        session
    }
}

fn build_audit_payload(
    ctx: &DutyRunContext,
    err: Option<&ArgoError>,
) -> (AuditKind, serde_json::Value) {
    let mut payload = serde_json::json!({
        "tenant_id": ctx.tenant_id.0.to_string(),
        "worker_id": ctx.worker_id.to_string(),
        "duty_id": ctx.duty_id,
        "session_id": ctx.session_id.as_str(),
        "cron_spec": ctx.cron_spec,
        "fired_at": ctx.fired_at.to_rfc3339(),
        "entry_action": ctx.entry_action,
    });
    match err {
        None => (AuditKind::WorkerDutyFired, payload),
        Some(e) => {
            if let serde_json::Value::Object(ref mut map) = payload {
                map.insert(
                    "error".to_string(),
                    serde_json::Value::String(e.to_string()),
                );
            }
            (AuditKind::WorkerDutyFailed, payload)
        }
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
#[path = "duty_scheduler_tests.rs"]
mod tests;
