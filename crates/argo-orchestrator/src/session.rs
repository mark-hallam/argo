//! `Session` — a thread of work joining N workers + N humans.
//!
//! Sessions are tenant-scoped. The session_id flows through every
//! action emitted under it, so audit + memory queries can group
//! by session. Workers join a session via the orchestrator's
//! `join_session` call (wiring deferred to a follow-up); the
//! worker's [`MemoryNs::shared_team`] memory is then visible to
//! every member of the session.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Globally-unique session identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Construct a fresh `SessionId` (UUID v4 under the hood).
    ///
    /// ```
    /// use argo_orchestrator::SessionId;
    /// let id = SessionId::new();
    /// assert_eq!(id.as_str().len(), 36); // UUID hyphenated form
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7().to_string())
    }

    /// Borrow the raw id string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Wrap an existing id string (for hydrating from the store).
    #[must_use]
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Lifecycle state of a [`Session`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Session is open; participants may emit work.
    Active,
    /// Session resolved successfully; no further work expected.
    Resolved,
    /// Session was cancelled by an operator before resolution.
    Cancelled,
    /// Session timed out (a deadline passed without resolution).
    Expired,
}

/// A multi-worker / multi-human work thread.
///
/// ```
/// use argo_orchestrator::{Session, SessionStatus};
/// use argo_types::TenantId;
///
/// let s = Session::new(TenantId::new(), "skate-followup");
/// assert_eq!(s.status, SessionStatus::Active);
/// assert_eq!(s.title, "skate-followup");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Stable session id.
    pub id: SessionId,
    /// Tenant this session belongs to.
    pub tenant_id: argo_types::TenantId,
    /// Short human-facing title (operator-visible).
    pub title: String,
    /// Lifecycle state.
    pub status: SessionStatus,
    /// Worker ids currently joined.
    pub participants: Vec<argo_identity::WorkerId>,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// Optional resolution deadline.
    pub deadline: Option<DateTime<Utc>>,
}

impl Session {
    /// Create a new active session with no participants yet.
    #[must_use]
    pub fn new(tenant_id: argo_types::TenantId, title: impl Into<String>) -> Self {
        let id = SessionId::new();
        // Fires once per logical session creation. Surfaces the
        // session start at INFO so SREs can grep
        // `event=session.start tenant_id=… session_id=…` even when
        // no caller has wrapped work in `enter_log_context()` yet.
        tracing::info!(
            event = "session.start",
            tenant_id = %tenant_id,
            session_id = %id,
            "session created",
        );
        Self {
            id,
            tenant_id,
            title: title.into(),
            status: SessionStatus::Active,
            participants: Vec::new(),
            created_at: Utc::now(),
            deadline: None,
        }
    }

    /// Enter a `tracing` span carrying the session's `tenant_id` and
    /// `session_id` as fields. Returns an `EnteredSpan` guard that
    /// exits the span when dropped — bind it to a local for the scope
    /// of the session-scoped work. The
    /// `argo_telemetry::span_context::ContextInjectorLayer` (when
    /// installed in the global subscriber) walks the span's fields
    /// and stamps every log line emitted under the guard with
    /// `tenant_id=… session_id=…` (Stream
    /// `mission-structured-logging-tenant-worker-session`).
    ///
    /// `worker_id` is not part of the session-level context — workers
    /// joining the session enter their own [`enter_context`] guard
    /// on top, and the layer composes the two (event-author wins,
    /// then ancestor).
    ///
    /// [`enter_context`]: ../../../argo-telemetry/src/span_context.rs
    #[must_use = "drop the returned guard at end of session-scoped work"]
    pub fn enter_log_context(&self) -> tracing::span::EnteredSpan {
        tracing::info_span!(
            "session",
            tenant_id = %self.tenant_id,
            session_id = %self.id,
        )
        .entered()
    }

    /// Add a worker as a participant. Idempotent — adding the
    /// same worker twice is a no-op.
    pub fn add_participant(&mut self, worker_id: argo_identity::WorkerId) {
        if !self.participants.contains(&worker_id) {
            self.participants.push(worker_id);
        }
    }

    /// Mark the session resolved. No-op if not currently active.
    pub fn resolve(&mut self) {
        if matches!(self.status, SessionStatus::Active) {
            self.status = SessionStatus::Resolved;
        }
    }

    /// Mark the session cancelled. No-op if not currently active.
    pub fn cancel(&mut self) {
        if matches!(self.status, SessionStatus::Active) {
            self.status = SessionStatus::Cancelled;
        }
    }

    /// Returns `true` iff the session is still accepting work.
    #[must_use]
    pub fn is_open(&self) -> bool {
        matches!(self.status, SessionStatus::Active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_identity::{Persona, Role, Worker};
    use argo_types::TenantId;

    fn worker() -> argo_identity::WorkerId {
        let w = Worker::draft(
            TenantId::new(),
            "vert",
            Persona::named("Maya", "test"),
            Role::new("test"),
        );
        w.id
    }

    #[test]
    fn new_session_is_active_with_zero_participants() {
        let s = Session::new(TenantId::new(), "demo");
        assert!(s.is_open());
        assert!(s.participants.is_empty());
    }

    #[test]
    fn add_participant_is_idempotent() {
        let mut s = Session::new(TenantId::new(), "demo");
        let w = worker();
        s.add_participant(w);
        s.add_participant(w);
        assert_eq!(s.participants.len(), 1);
    }

    #[test]
    fn resolve_only_acts_on_active() {
        let mut s = Session::new(TenantId::new(), "demo");
        s.cancel();
        // Cancel locks; resolve must be a no-op now.
        s.resolve();
        assert_eq!(s.status, SessionStatus::Cancelled);
    }

    #[test]
    fn session_id_round_trips_through_string() {
        let id = SessionId::new();
        let raw = id.to_string();
        let restored = SessionId::from_string(raw);
        assert_eq!(id, restored);
    }
}
