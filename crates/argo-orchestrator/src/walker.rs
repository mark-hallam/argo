//! Periodic walker that escalates overdue Pending handoffs along
//! their declared `EscalationChain` (Stream
//! `claude-orchestrator-handoff-walker`).
//!
//! ## What it does
//!
//! On each tick, [`HandoffWalker::sweep_once`]:
//! 1. Finds every handoff with `status = Pending` AND `deadline <
//!    now` (across every tenant, since the gateway runs one sweeper).
//! 2. For each: marks `Expired` (per [`Handoff::expire`]).
//! 3. If the originating handoff carried an `escalation_chain` +
//!    `escalation_index` in its payload, walks
//!    [`EscalationChain::advance`] to the next step:
//!    - `Worker(id)` → emits a NEW Pending handoff to that worker
//!      with the same `expected_outcome` + a fresh deadline.
//!    - `Human(addr)` → calls the [`HandoffNotifier`] (which
//!      resolves the address scheme — `mailto:` → email, `slack:`
//!      → slack, etc — and dispatches a notification).
//! 4. If the chain is exhausted (or never had a chain to begin
//!    with), writes an audit row labelled `escalation_exhausted`
//!    (with the original `session_id`) AND flips the parent
//!    session to [`SessionStatus::Expired`].
//!
//! ## Idempotency
//!
//! The first action taken on each handoff is the
//! `Pending → Expired` flip. Once that's persisted, subsequent
//! sweeps see the row in `Expired` state and skip it. So a slow
//! sweep that races a fast sweep won't double-emit.
//!
//! ## Schema-coupled queries
//!
//! The brief restricts the [`OrchestratorStore`] persist API to
//! read-only consumption — but the existing surface only lists
//! handoffs per-session or per-worker, not "overdue across
//! tenants". The walker uses raw sqlx against the same
//! `audit_record` + `handoff` + `session` tables to find overdue
//! rows, then delegates state transitions to the typed persist
//! methods so invariants (status transition checks etc.) still apply.
//!
//! ## File layout
//!
//! Persistence + parsing helpers (cross-tenant overdue query,
//! payload decoders, exhausted-chain audit writer) live in
//! [`crate::walker_helpers`] to keep this file under the 500-LOC
//! ceiling (HARD RULE 6). Pure refactor split — no behaviour
//! change.
use crate::escalation::{EscalationChain, EscalationStep};
use crate::handoff::{Handoff, HandoffStatus};
use crate::persist::OrchestratorStore;
use crate::routing::{RoutingContext, RoutingEngine};
use crate::session::SessionStatus;
use crate::walker_helpers::{
    find_overdue_handoffs, parse_chain_from_payload, parse_index_from_payload, stringify_step,
    write_escalation_exhausted_audit, OverdueRow,
};
use argo_identity::WorkerId;
use argo_types::ArgoResult;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;
use std::sync::Arc;

/// Policy hook consulted before [`RoutingEngine`] in
/// [`HandoffWalker::routing_override`] (Stream `claude-worker-handoff-policy`).
pub trait HandoffPolicyHook: Send + Sync {
    /// `Some(w)` re-routes the next handoff to `w`; `None` defers to the engine.
    fn override_target(&self, expected_outcome: &str) -> Option<WorkerId>;
}

/// How the walker reaches an external operator (email / slack /
/// SMS / push) for a `Human` step. The walker doesn't care which
/// channel handles which scheme — implementations route by the
/// `addr` prefix (`mailto:`, `slack:`, `sms:`, etc) and fall back
/// to email when ambiguous.
#[async_trait]
pub trait HandoffNotifier: Send + Sync {
    /// Notify a human at `addr` with `summary`. Returns a delivery
    /// id for audit. Errors propagate; the walker treats a notify
    /// failure as a chain-exhausted event so the operator still
    /// learns the handoff couldn't be delivered.
    async fn notify_human(&self, addr: &str, summary: &str) -> ArgoResult<String>;
}

/// Outcome of one sweep. Returned so `argo doctor` / metrics can
/// observe walker activity without scraping logs.
#[derive(Debug, Default, Clone, Copy)]
pub struct SweepReport {
    /// Handoffs flipped from Pending to Expired this tick.
    pub expired: usize,
    /// New handoffs emitted to advance the chain.
    pub re_handed_off: usize,
    /// Chains that ran out of steps this tick.
    pub chain_exhausted: usize,
    /// Successful Human notifications dispatched.
    pub notifications: usize,
}

/// Periodic walker that drives escalation autonomy.
pub struct HandoffWalker {
    store: Arc<OrchestratorStore>,
    pool: SqlitePool,
    notifier: Arc<dyn HandoffNotifier>,
    /// Default deadline for re-handed-off handoffs. The original
    /// handoff's deadline is honoured if the payload encodes one;
    /// otherwise this constant applies.
    default_deadline_secs: i64,
    /// Optional routing engine consulted before chain advance.
    /// `None` (the default) preserves pre-routing-rules behaviour
    /// — the chain's next target wins unconditionally.
    routing_engine: Option<RoutingEngine>,
    /// Optional auction store (Stream `claude-skill-auction`) —
    /// when set, an exhausted-chain fallthrough records a
    /// `no_bidders` auction row in the audit trail before flipping
    /// the session to `Expired`. v1 doesn't run the auction
    /// in-process; bidding is the operator's hookup point.
    auction_store: Option<Arc<crate::auction::AuctionStore>>,
    policy_hook: Option<Arc<dyn HandoffPolicyHook>>,
}

impl HandoffWalker {
    /// Construct a walker bound to the given store + notifier.
    /// `pool` is the underlying [`sqlx::SqlitePool`] — needed for
    /// the cross-tenant overdue query that the persist API
    /// doesn't expose.
    #[must_use]
    pub fn new(
        store: Arc<OrchestratorStore>,
        pool: SqlitePool,
        notifier: Arc<dyn HandoffNotifier>,
    ) -> Self {
        Self {
            store,
            pool,
            notifier,
            default_deadline_secs: 60 * 60, // 1 hour
            routing_engine: None,
            auction_store: None,
            policy_hook: None,
        }
    }

    /// Override the default deadline for re-handed-off handoffs.
    /// Mostly for tests.
    #[must_use]
    pub fn with_default_deadline(mut self, secs: i64) -> Self {
        self.default_deadline_secs = secs;
        self
    }

    /// Attach a [`RoutingEngine`] consulted before the static
    /// escalation chain's next target. When the engine returns
    /// `Some(EscalationStep::Worker(w))`, the walker re-emits the
    /// handoff to `w` instead of the chain's pick.
    #[must_use]
    pub fn with_routing_engine(mut self, engine: RoutingEngine) -> Self {
        self.routing_engine = Some(engine);
        self
    }

    /// Attach an [`crate::auction::AuctionStore`] (Stream
    /// `claude-skill-auction`). When set, exhausted-chain fall-
    /// throughs record a `no_bidders` auction row in
    /// `auction_log` before the human-operator notification
    /// fires, giving the audit trail a marker for "tried fallback
    /// auction, no bidders → escalated to human".
    #[must_use]
    pub fn with_auction_store(mut self, store: Arc<crate::auction::AuctionStore>) -> Self {
        self.auction_store = Some(store);
        self
    }

    /// Attach a [`HandoffPolicyHook`] consulted before the routing engine.
    #[must_use]
    pub fn with_policy_hook(mut self, hook: Arc<dyn HandoffPolicyHook>) -> Self {
        self.policy_hook = Some(hook); self
    }

    /// One sweep. Idempotent — repeated calls on the same database
    /// are no-ops if no new handoffs went overdue between calls.
    pub async fn sweep_once(&self, now: DateTime<Utc>) -> ArgoResult<SweepReport> {
        let mut report = SweepReport::default();
        for row in find_overdue_handoffs(&self.pool, now).await? {
            self.process_overdue(row, now, &mut report).await?;
        }
        Ok(report)
    }

    async fn process_overdue(
        &self,
        row: OverdueRow,
        now: DateTime<Utc>,
        report: &mut SweepReport,
    ) -> ArgoResult<()> {
        // Step 1: flip Pending → Expired. This is the
        // synchronization point; sub-sequent sweeps skip the row.
        self.store
            .update_handoff_status(row.tenant_id, &row.handoff.id, HandoffStatus::Expired)
            .await?;
        report.expired += 1;

        // Step 2: try to advance the chain.
        let payload = &row.handoff.payload;
        let current_index = parse_index_from_payload(payload);
        let Some(chain) = parse_chain_from_payload(payload) else {
            self.exhaust_chain(&row).await?;
            report.chain_exhausted += 1;
            return Ok(());
        };
        let Some((next_index, next_step)) = chain.advance(current_index) else {
            self.exhaust_chain(&row).await?;
            report.chain_exhausted += 1;
            return Ok(());
        };
        match next_step {
            EscalationStep::Worker(next_worker) => {
                let target = self
                    .routing_override(&row.handoff.expected_outcome, now)
                    .unwrap_or(*next_worker);
                self.advance_to_worker(&row, &chain, next_index, target, now)
                    .await?;
                report.re_handed_off += 1;
            }
            EscalationStep::Human(addr) => {
                let summary = self.summarize(&row.handoff);
                if self.notifier.notify_human(addr, &summary).await.is_ok() {
                    report.notifications += 1;
                } else {
                    // Notify failure → treat as exhaustion so the
                    // operator still learns the chain stopped.
                    self.exhaust_chain(&row).await?;
                    report.chain_exhausted += 1;
                }
            }
        }
        Ok(())
    }

    async fn advance_to_worker(
        &self,
        row: &OverdueRow,
        chain: &EscalationChain,
        next_index: usize,
        next_worker: WorkerId,
        now: DateTime<Utc>,
    ) -> ArgoResult<()> {
        let next_payload = serde_json::json!({
            "escalation_chain": chain.steps()
                .iter()
                .map(stringify_step)
                .collect::<Vec<_>>(),
            "escalation_index": next_index,
            "previous_handoff_id": row.handoff.id.as_str(),
            "user_payload": row.handoff.payload.get("user_payload")
                .cloned()
                .unwrap_or_else(|| row.handoff.payload.clone()),
        });
        let mut next = Handoff::new(
            row.handoff.session_id.clone(),
            row.handoff.from,
            EscalationStep::Worker(next_worker),
            row.handoff.expected_outcome.clone(),
            next_payload,
        );
        next.deadline = Some(now + Duration::seconds(self.default_deadline_secs));
        self.store.insert_handoff(row.tenant_id, &next).await?;
        Ok(())
    }

    async fn exhaust_chain(&self, row: &OverdueRow) -> ArgoResult<()> {
        // Stream `claude-skill-auction` — record a no_bidders
        // auction row before exhausting, so the audit trail shows
        // the fallback was attempted. Errors here are logged + swallowed
        // so a transient sqlite hiccup doesn't block the existing
        // human-operator notification path.
        if let Some(store) = &self.auction_store {
            let task_id = format!("auction-{}", row.handoff.id.as_str());
            let mut auction = crate::auction::Auction::open(crate::auction::AuctionTask {
                id: task_id,
                description: row.handoff.expected_outcome.clone(),
                required_skills: Vec::new(),
            });
            auction.close();
            if let Err(e) = store.insert_auction(&auction).await {
                tracing::warn!(
                    target: "argo_orchestrator::auction",
                    error = %e,
                    "failed to record no_bidders auction; continuing to escalate",
                );
            }
        }
        let payload = serde_json::json!({
            "session_id": row.handoff.session_id.as_str(),
            "handoff_id": row.handoff.id.as_str(),
            "from_worker_id": row.handoff.from.to_string(),
            "expected_outcome": row.handoff.expected_outcome,
            "reason": "chain_exhausted_or_missing",
        });
        write_escalation_exhausted_audit(&self.pool, row.handoff.session_id.clone(), payload)
            .await?;
        self.store
            .update_session_status(row.tenant_id, &row.handoff.session_id, SessionStatus::Expired)
            .await?;
        Ok(())
    }

    /// Consult the routing engine. Returns `Some(worker)` when a
    /// rule fires with a `SpecificWorker` / `RoundRobin` target.
    /// The walker only re-routes to workers — `Human` selectors
    /// are not currently supported and fall through to the chain.
    fn routing_override(
        &self,
        expected_outcome: &str,
        now: DateTime<Utc>,
    ) -> Option<WorkerId> {
        // policy hook (Stream `claude-worker-handoff-policy`)
        if let Some(w) = self.policy_hook.as_ref().and_then(|h| h.override_target(expected_outcome)) {
            return Some(w);
        }
        let engine = self.routing_engine.as_ref()?;
        let ctx = RoutingContext::new(now, expected_outcome);
        match engine.route(&ctx)? {
            EscalationStep::Worker(w) => Some(w),
            EscalationStep::Human(_) => None,
        }
    }

    fn summarize(&self, h: &Handoff) -> String {
        format!(
            "Handoff escalated: session={}, expected={}, deadline missed",
            h.session_id.as_str(),
            h.expected_outcome
        )
    }
}

#[cfg(test)]
#[path = "walker_tests.rs"]
mod tests;
