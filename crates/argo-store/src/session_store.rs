//! [`SessionStore`] struct + its inherent impl block. Split out from
//! `lib.rs` under the 500-LOC per-file ceiling. The public surface is
//! preserved by a `pub use session_store::SessionStore;` re-export
//! from the crate root, so external callers continue to import
//! `argo_store::SessionStore`.
//!
//! Pure shape change — no behavior change.

use std::path::Path;
use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use argo_runtime::prompt_cache_affinity::PromptCacheAffinity;
use argo_types::{ArgoError, SessionId, TenantId, WorkspaceId};

use crate::batched_writer::BatchedWriter;
use crate::message_query::MessageItem;
use crate::{
    encryption, migrations, session_meta, spend, stats, store_open, wal_tuning, workspace,
    StoreResult,
};

/// A SQLite-backed session store.
///
/// One DB per tenant. WAL mode. Migrations applied on `migrate()`.
///
/// ```
/// # async fn doc() -> argo_store::StoreResult<()> {
/// use argo_store::SessionStore;
/// let store = SessionStore::open(":memory:").await?;
/// store.migrate().await?;
/// assert!(store.schema_version().await? > 0);
/// # Ok(()) }
/// ```
pub struct SessionStore {
    pub(crate) pool: Pool<Sqlite>,
    pub(crate) messages: Arc<BatchedWriter<MessageItem>>,
    pub(crate) prompt_cache_affinity: Option<Arc<PromptCacheAffinity>>,
}

impl SessionStore {
    /// Open a store at `path`. Creates the file if it doesn't exist.
    /// Use `":memory:"` for in-memory (tests).
    ///
    /// ```
    /// # async fn doc() -> argo_store::StoreResult<()> {
    /// let store = argo_store::SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// # Ok(()) }
    /// ```
    pub async fn open(path: impl AsRef<Path>) -> StoreResult<Self> {
        Self::open_with_profile(path, wal_tuning::WalTuningProfile::default()).await
    }

    /// Variant of [`SessionStore::open`] that takes an explicit
    /// [`wal_tuning::WalTuningProfile`]. Used by `argo-cli` so the
    /// `[store.wal_tuning].profile` knob from `argo.toml` flows
    /// through to the pool open.
    ///
    /// ```
    /// # async fn doc() -> argo_store::StoreResult<()> {
    /// use argo_store::{SessionStore, wal_tuning::WalTuningProfile};
    /// let store = SessionStore::open_with_profile(":memory:", WalTuningProfile::Default).await?;
    /// store.migrate().await?;
    /// # Ok(()) }
    /// ```
    pub async fn open_with_profile(
        path: impl AsRef<Path>,
        profile: wal_tuning::WalTuningProfile,
    ) -> StoreResult<Self> {
        store_open::open_with_profile(path, profile).await
    }

    /// Open with `PRAGMA key` applied from the supplied
    /// [`encryption::KeySource`] before any other SQL runs. When the
    /// key source resolves to `None` this behaves exactly like
    /// [`SessionStore::open_with_profile`].
    ///
    /// ```no_run
    /// # async fn run() -> argo_store::StoreResult<()> {
    /// use argo_store::{SessionStore, wal_tuning::WalTuningProfile};
    /// use argo_store::encryption::KeySource;
    /// let key_source = KeySource::parse("env:ARGO_DB_ENCRYPTION_KEY")?;
    /// let store = SessionStore::open_with_encryption(
    ///     ":memory:", WalTuningProfile::Default, &key_source,
    /// ).await?;
    /// store.migrate().await?;
    /// # Ok(()) }
    /// ```
    pub async fn open_with_encryption(
        path: impl AsRef<Path>,
        profile: wal_tuning::WalTuningProfile,
        key_source: &encryption::KeySource,
    ) -> StoreResult<Self> {
        store_open::open_with_encryption(path, profile, key_source).await
    }

    /// Apply all pending migrations.
    pub async fn migrate(&self) -> StoreResult<()> {
        migrations::run(&self.pool).await?;
        store_open::ensure_session_worker_id_column(&self.pool).await?;
        Ok(())
    }

    /// Roll the schema back to a specific version (test/dev only).
    pub async fn migrate_down_to(&self, version: u32) -> StoreResult<()> {
        migrations::roll_back_to(&self.pool, version).await
    }

    /// Current schema version (0 if uninitialized).
    pub async fn schema_version(&self) -> StoreResult<u32> {
        migrations::current_version(&self.pool).await
    }

    /// Strictly compare on-disk schema to [`crate::EXPECTED_SCHEMA_VERSION`].
    /// Returns `Ok(())` when they match exactly; otherwise returns a
    /// directional `ArgoError::Provider` distinguishing "schema is
    /// behind the binary" (ran an older `argo db migrate`?) from
    /// "schema is ahead of the binary" (binary needs rolling back).
    /// Used by the gateway startup health check.
    pub async fn verify_schema_version(&self) -> StoreResult<()> {
        store_open::verify_schema_version(self.schema_version().await?)
    }

    /// Cleanly tear the pool down so SQLite can checkpoint the WAL,
    /// release the lock file, and close every cached connection.
    /// Idempotent — re-calling `close()` after the pool has shut
    /// down is a no-op.
    ///
    /// Used by the gateway's graceful-shutdown path so a SIGTERM
    /// during a rolling deploy doesn't leak unflushed WAL frames or
    /// leave a stale `*.db-shm` lock that blocks the next process.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run() -> argo_store::StoreResult<()> {
    /// let store = argo_store::SessionStore::open(":memory:").await?;
    /// store.migrate().await?;
    /// store.close().await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close(&self) {
        // Drain in-flight batched writes before tearing the pool
        // down — otherwise pending message rows would be lost.
        // `flush_now` is a no-op when the buffer is empty.
        let _ = self.messages.flush_now().await;
        self.pool.close().await;
    }

    /// Force any in-flight batched writes (messages, audit) to flush
    /// to disk. Used by tests that read a row immediately after
    /// submitting it without waiting for the latency window, and by
    /// the gateway's graceful-shutdown hook ahead of `close()`.
    pub async fn flush_pending_writes(&self) -> StoreResult<()> {
        self.messages.flush_now().await
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
    pub async fn db_stats(&self) -> StoreResult<stats::DbStats> {
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
