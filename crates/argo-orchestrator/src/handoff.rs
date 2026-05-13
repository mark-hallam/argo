//! `Handoff` — pass control of a session from one worker to the
//! next participant in the escalation chain.
//!
//! A handoff persists with `from`, `to`, opaque `payload` JSON,
//! `expected_outcome`, and an optional `deadline`. The receiving
//! worker sees pending handoffs on its next tick and acts on
//! them. Status flips through `Pending` → `Acknowledged` →
//! `Settled` (or `Expired`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::escalation::EscalationStep;
use crate::session::SessionId;
use argo_types::{ArgoError, ArgoResult};

/// Globally-unique handoff identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandoffId(String);

impl HandoffId {
    /// Construct a fresh `HandoffId`.
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7().to_string())
    }

    /// Borrow the raw id string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Wrap an existing id string.
    #[must_use]
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl Default for HandoffId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for HandoffId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Lifecycle of a [`Handoff`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffStatus {
    /// The receiver has not yet picked up the handoff.
    Pending,
    /// Receiver has acknowledged but not yet completed.
    Acknowledged,
    /// Receiver completed the handoff (`expected_outcome` met).
    Settled,
    /// Deadline passed without the receiver acting.
    Expired,
    /// Operator cancelled before completion.
    Cancelled,
}

/// A persisted handoff between two participants of a session.
///
/// ```
/// use argo_orchestrator::{Handoff, EscalationStep};
/// use argo_orchestrator::SessionId;
/// use argo_identity::{Persona, Role, Worker};
/// use argo_types::TenantId;
///
/// let from = Worker::draft(TenantId::new(), "v",
///     Persona::named("Maya", "x"), Role::new("x")).id;
/// let to = EscalationStep::Worker(
///     Worker::draft(TenantId::new(), "v",
///         Persona::named("Sam", "x"), Role::new("x")).id);
///
/// let h = Handoff::new(
///     SessionId::new(),
///     from,
///     to,
///     "verify the listing price",
///     serde_json::json!({"sku": "USD-AEON-60"}),
/// );
/// assert_eq!(h.expected_outcome, "verify the listing price");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handoff {
    /// Stable handoff id.
    pub id: HandoffId,
    /// Session this handoff belongs to.
    pub session_id: SessionId,
    /// Worker emitting the handoff.
    pub from: argo_identity::WorkerId,
    /// Receiving participant (worker or human).
    pub to: EscalationStep,
    /// What the receiver is expected to accomplish.
    pub expected_outcome: String,
    /// Opaque payload JSON (typically context the receiver needs).
    pub payload: serde_json::Value,
    /// Lifecycle state.
    pub status: HandoffStatus,
    /// When the handoff was emitted.
    pub created_at: DateTime<Utc>,
    /// Optional deadline; past it the handoff transitions to
    /// `Expired` on the next sweep.
    pub deadline: Option<DateTime<Utc>>,
}

impl Handoff {
    /// Construct a fresh pending handoff.
    #[must_use]
    pub fn new(
        session_id: SessionId,
        from: argo_identity::WorkerId,
        to: EscalationStep,
        expected_outcome: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: HandoffId::new(),
            session_id,
            from,
            to,
            expected_outcome: expected_outcome.into(),
            payload,
            status: HandoffStatus::Pending,
            created_at: Utc::now(),
            deadline: None,
        }
    }

    /// Mark the handoff `Acknowledged`. No-op unless `Pending`.
    pub fn acknowledge(&mut self) {
        if matches!(self.status, HandoffStatus::Pending) {
            self.status = HandoffStatus::Acknowledged;
        }
    }

    /// Mark the handoff `Settled`. Permitted from `Pending` OR
    /// `Acknowledged` — receivers may settle without an explicit
    /// ack if they finish in one tick.
    pub fn settle(&mut self) {
        if matches!(
            self.status,
            HandoffStatus::Pending | HandoffStatus::Acknowledged
        ) {
            self.status = HandoffStatus::Settled;
        }
    }

    /// Mark `Expired`. Only meaningful while still pending.
    pub fn expire(&mut self) {
        if matches!(self.status, HandoffStatus::Pending) {
            self.status = HandoffStatus::Expired;
        }
    }

    /// Mark `Cancelled`. Only Pending/Acknowledged can be cancelled.
    pub fn cancel(&mut self) -> ArgoResult<()> {
        match self.status {
            HandoffStatus::Pending | HandoffStatus::Acknowledged => { self.status = HandoffStatus::Cancelled; Ok(()) }
            HandoffStatus::Cancelled => Ok(()),
            HandoffStatus::Settled | HandoffStatus::Expired => Err(ArgoError::Invariant("invalid handoff status transition".into())),
        }
    }

    /// `true` iff `deadline` is set and has passed.
    #[must_use]
    pub fn is_overdue(&self, now: DateTime<Utc>) -> bool {
        self.deadline.is_some_and(|d| now > d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_identity::{Persona, Role, Worker};
    use argo_types::TenantId;

    fn worker_id() -> argo_identity::WorkerId {
        let w = Worker::draft(
            TenantId::new(),
            "v",
            Persona::named("M", "x"),
            Role::new("x"),
        );
        w.id
    }

    fn handoff() -> Handoff {
        let to = EscalationStep::Worker(worker_id());
        Handoff::new(
            SessionId::new(),
            worker_id(),
            to,
            "do thing",
            serde_json::json!({}),
        )
    }

    #[test]
    fn new_handoff_is_pending() {
        let h = handoff();
        assert_eq!(h.status, HandoffStatus::Pending);
    }

    #[test]
    fn acknowledge_then_settle() {
        let mut h = handoff();
        h.acknowledge();
        assert_eq!(h.status, HandoffStatus::Acknowledged);
        h.settle();
        assert_eq!(h.status, HandoffStatus::Settled);
    }

    #[test]
    fn settle_directly_from_pending() {
        let mut h = handoff();
        h.settle();
        assert_eq!(h.status, HandoffStatus::Settled);
    }

    #[test]
    fn expire_only_acts_on_pending() {
        let mut h = handoff();
        h.acknowledge();
        h.expire();
        // Acknowledged → Expire is a no-op.
        assert_eq!(h.status, HandoffStatus::Acknowledged);
    }

    #[test]
    fn settle_after_expire_is_no_op() {
        let mut h = handoff();
        h.expire();
        h.settle();
        assert_eq!(h.status, HandoffStatus::Expired);
    }

    #[test]
    fn cancel_from_pending_or_acknowledged() {
        let mut h = handoff();
        h.cancel().unwrap();
        assert_eq!(h.status, HandoffStatus::Cancelled);
        let mut h = handoff();
        h.acknowledge();
        h.cancel().unwrap();
        assert_eq!(h.status, HandoffStatus::Cancelled);
    }

    #[test]
    fn cancel_rejects_settled_or_expired() {
        let mut h = handoff();
        h.settle();
        assert!(h.cancel().is_err());
        let mut h = handoff();
        h.expire();
        assert!(h.cancel().is_err());
    }

    #[test]
    fn is_overdue_respects_deadline() {
        let mut h = handoff();
        h.deadline = Some(Utc::now() - chrono::Duration::seconds(60));
        assert!(h.is_overdue(Utc::now()));
        h.deadline = Some(Utc::now() + chrono::Duration::hours(1));
        assert!(!h.is_overdue(Utc::now()));
        h.deadline = None;
        assert!(!h.is_overdue(Utc::now()));
    }
}
