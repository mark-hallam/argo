//! The single choke point that makes hard `[budget.ceiling]` spend
//! enforcement **un-forgettable** (Stream
//! `launch-wire-spendgate-llm-cost-ceiling`, the WS5 open-wallet close).
//!
//! `argo_provider::SpendGate` is the enforcement-time wrapper that refuses
//! a provider call when the projected charge would push a tenant past its
//! session/day/month USD ceiling. The first wiring attempt (#2855) bound it
//! only at the turn-loop seam, leaving SIX other live `build_transport`
//! paths un-wrapped — incl. the internet-facing pitch, Genesis, ghostwriter,
//! orchestration-team and the autonomous distillation loop. A stranger (or a
//! runaway loop) could still burn unbounded LLM dollars there.
//!
//! # The fix: gate by construction
//!
//! [`build_transport`](crate::transport::build_transport) and
//! [`build_transport_for_tenant`](crate::transport::build_transport_for_tenant)
//! now REQUIRE a [`TransportGate`]. Because the gate is a mandatory argument,
//! no call site can obtain a bare transport — the compiler refuses it. Every
//! `build_transport(...).complete(...)` site is gated by construction; there
//! is no path to forget.
//!
//! # Activation
//!
//! The gate is **ON by default in a production deployment** and **transparent
//! everywhere else** (zero per-call overhead — the wrap is skipped entirely
//! when the resolved ceilings are all uncapped). A deployment counts as
//! production when `ARGO_GATEWAY_PROFILE=production` (the signal the gateway
//! perimeter sets) or when `[core].profile` is `prod`/`production`. Dev / demo
//! / tests stay unchanged unless the operator opts into a production profile,
//! so no existing spend-attribution test starts blocking.
//!
//! # Fail direction
//!
//! Cost-safety is priority-zero ("we cannot lose money on this"). When we know
//! we're in production but the `[budget]` config cannot be read, we fall back
//! to the **shipped default ceilings** ($1 / $10 / $100 session/day/month)
//! rather than running uncapped — a degraded-but-bounded path, never an open
//! wallet.
use std::path::Path;
use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_provider::spend_gate::{BudgetEnforcer, SpendGate};
use argo_provider::Transport;
use argo_store::spend_ceiling::CeilingConfig;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, CostCeilings, SessionId, TenantId};
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

/// Env var the gateway sets to mark a live, internet-facing deployment.
const GATEWAY_PROFILE_ENV: &str = "ARGO_GATEWAY_PROFILE";

/// Stable tenant id every pre-auth / anonymous gateway LLM call maps to.
///
/// Fixed (NOT random) so the day/month spend windows ACCUMULATE across all
/// anonymous callers. That shared, bounded bucket is what closes ws5-4b: a
/// stranger — or many — cannot burn unbounded LLM dollars on a public path,
/// because each allowed anonymous call reserves its projected cost against
/// this one tenant's day/month ceiling. Derived from a constant so it is
/// identical on every process and machine.
const ANONYMOUS_TENANT_U128: u128 = 0xa17b0000_00004000_80000000_616e6f6e;

/// The fixed capped anonymous tenant. See [`ANONYMOUS_TENANT_U128`].
#[must_use]
pub fn anonymous_tenant_id() -> TenantId {
    TenantId(Uuid::from_u128(ANONYMOUS_TENANT_U128))
}

/// The spend-enforcement context a transport is being built for. **Required**
/// by [`build_transport`](crate::transport::build_transport) and
/// [`build_transport_for_tenant`](crate::transport::build_transport_for_tenant)
/// so the choke point holds by construction — there is no way to build a bare,
/// un-gated transport.
///
/// ```
/// use argo_agent_runner::TransportGate;
/// // Operator-run CLI tooling opts out explicitly — greppable + auditable.
/// let _gate = TransportGate::InternalTrusted;
/// ```
pub enum TransportGate<'a> {
    /// An authenticated `(tenant, session)` agent path (the turn loop and
    /// its tier-router classifier). Spend is read from — and recorded by —
    /// the existing per-turn spend-attribution pipeline
    /// (`record_turn_spend`), so the gate is READ-ONLY here: it projects the
    /// proposed charge against the live ledger but must NOT record, or it
    /// would double-count.
    Session {
        /// Live session store the ledger reads run against.
        store: &'a Arc<SessionStore>,
        /// Acting tenant.
        tenant_id: TenantId,
        /// Acting session (defines the session window).
        session_id: SessionId,
        /// Workspace root used to resolve the active `[budget]` ceilings.
        cwd: &'a Path,
    },
    /// A pre-auth / anonymous gateway one-shot LLM path (pitch, Genesis,
    /// ghostwriter, orchestration-team, ...). Maps to the fixed capped
    /// anonymous tenant ([`anonymous_tenant_id`]). These paths do NOT run the
    /// spend-attribution pipeline, so the gate RESERVES (records) each
    /// allowed call's conservative projected cost itself — that accumulation
    /// is what makes the day/month ceiling actually bite. `store == None`
    /// (dev / test, no DB handle) is an uncapped passthrough so dev/demo is
    /// unchanged.
    Anonymous {
        /// Live session store, or `None` off the production path.
        store: Option<&'a Arc<SessionStore>>,
        /// Workspace root used to resolve the active `[budget]` ceilings.
        cwd: &'a Path,
    },
    /// Operator-run CLI tooling (forge skill-tuning, `skill self-improve`'s
    /// local invocation, local eval/judge). NOT internet-facing and NOT an
    /// autonomous-unbounded loop — the operator's own provider quota bounds
    /// it. Explicit, greppable escape hatch: every use is auditable in review.
    InternalTrusted,
}

impl TransportGate<'_> {
    /// Wrap `inner` with a [`SpendGate`] per this context. Returns `inner`
    /// unchanged for [`TransportGate::InternalTrusted`], for an `Anonymous`
    /// context with no store, and for any context whose resolved ceilings are
    /// all uncapped (the dev / demo default) — so there is zero per-call
    /// overhead off the production path.
    pub(crate) fn wrap(self, inner: Box<dyn Transport>) -> Box<dyn Transport> {
        match self {
            TransportGate::InternalTrusted => inner,
            TransportGate::Anonymous { store: None, .. } => inner,
            TransportGate::Session {
                store,
                tenant_id,
                session_id,
                cwd,
            } => wrap_spend_gate(store, tenant_id, session_id, cwd, false, inner),
            TransportGate::Anonymous {
                store: Some(store),
                cwd,
            } => wrap_spend_gate(
                store,
                anonymous_tenant_id(),
                // Fresh session each call: the session window never blocks a
                // one-shot anonymous call on its own; the tenant-scoped
                // day/month windows are what cap total pre-auth spend.
                SessionId::new(),
                cwd,
                true,
                inner,
            ),
        }
    }
}

/// Build the [`SpendGate`]-wrapped transport, or return `inner` unchanged when
/// the resolved ceilings are all uncapped (zero overhead off the prod path).
fn wrap_spend_gate(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    session_id: SessionId,
    cwd: &Path,
    reserve: bool,
    inner: Box<dyn Transport>,
) -> Box<dyn Transport> {
    let config = resolve_ceiling_config(cwd);
    if config.all_uncapped() {
        return inner;
    }
    Box::new(SpendGate::new(
        inner,
        Arc::new(StoreBudgetEnforcer::new(
            store.clone(),
            tenant_id,
            session_id,
            config,
            reserve,
        )),
    ))
}

/// Production [`BudgetEnforcer`] — reads the live `(session, day, month)`
/// spend totals for a `(tenant, session)` pair from a [`SessionStore`] and
/// projects the proposed charge against the configured [`CeilingConfig`]. On
/// a block it writes a tamper-evident [`AuditKind::BudgetCeilingBlocked`] row
/// **before** returning the structured [`ArgoError::BudgetExceeded`], so a
/// ceiling hit is never a silent failure.
///
/// When `reserve` is `true` (the anonymous gateway path, which has no
/// downstream spend-attribution), an allowed call RECORDS its conservative
/// projected cost against the ledger so the shared day/month windows
/// accumulate and the cap actually bites.
struct StoreBudgetEnforcer {
    store: Arc<SessionStore>,
    tenant_id: TenantId,
    session_id: SessionId,
    config: CeilingConfig,
    reserve: bool,
}

impl StoreBudgetEnforcer {
    fn new(
        store: Arc<SessionStore>,
        tenant_id: TenantId,
        session_id: SessionId,
        config: CeilingConfig,
        reserve: bool,
    ) -> Self {
        Self {
            store,
            tenant_id,
            session_id,
            config,
            reserve,
        }
    }

    /// Persist the honest "we stopped a runaway spend" breadcrumb. Best
    /// effort: an audit-write failure must not convert a *correct* block into
    /// a pass, so we log and still return the block error.
    async fn emit_block_audit(&self, err: &ArgoError) {
        let ArgoError::BudgetExceeded {
            window,
            used_micros,
            ceiling_micros,
            projected_cost_micros,
        } = err
        else {
            return;
        };
        let audit = AuditSink::new_with_tenant(self.store.clone(), self.tenant_id.to_string());
        let payload = serde_json::json!({
            "window": serde_json::to_value(window).unwrap_or(serde_json::Value::Null),
            "used_micros": used_micros,
            "ceiling_micros": ceiling_micros,
            "projected_cost_micros": projected_cost_micros,
        });
        if let Err(e) = audit
            .write(
                Some(self.session_id),
                AuditKind::BudgetCeilingBlocked,
                payload,
                None,
            )
            .await
        {
            tracing::warn!(
                target: "agent_runner::spend_gate",
                error = %e,
                "budget-ceiling block audit write failed",
            );
            return;
        }
        // Flush so the row is queryable immediately even on the rare path
        // where the turn aborts before the batch latency window.
        let _ = audit.flush_pending().await;
    }
}

#[async_trait]
impl BudgetEnforcer for StoreBudgetEnforcer {
    async fn enforce(&self, projected_cost_micros: u64) -> ArgoResult<()> {
        let check = self
            .store
            .check_ceiling(self.tenant_id, self.session_id, &self.config, Utc::now())
            .await?;
        if let Err(err) = check.project(projected_cost_micros) {
            self.emit_block_audit(&err).await;
            return Err(err);
        }
        // Anonymous path: no downstream attribution records this charge, so
        // reserve the projected cost now to drive the shared day/month cap.
        // Best-effort — a record failure must not block an otherwise-allowed
        // call, and the conservative projection means we never under-count.
        if self.reserve {
            if let Err(e) = self
                .store
                .record_spend(
                    self.tenant_id,
                    self.session_id,
                    Utc::now().timestamp_micros(),
                    projected_cost_micros,
                    "anon-gate",
                    "projected",
                )
                .await
            {
                tracing::warn!(
                    target: "agent_runner::spend_gate",
                    error = %e,
                    "anonymous spend reservation failed",
                );
            }
        }
        Ok(())
    }
}

/// Resolve the active ceiling config for this deployment: the configured
/// `[budget]` caps in production, fully uncapped elsewhere.
fn resolve_ceiling_config(cwd: &Path) -> CeilingConfig {
    let cfg = argo_config::load_from(cwd).ok();
    let production = gateway_profile_is_production()
        || cfg
            .as_ref()
            .map(|c| c.core.profile.as_str())
            .is_some_and(is_production_profile);
    if !production {
        return CeilingConfig::uncapped();
    }
    // Production: prefer the loaded `[budget]` caps; fall back to the shipped
    // defaults (NOT uncapped) when config is unreadable.
    let ceilings = cfg
        .as_ref()
        .map(|c| c.budget.to_cost_ceilings())
        .unwrap_or_else(|| argo_config::BudgetConfig::default().to_cost_ceilings());
    cost_ceilings_to_ceiling_config(&ceilings)
}

/// Map the `[budget]`-derived [`CostCeilings`] onto the gate's
/// [`CeilingConfig`] (drops the notification thresholds the gate doesn't use).
fn cost_ceilings_to_ceiling_config(c: &CostCeilings) -> CeilingConfig {
    CeilingConfig {
        per_session_usd_micros: c.per_session_usd_micros,
        per_day_usd_micros: c.per_day_usd_micros,
        per_month_usd_micros: c.per_month_usd_micros,
    }
}

fn gateway_profile_is_production() -> bool {
    std::env::var(GATEWAY_PROFILE_ENV).is_ok_and(|v| v.trim().eq_ignore_ascii_case("production"))
}

fn is_production_profile(profile: &str) -> bool {
    let p = profile.trim();
    p.eq_ignore_ascii_case("prod") || p.eq_ignore_ascii_case("production")
}

#[cfg(test)]
#[path = "transport_gate_tests.rs"]
mod tests;
