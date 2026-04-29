//! `argo-store` — SQLite + FTS5 session store with up/down migrations.
//!
//! See `docs/plan/argo-build-plan.md` §7.
//!
//! ```no_run
//! # async fn run() -> argo_types::ArgoResult<()> {
//! use argo_store::SessionStore;
//! let store = SessionStore::open(":memory:").await?;
//! store.migrate().await?;
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod migrations;
mod models;

pub use models::SessionHeader;

use argo_types::{ArgoError, MessageRole, SessionId, TenantId, WorkspaceId};
use chrono::Utc;
use models::SessionHeaderRow;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

/// Result alias.
pub type StoreResult<T> = Result<T, ArgoError>;

/// A SQLite-backed session store.
///
/// One DB per tenant. WAL mode. Migrations applied on `migrate()`.
pub struct SessionStore {
    pool: Pool<Sqlite>,
}

impl SessionStore {
    /// Open a store at `path`. Creates the file if it doesn't exist.
    /// Use `":memory:"` for in-memory (tests).
    pub async fn open(path: impl AsRef<Path>) -> StoreResult<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let opts = SqliteConnectOptions::from_str(&path_str)
            .map_err(|e| ArgoError::Provider(format!("sqlite open: {e}")))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await
            .map_err(|e| ArgoError::Provider(format!("sqlite pool: {e}")))?;

        Ok(Self { pool })
    }

    /// Apply all pending migrations.
    pub async fn migrate(&self) -> StoreResult<()> {
        migrations::run(&self.pool).await
    }

    /// Roll the schema back to a specific version (test/dev only).
    pub async fn migrate_down_to(&self, version: u32) -> StoreResult<()> {
        migrations::roll_back_to(&self.pool, version).await
    }

    /// Current schema version (0 if uninitialized).
    pub async fn schema_version(&self) -> StoreResult<u32> {
        migrations::current_version(&self.pool).await
    }

    /// Create a new session under a tenant + workspace.
    pub async fn create_session(
        &self,
        tenant_id: TenantId,
        workspace_id: WorkspaceId,
        model: &str,
        provider: &str,
        title: Option<&str>,
    ) -> StoreResult<SessionId> {
        let id = SessionId::new();
        let started_at = Utc::now().timestamp_micros();
        sqlx::query(
            "INSERT INTO session (id, tenant_id, workspace_id, model, provider, title, started_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.0.as_bytes().as_slice())
        .bind(tenant_id.0.as_bytes().as_slice())
        .bind(workspace_id.0.as_bytes().as_slice())
        .bind(model)
        .bind(provider)
        .bind(title)
        .bind(started_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("create_session: {e}")))?;
        Ok(id)
    }

    /// Append a message to a session, auto-assigning sequence number.
    pub async fn append_message(
        &self,
        session_id: SessionId,
        role: MessageRole,
        content_json: &str,
    ) -> StoreResult<Uuid> {
        let id = Uuid::now_v7();
        let created_at = Utc::now().timestamp_micros();
        let role_str = match role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ArgoError::Provider(format!("tx begin: {e}")))?;

        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM message WHERE session_id = ?",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("next_seq: {e}")))?;

        sqlx::query(
            "INSERT INTO message (id, session_id, role, content_json, created_at, seq)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(session_id.0.as_bytes().as_slice())
        .bind(role_str)
        .bind(content_json)
        .bind(created_at)
        .bind(next_seq)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("insert message: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| ArgoError::Provider(format!("tx commit: {e}")))?;
        Ok(id)
    }

    /// Fetch a session header.
    pub async fn get_session(&self, session_id: SessionId) -> StoreResult<Option<SessionHeader>> {
        let row = sqlx::query_as::<_, SessionHeaderRow>(
            "SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at
             FROM session WHERE id = ?",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("get_session: {e}")))?;

        Ok(row.map(SessionHeader::from))
    }

    /// List sessions for a tenant, ordered by recency.
    pub async fn list_sessions(
        &self,
        tenant_id: TenantId,
        limit: u32,
    ) -> StoreResult<Vec<SessionHeader>> {
        let rows = sqlx::query_as::<_, SessionHeaderRow>(
            "SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at
             FROM session WHERE tenant_id = ?
             ORDER BY started_at DESC LIMIT ?",
        )
        .bind(tenant_id.0.as_bytes().as_slice())
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_sessions: {e}")))?;

        Ok(rows.into_iter().map(SessionHeader::from).collect())
    }

    /// Count messages in a session.
    pub async fn message_count(&self, session_id: SessionId) -> StoreResult<u64> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM message WHERE session_id = ?")
            .bind(session_id.0.as_bytes().as_slice())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("message_count: {e}")))?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// Direct access to the underlying pool (for crates that need raw SQL).
    #[must_use]
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn open_and_migrate() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        assert_eq!(store.schema_version().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn migrate_round_trip() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store.migrate_down_to(0).await.unwrap();
        assert_eq!(store.schema_version().await.unwrap(), 0);
        store.migrate().await.unwrap();
        assert_eq!(store.schema_version().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn create_session_and_append_messages() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        let tenant = TenantId::new();
        let ws = WorkspaceId::new();
        sqlx::query(
            "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
             VALUES (?, ?, 'test', '.', strftime('%s','now'))",
        )
        .bind(ws.0.as_bytes().as_slice())
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();

        let session = store
            .create_session(tenant, ws, "claude-haiku", "anthropic", Some("test"))
            .await
            .unwrap();
        store
            .append_message(
                session,
                MessageRole::User,
                r#"[{"type":"text","text":"hi"}]"#,
            )
            .await
            .unwrap();
        store
            .append_message(
                session,
                MessageRole::Assistant,
                r#"[{"type":"text","text":"hello"}]"#,
            )
            .await
            .unwrap();
        assert_eq!(store.message_count(session).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn list_sessions_recency_order() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        let tenant = TenantId::new();
        let ws = WorkspaceId::new();
        sqlx::query(
            "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
             VALUES (?, ?, 'test', '.', strftime('%s','now'))",
        )
        .bind(ws.0.as_bytes().as_slice())
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();
        for i in 0..3 {
            store
                .create_session(tenant, ws, "model", "provider", Some(&format!("s{i}")))
                .await
                .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }
        let list = store.list_sessions(tenant, 10).await.unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].title.as_deref(), Some("s2"));
    }

    #[tokio::test]
    async fn fts5_table_exists() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='message_fts'",
        )
        .fetch_one(store.pool())
        .await
        .unwrap();
        assert_eq!(n, 1);
    }
}
