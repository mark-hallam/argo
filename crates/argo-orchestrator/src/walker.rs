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
use crate::escalation::{EscalationChain, EscalationStep};
use crate::handoff::{Handoff, HandoffStatus};
use crate::persist::OrchestratorStore;
use crate::routing::{RoutingContext, RoutingEngine};
use crate::session::{SessionId, SessionStatus};
use argo_audit::compute_chain_hash;
use argo_identity::WorkerId;
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::SqlitePool;
use std::str::FromStr;
use std::sync::Arc;

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

    /// One sweep. Idempotent — repeated calls on the same database
    /// are no-ops if no new handoffs went overdue between calls.
    pub async fn sweep_once(&self, now: DateTime<Utc>) -> ArgoResult<SweepReport> {
        let mut report = SweepReport::default();
        for row in self.find_overdue_handoffs(now).await? {
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

    async fn find_overdue_handoffs(&self, now: DateTime<Utc>) -> ArgoResult<Vec<OverdueRow>> {
        let rows = sqlx::query_as::<_, RawOverdue>(
            "SELECT h.id AS handoff_id, h.session_id, s.tenant_id, h.from_worker_id, \
             h.\"to\" AS to_field, h.expected_outcome, h.payload, h.created_at, h.deadline \
             FROM handoff h JOIN session s ON s.id = h.session_id \
             WHERE h.status = 'pending' AND h.deadline IS NOT NULL AND h.deadline < ? \
             ORDER BY h.deadline ASC",
        )
        .bind(now.timestamp_micros())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("walker overdue query: {e}")))?;
        rows.into_iter().map(OverdueRow::try_from).collect()
    }
}

#[derive(sqlx::FromRow)]
struct RawOverdue {
    handoff_id: String,
    session_id: Vec<u8>,
    tenant_id: Vec<u8>,
    from_worker_id: String,
    to_field: String,
    expected_outcome: String,
    payload: String,
    created_at: i64,
    deadline: Option<i64>,
}

struct OverdueRow {
    handoff: Handoff,
    tenant_id: TenantId,
}

impl TryFrom<RawOverdue> for OverdueRow {
    type Error = ArgoError;
    fn try_from(r: RawOverdue) -> ArgoResult<Self> {
        let session_id = SessionId::from_string(
            uuid::Uuid::from_slice(&r.session_id)
                .map_err(|e| ArgoError::Schema(format!("session uuid: {e}")))?
                .to_string(),
        );
        let tenant_id = TenantId(
            uuid::Uuid::from_slice(&r.tenant_id)
                .map_err(|e| ArgoError::Schema(format!("tenant uuid: {e}")))?,
        );
        let from = WorkerId::from_str(&r.from_worker_id)
            .map_err(|e| ArgoError::Schema(format!("worker uuid: {e}")))?;
        let to = parse_step_from_persist_json(&r.to_field)?;
        let payload: Value = serde_json::from_str(&r.payload)
            .map_err(|e| ArgoError::Schema(format!("payload: {e}")))?;
        Ok(Self {
            handoff: Handoff {
                id: crate::handoff::HandoffId::from_string(r.handoff_id),
                session_id,
                from,
                to,
                expected_outcome: r.expected_outcome,
                payload,
                status: HandoffStatus::Pending,
                created_at: chrono::DateTime::<Utc>::from_timestamp_micros(r.created_at)
                    .unwrap_or_else(Utc::now),
                deadline: r
                    .deadline
                    .and_then(chrono::DateTime::<Utc>::from_timestamp_micros),
            },
            tenant_id,
        })
    }
}

/// Parse the persisted `handoff."to"` JSON shape (mirrors
/// `persist::step_from_json` but lives here so the walker doesn't
/// import private symbols).
fn parse_step_from_persist_json(json: &str) -> ArgoResult<EscalationStep> {
    let v: Value = serde_json::from_str(json)
        .map_err(|e| ArgoError::Schema(format!("handoff.to JSON: {e}")))?;
    match v.get("kind").and_then(Value::as_str) {
        Some("worker") => {
            let raw = v
                .get("worker_id")
                .and_then(Value::as_str)
                .ok_or_else(|| ArgoError::Schema("handoff.to worker_id missing".into()))?;
            WorkerId::from_str(raw)
                .map(EscalationStep::Worker)
                .map_err(|e| ArgoError::Schema(format!("handoff.to worker_id: {e}")))
        }
        Some("human") => v
            .get("addr")
            .and_then(Value::as_str)
            .map(|s| EscalationStep::Human(s.to_string()))
            .ok_or_else(|| ArgoError::Schema("handoff.to addr missing".into())),
        _ => Err(ArgoError::Schema("handoff.to kind unknown".into())),
    }
}

fn parse_chain_from_payload(payload: &Value) -> Option<EscalationChain> {
    let arr = payload.get("escalation_chain")?.as_array()?;
    let strs: Vec<&str> = arr.iter().filter_map(Value::as_str).collect();
    Some(EscalationChain::from_strs(&strs))
}

fn parse_index_from_payload(payload: &Value) -> usize {
    payload
        .get("escalation_index")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(0)
}

fn stringify_step(step: &EscalationStep) -> String {
    match step {
        EscalationStep::Worker(id) => format!("worker:{id}"),
        EscalationStep::Human(addr) => format!("human:{addr}"),
    }
}

/// Insert an audit row labelled `escalation_exhausted` for the
/// supplied session. Mirrors what `AuditSink::write` would do but
/// avoids requiring a new variant on `AuditKind` (the brief
/// scopes the walker to `argo-orchestrator` + the gateway sweeper
/// — adding to `argo-audit` would expand the surface).
async fn write_escalation_exhausted_audit(
    pool: &SqlitePool,
    session_id: SessionId,
    payload: Value,
) -> ArgoResult<()> {
    let kind_label = "escalation_exhausted";
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Schema(format!("audit payload: {e}")))?;
    let session_blob = uuid::Uuid::parse_str(session_id.as_str())
        .map_err(|e| ArgoError::Schema(format!("session uuid: {e}")))?;
    let parent_hash = last_session_hash(pool, &session_blob).await?;
    let self_hash = compute_chain_hash(&parent_hash, kind_label, &canonical);
    let audit_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, \
         self_hash, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(audit_id.as_bytes().as_slice())
    .bind(session_blob.as_bytes().as_slice())
    .bind(kind_label)
    .bind(canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(Utc::now().timestamp_micros())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("audit insert: {e}")))?;
    Ok(())
}

async fn last_session_hash(pool: &SqlitePool, session: &uuid::Uuid) -> ArgoResult<[u8; 32]> {
    let row: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id = ? \
         ORDER BY seq DESC LIMIT 1",
    )
    .bind(session.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("audit last_hash: {e}")))?;
    let mut buf = [0u8; 32];
    if let Some(bytes) = row {
        if bytes.len() == 32 {
            buf.copy_from_slice(&bytes);
        }
    }
    Ok(buf)
}

#[cfg(test)]
#[path = "walker_tests.rs"]
mod tests;
