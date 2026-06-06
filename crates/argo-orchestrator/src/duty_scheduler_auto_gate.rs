//! UF Phase-B · B2 — real auto-approval duty gate.
//!
//! Replaces the V1 fail-safe [`crate::duty_scheduler::ClosedGate`]
//! (deny-all) with a gate that REUSES the merged H-1 evaluator
//! ([`argo_identity::auto_approval::evaluate`]) over each worker's
//! resolved [`argo_identity::AutoApprovalPolicy`] plus a fresh
//! per-worker [`argo_identity::auto_approval::SpendSnapshot`]
//! (UF-A6's `worker_budget` ledger). It authors NO new policy
//! engine — it is purely the wiring that lets a scheduled duty fire
//! be decided by the existing, already-tested evaluator.
//!
//! ## Default-OFF + reversible (DEC-036 / DEC-040)
//! This gate only becomes the scheduler-level gate when the gateway
//! boot path reads `ARGO_DUTY_AUTOFIRE` as truthy. With the flag
//! unset (the default), boot keeps wiring `ClosedGate`, so today's
//! deny-all behaviour is preserved byte-for-byte. The live flip is
//! operator-only, after the adversarial budget/risk review.
//!
//! ## Decision mapping
//! The gate trait can only return a [`FireDecision`], so the
//! "Ask → approvals inbox" half of the brief is performed as a side
//! effect here (the trait + tick loop are out of this brief's
//! may-touch list and cannot carry a richer payload):
//!
//! * [`AutoApprovalDecision::AutoApprove`] → [`FireDecision::AutoApprove`]
//!   (the scheduler spawns the real `run_turn`).
//! * [`AutoApprovalDecision::Ask`] / [`AutoApprovalDecision::Block`]
//!   → the fire is HELD ([`FireDecision::DeniedGate`]) AND an
//!   `approval.asked` audit row is written (when an [`AuditSink`] is
//!   injected) so the held fire surfaces in the audit-backed
//!   approvals inbox. Any non-auto-approve verdict surfaces an
//!   approval item — a budget-blocked scheduled duty is exactly the
//!   thing the operator wants to see and may choose to override.
//! * An unknown / cross-tenant `worker_id` → [`FireDecision::DeniedGate`]
//!   (fail-safe; nothing to approve).

use crate::duty_scheduler::{DutyGate, DutyRecord, FireDecision};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::auto_approval::{evaluate, AutoApprovalDecision, PendingAction, SpendSnapshot};
use argo_identity::worker_budget::spend_today_for_worker;
use argo_identity::{AutoApprovalPolicy, IdentityStore};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

/// Nominal per-fire spend estimate (USD micros) attributed to a duty
/// fire when the duty carries no explicit cost. $0.05 — large enough
/// that a `$0` budget meaningfully holds a fire, small enough that a
/// generous (uncapped) policy clears it. A duty fire spawns a real
/// `run_turn`, so the true cost is non-zero; this is the floor the
/// budget caps are measured against until per-action cost estimates
/// are plumbed onto `DutyRecord`.
pub const DEFAULT_DUTY_ESTIMATE_USD_MICROS: u64 = 50_000;

/// Default risk class assigned to a scheduled-duty fire. The
/// `DutyRecord` carries no risk metadata, so the gate picks a
/// conservative middle: `medium` auto-approves under a generous
/// policy but lets operators force a human ask via a high-risk
/// policy classification in a later iteration. Never `high` — that
/// would make auto-fire impossible regardless of policy.
const DEFAULT_DUTY_RISK_LEVEL: &str = "medium";

/// [`DutyGate`] that decides a fire via the worker's resolved
/// [`AutoApprovalPolicy`] + a fresh per-worker spend snapshot.
///
/// Wired by gateway boot behind `ARGO_DUTY_AUTOFIRE`. Holds its own
/// [`SessionStore`] handle so it can resolve the worker (and its
/// policy) and read the `worker_budget` ledger at evaluate time;
/// the optional [`AuditSink`] backs the approvals-inbox side effect.
pub struct AutoApprovalDutyGate {
    store: Arc<SessionStore>,
    workspace_default: AutoApprovalPolicy,
    audit_sink: Option<Arc<AuditSink>>,
    per_fire_estimate_usd_micros: u64,
}

impl AutoApprovalDutyGate {
    /// Build a gate over an open, migrated [`SessionStore`]. No
    /// audit sink (held fires are not surfaced) and the workspace
    /// default policy is the disabled default (`Ask` everything that
    /// isn't worker-policy-approved) until [`Self::with_audit_sink`]
    /// / [`Self::with_workspace_default`] opt in.
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self {
            store,
            workspace_default: AutoApprovalPolicy::default(),
            audit_sink: None,
            per_fire_estimate_usd_micros: DEFAULT_DUTY_ESTIMATE_USD_MICROS,
        }
    }

    /// Inject the workspace-default policy a worker falls back to
    /// when its own `auto_approval` is disabled (see
    /// [`argo_identity::Worker::resolved_policy`]).
    #[must_use]
    pub fn with_workspace_default(mut self, policy: AutoApprovalPolicy) -> Self {
        self.workspace_default = policy;
        self
    }

    /// Inject the [`AuditSink`] that backs the approvals inbox. When
    /// present, a held fire (`Ask` / `Block`) writes an
    /// `approval.asked` row scoped to the duty's tenant.
    #[must_use]
    pub fn with_audit_sink(mut self, sink: Arc<AuditSink>) -> Self {
        self.audit_sink = Some(sink);
        self
    }

    /// Override the nominal per-fire spend estimate (default
    /// [`DEFAULT_DUTY_ESTIMATE_USD_MICROS`]).
    #[must_use]
    pub fn with_per_fire_estimate(mut self, usd_micros: u64) -> Self {
        self.per_fire_estimate_usd_micros = usd_micros;
        self
    }

    /// Surface a held fire to the audit-backed approvals inbox. A
    /// no-op (logs only) when no sink is configured. Failure to
    /// write the audit row is logged but never propagated — a held
    /// fire stays held regardless.
    async fn surface_approval(&self, duty: &DutyRecord, decision: &AutoApprovalDecision) {
        let Some(sink) = self.audit_sink.as_ref() else {
            return;
        };
        let verdict = match decision {
            AutoApprovalDecision::Ask { .. } => "ask",
            AutoApprovalDecision::Block { .. } => "block",
            AutoApprovalDecision::AutoApprove { .. } => return,
        };
        let payload = serde_json::json!({
            "source": "duty_scheduler_auto_gate",
            "duty_id": duty.id,
            "worker_id": duty.worker_id.to_string(),
            "entry_action": duty.entry_action,
            "decision": verdict,
            "reason": decision.reason(),
            "estimated_usd_micros": self.per_fire_estimate_usd_micros,
        });
        let tenant = duty.tenant_id.0.to_string();
        if let Err(e) = sink
            .write_with_tenant(None, AuditKind::ApprovalAsked, payload, None, &tenant)
            .await
        {
            tracing::warn!(
                target: "argo_orchestrator::duty_scheduler_auto_gate",
                tenant_id = %duty.tenant_id.0,
                duty_id = %duty.id,
                error = %e,
                "approval.asked audit write failed; duty fire stays held"
            );
        }
    }
}

#[async_trait]
impl DutyGate for AutoApprovalDutyGate {
    async fn evaluate(&self, duty: &DutyRecord) -> ArgoResult<FireDecision> {
        // Resolve the worker WITHIN the duty's tenant — a cross-tenant
        // worker_id yields `None` and fails closed, so the gate is
        // tenant-isolated by construction.
        let identities = IdentityStore::new(self.store.as_ref());
        let Some(worker) = identities.get_worker(duty.tenant_id, duty.worker_id).await? else {
            tracing::warn!(
                target: "argo_orchestrator::duty_scheduler_auto_gate",
                tenant_id = %duty.tenant_id.0,
                worker_id = %duty.worker_id,
                duty_id = %duty.id,
                "duty worker not found in tenant; fire denied (fail-safe)"
            );
            return Ok(FireDecision::DeniedGate);
        };

        let policy = worker.resolved_policy(&self.workspace_default);
        let now = Utc::now();
        let today = spend_today_for_worker(self.store.pool(), &duty.worker_id.to_string(), now)
            .await
            .unwrap_or(0);
        // `evaluate` reads only `today_usd_micros`; the persisted
        // ledger exposes today's spend per worker (UF-A6). The
        // monthly field is a lower-bounded best effort (today's spend
        // is a subset of the month) and is unused by the daily cap.
        let spend = SpendSnapshot {
            today_usd_micros: today,
            this_month_usd_micros: today,
        };

        let pending = PendingAction {
            risk_level: DEFAULT_DUTY_RISK_LEVEL,
            requires_approval: true,
            estimated_usd_micros: Some(self.per_fire_estimate_usd_micros),
            action_type: duty.entry_action.as_str(),
        };

        let decision = evaluate(&policy, &pending, spend, now);
        match decision {
            AutoApprovalDecision::AutoApprove { reason } => {
                tracing::debug!(
                    target: "argo_orchestrator::duty_scheduler_auto_gate",
                    tenant_id = %duty.tenant_id.0,
                    worker_id = %duty.worker_id,
                    duty_id = %duty.id,
                    reason = %reason,
                    "duty fire auto-approved"
                );
                Ok(FireDecision::AutoApprove)
            }
            held => {
                tracing::info!(
                    target: "argo_orchestrator::duty_scheduler_auto_gate",
                    tenant_id = %duty.tenant_id.0,
                    worker_id = %duty.worker_id,
                    duty_id = %duty.id,
                    reason = %held.reason(),
                    "duty fire held; surfacing approval item"
                );
                self.surface_approval(duty, &held).await;
                Ok(FireDecision::DeniedGate)
            }
        }
    }
}

#[cfg(test)]
#[path = "duty_scheduler_auto_gate_tests.rs"]
mod tests;
