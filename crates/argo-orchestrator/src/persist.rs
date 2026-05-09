//! SQLite persistence for orchestrator sessions and handoffs.
//!
//! Handoff-row I/O and the cancel-audit chain hand-off live in the
//! [`crate::persist_handoffs`] sibling (HARD RULE 6 defensive split,
//! brief `claude-orchestrator-persist-split`). This file keeps the
//! [`OrchestratorStore`] type, schema bring-up, session-row methods,
//! and the shared codec helpers re-used by both files.
use crate::{Session, SessionId, SessionStatus};
use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{TimeZone, Utc};
use sqlx::Row;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) const ORCHESTRATOR_PROVIDER: &str = "orchestrator";
/// Repository for persisted orchestrator sessions and handoffs.
pub struct OrchestratorStore {
    store: Arc<SessionStore>,
}

impl OrchestratorStore {
    /// Build over an open, migrated [`SessionStore`].
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { store }
    }

    /// Borrow the underlying SQLite pool. `pub(crate)` so sibling persist modules
    /// (e.g. [`crate::persist_handoffs`]) can share the connection without exposing the field.
    pub(crate) fn pool(&self) -> &sqlx::Pool<sqlx::Sqlite> { self.store.pool() }

    /// Create the additive orchestrator schema if it is missing.
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        let pool = self.store.pool();
        add_column_if_missing(pool, "session", "status", "TEXT NOT NULL DEFAULT 'active'").await?;
        add_column_if_missing(
            pool,
            "session",
            "participants",
            "TEXT NOT NULL DEFAULT '[]'",
        )
        .await?;
        add_column_if_missing(pool, "session", "created_at", "INTEGER NOT NULL DEFAULT 0").await?;
        add_column_if_missing(pool, "session", "deadline", "INTEGER").await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_orchestrator_session_active \
             ON session(tenant_id, status, created_at)",
        )
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("orchestrator session index: {e}")))?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS handoff (\
             id TEXT PRIMARY KEY, \
             session_id BLOB NOT NULL REFERENCES session(id) ON DELETE CASCADE, \
             from_worker_id TEXT NOT NULL, \
             \"to\" TEXT NOT NULL, \
             expected_outcome TEXT NOT NULL, \
             payload TEXT NOT NULL, \
             status TEXT NOT NULL, \
             created_at INTEGER NOT NULL, \
             deadline INTEGER)",
        )
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("orchestrator handoff table: {e}")))?;
        add_column_if_missing(pool, "handoff", "resolved_at", "INTEGER").await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_handoff_session ON handoff(session_id, created_at)",
        )
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("handoff session index: {e}")))?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_handoff_status ON handoff(status, created_at)")
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("handoff status index: {e}")))?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_handoff_resolved_at ON handoff(resolved_at) WHERE resolved_at IS NOT NULL").execute(pool).await.map_err(|e| ArgoError::Provider(format!("handoff resolved_at index: {e}")))?;
        Ok(())
    }

    /// Insert a new orchestrator session.
    pub async fn insert_session(&self, session: &Session) -> ArgoResult<()> {
        self.ensure_schema().await?;
        let id = session_id_blob(&session.id)?;
        let tenant = tenant_blob(session.tenant_id);
        sqlx::query(
            "INSERT OR IGNORE INTO workspace (id, tenant_id, name, root_path, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(tenant.as_slice())
        .bind(tenant.as_slice())
        .bind(ORCHESTRATOR_PROVIDER)
        .bind("")
        .bind(session.created_at.timestamp_micros())
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("orchestrator workspace: {e}")))?;
        sqlx::query(
            "INSERT INTO session (id, tenant_id, workspace_id, title, started_at, \
             model, provider, status, participants, created_at, deadline) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_slice())
        .bind(tenant.as_slice())
        .bind(tenant.as_slice())
        .bind(&session.title)
        .bind(session.created_at.timestamp_micros())
        .bind(ORCHESTRATOR_PROVIDER)
        .bind(ORCHESTRATOR_PROVIDER)
        .bind(session_status_label(session.status))
        .bind(participants_json(&session.participants)?)
        .bind(session.created_at.timestamp_micros())
        .bind(session.deadline.map(|d| d.timestamp_micros()))
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("insert orchestrator session: {e}")))?;
        Ok(())
    }

    /// Find one orchestrator session inside a tenant.
    pub async fn find_session(
        &self,
        tenant_id: TenantId,
        session_id: &SessionId,
    ) -> ArgoResult<Option<Session>> {
        self.ensure_schema().await?;
        let row = sqlx::query_as::<_, RawSession>(SESSION_SELECT)
            .bind(session_id_blob(session_id)?.as_slice())
            .bind(tenant_blob(tenant_id).as_slice())
            .bind(ORCHESTRATOR_PROVIDER)
            .fetch_optional(self.store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("find orchestrator session: {e}")))?;
        row.map(Session::try_from).transpose()
    }

    /// List active orchestrator sessions for one tenant.
    pub async fn list_active_sessions(&self, tenant_id: TenantId) -> ArgoResult<Vec<Session>> {
        self.ensure_schema().await?;
        let rows = sqlx::query_as::<_, RawSession>(
            "SELECT id, tenant_id, title, status, participants, created_at, deadline \
             FROM session WHERE tenant_id = ? AND provider = ? AND status = 'active' \
             ORDER BY created_at DESC, id ASC",
        )
        .bind(tenant_blob(tenant_id).as_slice())
        .bind(ORCHESTRATOR_PROVIDER)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list active orchestrator sessions: {e}")))?;
        rows.into_iter().map(Session::try_from).collect()
    }

    /// Update a session lifecycle status.
    pub async fn update_session_status(
        &self,
        tenant_id: TenantId,
        session_id: &SessionId,
        status: SessionStatus,
    ) -> ArgoResult<()> {
        let current = self
            .find_session(tenant_id, session_id)
            .await?
            .ok_or_else(|| ArgoError::Schema("orchestrator session not found".into()))?
            .status;
        if !session_transition_allowed(current, status) {
            return Err(ArgoError::Invariant("invalid session status transition"));
        }
        sqlx::query(
            "UPDATE session SET status = ? WHERE id = ? AND tenant_id = ? AND provider = ?",
        )
        .bind(session_status_label(status))
        .bind(session_id_blob(session_id)?.as_slice())
        .bind(tenant_blob(tenant_id).as_slice())
        .bind(ORCHESTRATOR_PROVIDER)
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("update orchestrator session status: {e}")))?;
        Ok(())
    }

    /// Add one worker participant to a session. The operation is idempotent.
    pub async fn add_participant(
        &self,
        tenant_id: TenantId,
        session_id: &SessionId,
        worker_id: WorkerId,
    ) -> ArgoResult<()> {
        let mut session = self
            .find_session(tenant_id, session_id)
            .await?
            .ok_or_else(|| ArgoError::Schema("orchestrator session not found".into()))?;
        session.add_participant(worker_id);
        sqlx::query(
            "UPDATE session SET participants = ? WHERE id = ? AND tenant_id = ? AND provider = ?",
        )
        .bind(participants_json(&session.participants)?)
        .bind(session_id_blob(session_id)?.as_slice())
        .bind(tenant_blob(tenant_id).as_slice())
        .bind(ORCHESTRATOR_PROVIDER)
        .execute(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("add orchestrator participant: {e}")))?;
        Ok(())
    }
}

const SESSION_SELECT: &str = "SELECT id, tenant_id, title, status, participants, created_at, \
deadline FROM session WHERE id = ? AND tenant_id = ? AND provider = ?";

#[derive(sqlx::FromRow)]
struct RawSession { id: Vec<u8>, tenant_id: Vec<u8>, title: Option<String>, status: String, participants: String, created_at: i64, deadline: Option<i64> }

impl TryFrom<RawSession> for Session {
    type Error = ArgoError;
    fn try_from(r: RawSession) -> ArgoResult<Self> {
        Ok(Self { id: session_id_from_blob(&r.id)?, tenant_id: tenant_from_blob(&r.tenant_id)?,
            title: r.title.unwrap_or_default(), status: parse_session_status(&r.status)?,
            participants: parse_participants(&r.participants)?,
            created_at: micros_to_utc(r.created_at, "session.created_at")?,
            deadline: opt_micros(r.deadline, "session.deadline")? })
    }
}

async fn add_column_if_missing(pool: &sqlx::Pool<sqlx::Sqlite>, table: &str, column: &str, definition: &str) -> ArgoResult<()> {
    if column_exists(pool, table, column).await? { return Ok(()); }
    sqlx::query(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"))
        .execute(pool).await.map_err(|e| ArgoError::Provider(format!("orchestrator schema column {column}: {e}")))?;
    Ok(())
}

async fn column_exists(pool: &sqlx::Pool<sqlx::Sqlite>, table: &str, column: &str) -> ArgoResult<bool> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table})")).fetch_all(pool).await
        .map_err(|e| ArgoError::Provider(format!("orchestrator schema inspect: {e}")))?;
    for row in rows {
        let name: String = row.try_get("name").map_err(|e| ArgoError::Provider(format!("orchestrator schema column name: {e}")))?;
        if name == column { return Ok(true); }
    }
    Ok(false)
}

fn session_status_label(s: SessionStatus) -> &'static str { match s { SessionStatus::Active => "active", SessionStatus::Resolved => "resolved", SessionStatus::Cancelled => "cancelled", SessionStatus::Expired => "expired" } }
fn parse_session_status(s: &str) -> ArgoResult<SessionStatus> { match s { "active" => Ok(SessionStatus::Active), "resolved" => Ok(SessionStatus::Resolved), "cancelled" => Ok(SessionStatus::Cancelled), "expired" => Ok(SessionStatus::Expired), other => Err(ArgoError::Schema(format!("session.status: {other}"))) } }
fn session_transition_allowed(current: SessionStatus, next: SessionStatus) -> bool { matches!(current, SessionStatus::Active) || current == next }

fn participants_json(ps: &[WorkerId]) -> ArgoResult<String> { to_json(&ps.iter().map(ToString::to_string).collect::<Vec<_>>(), "session.participants") }
fn parse_participants(json: &str) -> ArgoResult<Vec<WorkerId>> {
    from_json::<Vec<String>>(json, "session.participants")?.into_iter()
        .map(|id| WorkerId::from_str(&id).map_err(|e| ArgoError::Schema(format!("session.participant: {e}")))).collect()
}
pub(crate) fn tenant_blob(id: TenantId) -> [u8; 16] { *id.0.as_bytes() }
fn tenant_from_blob(b: &[u8]) -> ArgoResult<TenantId> { Uuid::from_slice(b).map(TenantId).map_err(|e| ArgoError::Schema(format!("session.tenant_id: {e}"))) }
pub(crate) fn session_id_blob(id: &SessionId) -> ArgoResult<[u8; 16]> { Uuid::parse_str(id.as_str()).map(|u| *u.as_bytes()).map_err(|e| ArgoError::Schema(format!("session.id: {e}"))) }
pub(crate) fn session_id_from_blob(b: &[u8]) -> ArgoResult<SessionId> { Uuid::from_slice(b).map(|id| SessionId::from_string(id.to_string())).map_err(|e| ArgoError::Schema(format!("session.id: {e}"))) }
pub(crate) fn micros_to_utc(v: i64, field: &'static str) -> ArgoResult<chrono::DateTime<Utc>> { Utc.timestamp_micros(v).single().ok_or_else(|| ArgoError::Schema(format!("{field} out of range"))) }
pub(crate) fn opt_micros(v: Option<i64>, field: &'static str) -> ArgoResult<Option<chrono::DateTime<Utc>>> { v.map(|m| micros_to_utc(m, field)).transpose() }
pub(crate) fn to_json<T: serde::Serialize>(v: &T, label: &'static str) -> ArgoResult<String> { serde_json::to_string(v).map_err(|e| ArgoError::Schema(format!("{label}: {e}"))) }
pub(crate) fn from_json<T: serde::de::DeserializeOwned>(json: &str, label: &'static str) -> ArgoResult<T> { serde_json::from_str(json).map_err(|e| ArgoError::Schema(format!("{label}: {e}"))) }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EscalationStep, Handoff, HandoffStatus};
    use std::sync::Arc;

    async fn repo() -> OrchestratorStore { let store = Arc::new(SessionStore::open(":memory:").await.unwrap()); store.migrate().await.unwrap(); let repo = OrchestratorStore::new(store); repo.ensure_schema().await.unwrap(); repo.ensure_schema().await.unwrap(); repo }
    async fn inserted(repo: &OrchestratorStore, tenant: TenantId, title: &str) -> Session { let s = Session::new(tenant, title); repo.insert_session(&s).await.unwrap(); s }
    fn handoff(session: &Session, from: WorkerId, to: WorkerId) -> Handoff { Handoff::new(session.id.clone(), from, EscalationStep::Worker(to), "finish the customer handoff", serde_json::json!({"case": "aeon"})) }

    #[tokio::test]
    async fn round_trip_session_and_two_handoffs() {
        let repo = repo().await; let tenant = TenantId::new();
        let session = inserted(&repo, tenant, "renewal").await;
        let from = WorkerId::new(); let to = WorkerId::new();
        repo.insert_handoff(tenant, &handoff(&session, from, to)).await.unwrap();
        repo.insert_handoff(tenant, &handoff(&session, from, to)).await.unwrap();
        let loaded = repo.find_session(tenant, &session.id).await.unwrap().unwrap();
        let handoffs = repo
            .list_handoffs_for_session(tenant, &session.id)
            .await
            .unwrap();
        assert_eq!(loaded.status, SessionStatus::Active);
        assert_eq!(handoffs.len(), 2);
        assert!(handoffs.iter().all(|h| h.status == HandoffStatus::Pending));
    }

    #[tokio::test]
    async fn cross_tenant_isolation_for_active_sessions() {
        let repo = repo().await; let tenant_a = TenantId::new(); let tenant_b = TenantId::new();
        inserted(&repo, tenant_a, "a").await;
        assert_eq!(repo.list_active_sessions(tenant_b).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn add_participant_is_idempotent_and_persists() {
        let repo = repo().await; let tenant = TenantId::new();
        let session = inserted(&repo, tenant, "participants").await;
        let worker = WorkerId::new();
        repo.add_participant(tenant, &session.id, worker).await.unwrap();
        repo.add_participant(tenant, &session.id, worker).await.unwrap();
        let loaded = repo.find_session(tenant, &session.id).await.unwrap().unwrap();
        assert_eq!(loaded.participants, vec![worker]);
    }

    #[tokio::test]
    async fn session_status_rejects_terminal_backslide() {
        let repo = repo().await; let tenant = TenantId::new();
        let session = inserted(&repo, tenant, "status").await;
        repo.update_session_status(tenant, &session.id, SessionStatus::Resolved)
            .await
            .unwrap();
        assert!(repo
            .update_session_status(tenant, &session.id, SessionStatus::Active)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn handoff_status_rejects_settled_to_pending() {
        let repo = repo().await; let tenant = TenantId::new();
        let session = inserted(&repo, tenant, "handoff-status").await;
        let h = handoff(&session, WorkerId::new(), WorkerId::new());
        repo.insert_handoff(tenant, &h).await.unwrap();
        repo.update_handoff_status(tenant, &h.id, HandoffStatus::Settled)
            .await
            .unwrap();
        assert!(repo
            .update_handoff_status(tenant, &h.id, HandoffStatus::Pending)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn pending_handoffs_filter_by_worker() {
        let repo = repo().await; let tenant = TenantId::new();
        let session = inserted(&repo, tenant, "pending").await;
        let target = WorkerId::new(); let other = WorkerId::new();
        repo.insert_handoff(tenant, &handoff(&session, WorkerId::new(), target)).await.unwrap();
        repo.insert_handoff(tenant, &handoff(&session, WorkerId::new(), other)).await.unwrap();
        let pending = repo
            .list_pending_handoffs_for_worker(tenant, target)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);
        assert!(matches!(pending[0].to, EscalationStep::Worker(id) if id == target));
    }
}
