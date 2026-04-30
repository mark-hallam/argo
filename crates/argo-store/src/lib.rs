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

mod forge_meta;
mod marketplace_meta;
mod migrations;
mod models;
mod session_meta;
mod spend;
mod stats;
mod workspace;

pub use marketplace_meta::MarketplaceArtifactRow;
pub use models::{AuditRow, ForgeRunRow, JudgeRow, MessageRow, MessageSearchHit, SessionHeader};
pub use stats::DbStats;

use argo_types::{ArgoError, MessageRole, RoutingDecision, SessionId, TenantId, WorkspaceId};
use chrono::Utc;
use models::{MessageSearchHitRow, RoutingDecisionRow, SessionHeaderRow};
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

    /// First workspace registered in the DB (by `created_at`),
    /// if any. The CLI's read-only commands fall through to this
    /// when the user hasn't configured a tenant explicitly — the
    /// daemon binding will replace it with proper tenant + workspace
    /// resolution.
    pub async fn first_workspace(&self) -> StoreResult<Option<WorkspaceId>> {
        let row: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT id FROM workspace ORDER BY created_at ASC LIMIT 1")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("first_workspace: {e}")))?;
        Ok(row.map(|b| WorkspaceId(models::uuid_from_blob(&b))))
    }

    /// List sessions across all tenants, ordered by recency. Used
    /// by the CLI's `argo session list` when no tenant filter is
    /// provided.
    pub async fn list_all_sessions(&self, limit: u32) -> StoreResult<Vec<SessionHeader>> {
        let rows = sqlx::query_as::<_, SessionHeaderRow>(
            "SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at
             FROM session ORDER BY started_at DESC LIMIT ?",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_all_sessions: {e}")))?;
        Ok(rows.into_iter().map(SessionHeader::from).collect())
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

    /// Persist a [`RoutingDecision`] (Amendment 1, §24.4 acceptance: "all
    /// non-Fixed decisions persisted in `routing_decision`"). The cost
    /// router calls this for every turn when policy ≠ Fixed.
    pub async fn persist_routing_decision(&self, d: &RoutingDecision) -> StoreResult<()> {
        let decided_at = d.classified_at.timestamp_micros();
        sqlx::query(
            "INSERT INTO routing_decision (id, session_id, turn, policy, tier, \
             classifier_latency_ms, chosen_provider, chosen_model, reason, decided_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(d.id.0.as_bytes().as_slice())
        .bind(d.session_id.0.as_bytes().as_slice())
        .bind(i64::from(d.turn))
        .bind(&d.policy)
        .bind(&d.tier)
        .bind(i64::try_from(d.classifier_latency_ms).unwrap_or(i64::MAX))
        .bind(&d.chosen_provider_model.provider)
        .bind(&d.chosen_provider_model.model)
        .bind(&d.reason)
        .bind(decided_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("persist_routing_decision: {e}")))?;
        Ok(())
    }

    /// Delete every session whose `started_at` is older than
    /// `cutoff_micros`. Cascades through `message`, `tool_call`,
    /// `audit_record`, `routing_decision`, and `judge_record` via
    /// SQLite ON DELETE CASCADE. Returns the number of session
    /// rows deleted.
    pub async fn prune_sessions_before(&self, cutoff_micros: i64) -> StoreResult<u64> {
        let result = sqlx::query("DELETE FROM session WHERE started_at < ?")
            .bind(cutoff_micros)
            .execute(&self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("prune_sessions_before: {e}")))?;
        Ok(result.rows_affected())
    }

    /// Count sessions whose `started_at` is older than
    /// `cutoff_micros`. Used by `argo db prune --dry-run`.
    pub async fn count_sessions_before(&self, cutoff_micros: i64) -> StoreResult<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM session WHERE started_at < ?")
            .bind(cutoff_micros)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("count_sessions_before: {e}")))
    }

    /// Run `VACUUM` to reclaim SQLite-page space (typically after a
    /// large prune). Wallclock cost scales with DB size; the
    /// operation rebuilds the file in-place. Returns when the
    /// VACUUM completes.
    pub async fn vacuum(&self) -> StoreResult<()> {
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("vacuum: {e}")))?;
        Ok(())
    }

    /// Aggregate row counts across the DB — used by
    /// `argo doctor` / `argo db status` to summarize the workspace.
    pub async fn db_stats(&self) -> StoreResult<DbStats> {
        stats::db_stats(&self.pool).await
    }

    /// Delete a session and all its dependent rows. SQLite's
    /// ON DELETE CASCADE on `message`, `tool_call`, `audit_record`,
    /// `routing_decision`, and `judge_record` cleans up the
    /// children automatically; this method just deletes the parent
    /// `session` row. Returns the number of rows deleted (0 if the
    /// session was not found).
    pub async fn delete_session(&self, sid: SessionId) -> StoreResult<u64> {
        session_meta::delete_session(&self.pool, sid).await
            .map_err(|e| ArgoError::Provider(format!("delete_session: {e}")))
    }

    /// List judge records for a session, oldest first. Used by
    /// `argo judge inspect <session_id>`.
    pub async fn list_judge_records(&self, session_id: SessionId) -> StoreResult<Vec<JudgeRow>> {
        let rows = sqlx::query_as::<_, JudgeRow>(
            "SELECT subject_kind, subject_ref, verdict, reason, judge_provider, \
             judge_model, rubric_id, rubric_version, created_at \
             FROM judge_record WHERE session_id = ? \
             ORDER BY created_at ASC",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_judge_records: {e}")))?;
        Ok(rows)
    }

    /// Stream a session's audit-log rows in `seq` order. Used by
    /// `argo trace --session <id>` to render the per-session
    /// trace timeline.
    pub async fn list_audit_for_session(
        &self,
        session_id: SessionId,
    ) -> StoreResult<Vec<AuditRow>> {
        let rows = sqlx::query_as::<_, AuditRow>(
            "SELECT seq, kind, payload_json, created_at FROM audit_record \
             WHERE session_id = ? ORDER BY seq ASC",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_audit_for_session: {e}")))?;
        Ok(rows)
    }

    /// Full-text search the message log via FTS5. Returns the
    /// per-row `(session_id, seq, role, snippet)` for the top
    /// `limit` matches across all sessions, ordered by relevance.
    /// Used by `argo session search`.
    pub async fn search_messages(
        &self,
        query: &str,
        limit: u32,
    ) -> StoreResult<Vec<MessageSearchHit>> {
        let rows: Vec<MessageSearchHitRow> = sqlx::query_as::<_, MessageSearchHitRow>(
            "SELECT m.session_id, m.seq, m.role, \
             snippet(message_fts, 0, '[', ']', '…', 32) AS snippet \
             FROM message_fts JOIN message m ON m.rowid = message_fts.rowid \
             WHERE message_fts MATCH ? ORDER BY rank LIMIT ?",
        )
        .bind(query)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("search_messages: {e}")))?;
        Ok(rows.into_iter().map(MessageSearchHit::from).collect())
    }

    /// FTS5 search scoped to a single session. Same shape as
    /// [`Self::search_messages`] but adds a `session_id` filter.
    pub async fn search_messages_in_session(
        &self,
        session_id: SessionId,
        query: &str,
        limit: u32,
    ) -> StoreResult<Vec<MessageSearchHit>> {
        let rows: Vec<MessageSearchHitRow> = sqlx::query_as::<_, MessageSearchHitRow>(
            "SELECT m.session_id, m.seq, m.role, \
             snippet(message_fts, 0, '[', ']', '…', 32) AS snippet \
             FROM message_fts JOIN message m ON m.rowid = message_fts.rowid \
             WHERE message_fts MATCH ? AND m.session_id = ? \
             ORDER BY rank LIMIT ?",
        )
        .bind(query)
        .bind(session_id.0.as_bytes().as_slice())
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("search_messages_in_session: {e}")))?;
        Ok(rows.into_iter().map(MessageSearchHit::from).collect())
    }

    /// Stream a session's messages in `seq` order. Used by
    /// `argo session export` to dump sessions to JSONL.
    pub async fn list_messages(&self, session_id: SessionId) -> StoreResult<Vec<MessageRow>> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT id, session_id, role, content_json, tool_calls_json, \
             tool_call_id, reasoning, provider_meta, created_at, seq \
             FROM message WHERE session_id = ? ORDER BY seq ASC",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_messages: {e}")))?;
        Ok(rows)
    }

    /// List recent Forge runs (most-recent first). Used by
    /// `argo forge runs list` to surface optimization history.
    pub async fn list_forge_runs(&self, limit: u32) -> StoreResult<Vec<ForgeRunRow>> {
        let rows = sqlx::query_as::<_, ForgeRunRow>(
            "SELECT id, skill_name, started_at, ended_at, base_version, \
             candidate_version, iterations, spend_usd_micros, \
             final_score, status FROM forge_run \
             ORDER BY started_at DESC LIMIT ?",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_forge_runs: {e}")))?;
        Ok(rows)
    }

    /// List routing decisions for `session_id` in turn order.
    /// Used by `argo route inspect` to surface the cost-router's
    /// per-turn picks (Amendment 1, §24.4).
    pub async fn list_routing_decisions(
        &self,
        session_id: SessionId,
    ) -> StoreResult<Vec<RoutingDecision>> {
        let rows: Vec<RoutingDecisionRow> = sqlx::query_as::<_, RoutingDecisionRow>(
            "SELECT id, session_id, turn, policy, tier, classifier_latency_ms, \
             chosen_provider, chosen_model, reason, decided_at \
             FROM routing_decision WHERE session_id = ? \
             ORDER BY turn ASC",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_routing_decisions: {e}")))?;
        Ok(rows.into_iter().map(RoutingDecision::from).collect())
    }

    /// Direct access to the underlying pool (for crates that need raw SQL).
    #[must_use]
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Append a spend record to `spend_ledger` (item C — hard $$$
    /// cost ceilings). Idempotent against `(tenant, session,
    /// charged_at_micros)` is **not** enforced; callers should pass
    /// the timestamp + amount they actually charged.
    pub async fn record_spend(
        &self,
        tenant_id: TenantId,
        session_id: SessionId,
        charged_at_micros: i64,
        amount_usd_micros: u64,
        provider: &str,
        model: &str,
    ) -> StoreResult<()> {
        spend::record_spend(
            &self.pool,
            tenant_id,
            session_id,
            charged_at_micros,
            amount_usd_micros,
            provider,
            model,
        )
        .await
    }

    /// Load the per-`(tenant, session)` spend counters as of `now`.
    /// Returns `(session_used, day_used, month_used)` in USD micros,
    /// suitable for [`argo_types::SpendCounters`].
    pub async fn load_spend_counters(
        &self,
        tenant_id: TenantId,
        session_id: SessionId,
        now: chrono::DateTime<chrono::Utc>,
    ) -> StoreResult<argo_types::SpendCounters> {
        spend::load_spend_counters(&self.pool, tenant_id, session_id, now).await
    }

    /// Insert a tenant + workspace pair (or reuse the existing one if
    /// `root_path` matches). Used by `argo workspace init` to bootstrap
    /// the row that `MemoryStore`/`SkillStore` writes anchor to.
    pub async fn ensure_workspace(
        &self,
        name: &str,
        root_path: &str,
    ) -> StoreResult<(TenantId, WorkspaceId)> {
        workspace::ensure_workspace(&self.pool, name, root_path).await
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
