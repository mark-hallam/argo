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

/// Resolves a duty's real `(risk_level, requires_approval)` from its
/// worker's vertical manifest. `argo-orchestrator` cannot read
/// manifests, so the gateway (`argo-cli`, which can) injects this at
/// construction. Called with `(vertical_id, entry_action)`; returns
/// `None` when the action can't be resolved.
///
/// **Fail-closed (DEC-041):** when no resolver is injected, or it
/// returns `None`, the gate treats the fire as `("high", true)` —
/// high-risk + approval-required — so it is HELD, never auto-fired.
/// A duty only enters the act-by-default safe tier when the resolver
/// affirmatively reports the action is low-risk + `requires_approval=false`.
pub type DutyRiskResolver = Arc<dyn Fn(&str, &str) -> Option<(String, bool)> + Send + Sync>;

/// Reads a worker's spend-to-date (USD micros) for the daily-budget check.
/// Injectable so a test can force the read to ERROR; the production reader is
/// the async `worker_budget` ledger query ([`spend_today_for_worker`]). Called
/// with the worker id string. On `Err` the gate fails CLOSED (holds the fire) —
/// see [`AutoApprovalDutyGate::evaluate`].
pub type DutySpendReader = Arc<dyn Fn(&str) -> ArgoResult<u64> + Send + Sync>;

/// Fail-closed risk class + approval flag used when the duty's real
/// risk cannot be resolved. `high` + `requires_approval` → the policy
/// always asks (held), so an unresolved duty NEVER auto-fires.
const FAILCLOSED_RISK_LEVEL: &str = "high";
const FAILCLOSED_REQUIRES_APPROVAL: bool = true;

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
    risk_resolver: Option<DutyRiskResolver>,
    spend_reader: Option<DutySpendReader>,
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
            risk_resolver: None,
            spend_reader: None,
        }
    }

    /// Inject an alternate spend reader (defaults to the async `worker_budget`
    /// ledger query). Used by tests to force a read error and prove the gate
    /// fails CLOSED; production never sets it.
    #[must_use]
    pub fn with_spend_reader(mut self, reader: DutySpendReader) -> Self {
        self.spend_reader = Some(reader);
        self
    }

    /// Inject the manifest-backed risk resolver (DEC-041 risk-tiering).
    /// Without it the gate fail-closes every fire to high-risk +
    /// approval-required (held). The gateway provides a resolver that
    /// reads the worker's vertical manifest; orchestrator can't.
    #[must_use]
    pub fn with_risk_resolver(mut self, resolver: DutyRiskResolver) -> Self {
        self.risk_resolver = Some(resolver);
        self
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

        // PER-TENANT AUTONOMY OPT-IN (default OFF, operator standing rule).
        // Autonomy = the scheduled duty FIRES without a human starting it; it
        // is NOT auto-approval of the resulting action (the fire still flows
        // through the H-1 evaluator below, and a `low`/safe fire that the
        // evaluator does auto-approve still spawns a real `run_turn` whose own
        // tool actions hit the runtime approval gate). A tenant that has not
        // explicitly opted in keeps every scheduled fire HELD — even when the
        // process-global `ARGO_DUTY_AUTOFIRE` flag selected this gate over the
        // deny-all `ClosedGate` at boot. The per-tenant setting is therefore
        // authoritative: one customer can opt ONE workspace into scheduled
        // autonomous work without flipping it for everyone. A held fire still
        // surfaces an approval item so the operator sees it in the inbox.
        if !argo_store::workspace_settings::tenant_duty_autofire_enabled(
            self.store.pool(),
            &duty.tenant_id.0.to_string(),
        )
        .await
        .unwrap_or(false)
        {
            tracing::info!(
                target: "argo_orchestrator::duty_scheduler_auto_gate",
                tenant_id = %duty.tenant_id.0,
                worker_id = %duty.worker_id,
                duty_id = %duty.id,
                "tenant has not opted into scheduled autonomy (default OFF); fire held"
            );
            self.surface_approval(
                duty,
                &AutoApprovalDecision::Ask {
                    channel: None,
                    reason: "tenant autonomy disabled (default OFF)".to_string(),
                },
            )
            .await;
            return Ok(FireDecision::DeniedGate);
        }

        // DEC-060 PER-TEAMMATE AUTONOMY LEVEL (authoritative floor). A worker
        // only fires autonomously when its OWN level is >= Accept Safe — i.e.
        // its `auto_approval` policy is explicitly enabled. `Always Ask` (and
        // any unconfigured worker, whose default policy is disabled) holds
        // every fire. This is checked on `worker.auto_approval` DIRECTLY, NOT
        // via `resolved_policy`: when the worker policy is disabled,
        // `resolved_policy` falls back to the (production-enabled) safe-tier
        // workspace default, which would otherwise let an `Always Ask`
        // teammate auto-fire at the workspace budget. The explicit check makes
        // the chosen per-teammate level the floor the fallback cannot lift.
        if !worker.auto_approval.enabled {
            tracing::info!(
                target: "argo_orchestrator::duty_scheduler_auto_gate",
                tenant_id = %duty.tenant_id.0,
                worker_id = %duty.worker_id,
                duty_id = %duty.id,
                "teammate autonomy level is Always Ask (policy disabled); fire held"
            );
            self.surface_approval(
                duty,
                &AutoApprovalDecision::Ask {
                    channel: None,
                    reason: "teammate autonomy level Always Ask (held)".to_string(),
                },
            )
            .await;
            return Ok(FireDecision::DeniedGate);
        }

        let mut policy = worker.resolved_policy(&self.workspace_default);
        // Defense in depth (DEC-060 adversarial review, HIGH): an enabled
        // policy with NO explicit ceiling auto-approves unbounded — the
        // worker's policy is returned verbatim by `resolved_policy`, so the
        // workspace-default caps are only the fallback the enabled worker never
        // hits. Clamp any MISSING cap down to the workspace default so an
        // enabled teammate always has a ceiling. The DEC-060 ladder always sets
        // caps, so this only bites a mis-configured / legacy enabled-but-
        // uncapped policy. Explicit caps (incl. the ladder's higher rungs) are
        // untouched.
        if policy.enabled {
            policy.max_usd_micros_per_action = policy
                .max_usd_micros_per_action
                .or(self.workspace_default.max_usd_micros_per_action);
            policy.daily_budget_usd_micros = policy
                .daily_budget_usd_micros
                .or(self.workspace_default.daily_budget_usd_micros);
        }
        let now = Utc::now();
        // FAIL-CLOSED on a spend-ledger read error (DEC-060 adversarial review).
        // Every other branch of this gate fails closed, but this read previously
        // coerced an error to `$0 spent` (fail-OPEN) — which makes the daily-cap
        // check MORE likely to auto-approve precisely when the ledger is
        // unreadable. An unknown spend state must never auto-fire: hold it +
        // surface an approval item. The `spend_reader` seam lets tests force the
        // error; production uses the async `worker_budget` query.
        let worker_id_str = duty.worker_id.to_string();
        let today_result = match self.spend_reader.as_ref() {
            Some(read) => read(&worker_id_str),
            None => spend_today_for_worker(self.store.pool(), &worker_id_str, now).await,
        };
        let today = match today_result {
            Ok(spent) => spent,
            Err(e) => {
                tracing::warn!(
                    target: "argo_orchestrator::duty_scheduler_auto_gate",
                    tenant_id = %duty.tenant_id.0,
                    worker_id = %duty.worker_id,
                    duty_id = %duty.id,
                    error = %e,
                    "spend-ledger read failed; holding fire (fail-closed) rather than assuming $0 spent"
                );
                self.surface_approval(
                    duty,
                    &AutoApprovalDecision::Ask {
                        channel: None,
                        reason: "spend ledger unavailable (fail-closed)".to_string(),
                    },
                )
                .await;
                return Ok(FireDecision::DeniedGate);
            }
        };
        // `evaluate` reads only `today_usd_micros`; the persisted
        // ledger exposes today's spend per worker (UF-A6). The
        // monthly field is a lower-bounded best effort (today's spend
        // is a subset of the month) and is unused by the daily cap.
        let spend = SpendSnapshot {
            today_usd_micros: today,
            this_month_usd_micros: today,
        };

        // DEC-041 risk-tiering: resolve the duty's REAL risk + approval
        // flag from the worker's vertical manifest (via the injected
        // resolver, since orchestrator can't read manifests). Fail-closed
        // to high-risk + approval-required when unresolved.
        let (risk_level, manifest_requires_approval) = self
            .risk_resolver
            .as_ref()
            .and_then(|resolve| resolve(&worker.vertical_id, &duty.entry_action))
            .unwrap_or_else(|| {
                (
                    FAILCLOSED_RISK_LEVEL.to_string(),
                    FAILCLOSED_REQUIRES_APPROVAL,
                )
            });
        // The act-by-default SAFE TIER is narrow on purpose (DEC-041):
        // ONLY `low` risk AND the manifest says no approval needed.
        // `medium` ("public surface but reversible") and `high` STAY held
        // — DEC-028's no-autonomous-public-posting line.
        //
        // We DON'T weaken the shared evaluator. Instead: the safe tier is
        // sent at its real `low` risk so the evaluator's spend/rate caps
        // still gate it (a $0 budget holds even a safe fire); everything
        // else is coerced to `high` so the evaluator's existing
        // `high_risk_always_asks` rule holds it regardless of how generous
        // the worker's policy is. `requires_approval` stays `true` so the
        // `!requires_approval` short-circuit never bypasses the caps.
        let autofire_safe = risk_level == "low" && !manifest_requires_approval;
        let eval_risk = if autofire_safe { "low" } else { "high" };
        let pending = PendingAction {
            risk_level: eval_risk,
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
