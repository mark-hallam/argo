//! Agent-runner trait, per-fire context, default no-op runner, and the
//! audit-payload builder used by [`super::DutyScheduler::tick`].
//!
//! Extracted from `duty_scheduler.rs` to keep that file under the
//! 500-LOC ceiling once PR #2088 (`fix-duty-scheduler-invokes-agent-
//! runner`) landed the `WorkerDutyFired` / `WorkerDutyFailed` audit
//! emission.

use argo_audit::AuditKind;
use argo_identity::WorkerId;
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::SessionId;

/// Behaviour the [`super::DutyScheduler`] needs from the agent runtime
/// when a duty's cron window elapses and the gate auto-approves.
///
/// Defined here so [`argo-orchestrator`] doesn't depend on
/// [`argo-agent-runner`] (the existing dep direction is
/// runner → orchestrator). Production wires a thin adapter that
/// calls `argo_agent_runner::run_turn`; tests inject mocks.
/// [`NoopDutyRunner`] is the default — it always succeeds and is
/// used by every constructor that doesn't explicitly opt into a
/// runner via [`super::DutyScheduler::with_agent_runner`].
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
/// [`super::DutyScheduler::with_agent_runner`]. Lets every existing
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

/// Build the JSON payload + audit kind for a duty fire. `err = None`
/// ⇒ `WorkerDutyFired`; `err = Some` ⇒ `WorkerDutyFailed` with the
/// stringified error inlined under `"error"`.
pub(super) fn build_audit_payload(
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
