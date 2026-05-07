//! `argo-store` — SQLite + FTS5 session store with up/down migrations.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod anomaly;
/// Data classification tags (Stream `claude-data-classification`).
pub mod classification;
/// Operator feedback (Stream `claude-operator-feedback-channel`).
pub mod feedback;
mod forge_meta;
/// GDPR erasure helpers.
pub mod gdpr;
mod grants;
mod marketplace_meta;
mod message_query;
mod migrations;
mod models;
mod session_meta;
mod session_query;
mod spend;
/// Spend anomaly detector — Welford rolling stats per (worker,
/// hour-of-day) + z-score check (Stream
/// `claude-spend-anomaly-alerts`).
pub mod spend_anomaly;
mod stats;
/// Per-tenant resource quotas for budget and auto-approval gates.
pub mod tenant_quota;
/// Workflow checkpoint helpers (Stream `claude-checkpoint-resume`).
pub mod workflow_checkpoint;
mod workspace;

pub use grants::SessionGrant;
pub use marketplace_meta::MarketplaceArtifactRow;
pub use models::{AuditRow, ForgeRunRow, JudgeRow, MessageRow, MessageSearchHit, SessionHeader};
pub use spend::GlobalSpend;
pub use stats::{tool_errors_histogram, DbStats};

use argo_types::{ArgoError, SessionId, TenantId, WorkspaceId};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;

/// Result alias.
pub type StoreResult<T> = Result<T, ArgoError>;

/// A SQLite-backed session store.
///
/// One DB per tenant. WAL mode. Migrations applied on `migrate()`.
pub struct SessionStore {
    pub(crate) pool: Pool<Sqlite>,
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
        session_meta::delete_session(&self.pool, sid)
            .await
            .map_err(|e| ArgoError::Provider(format!("delete_session: {e}")))
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

    /// Cross-tenant spend rollup — `(today, this_month, all_time)`
    /// totals in USD micros, all relative to `Utc::now()`. Used by
    /// `argo costs show` and the TUI cost overlay so we don't
    /// replicate the SQL in the caller.
    pub async fn load_global_spend(&self) -> StoreResult<spend::GlobalSpend> {
        spend::load_global_spend(&self.pool, chrono::Utc::now()).await
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
