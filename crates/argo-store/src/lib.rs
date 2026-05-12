//! `argo-store` — SQLite + FTS5 session store with up/down migrations.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod anomaly;
mod anomaly_persistence;
/// First-class approval requests for the auto-approval gate
/// (Stream `claude-approval-request-store`). Records pending grants,
/// who can decide them, and a deadline + `default_decision` the
/// downstream timeout sweep promotes stale rows by.
pub mod approvals;
/// Typed scaffold for blob storage — `AttachedBlob`, `BlobRef`,
/// `BlobStore` trait, and an in-memory test impl. No persistence yet
/// (Stream `claude-store-attached-blob-layer`); downstream
/// `claude-cu-screenshot-storage` lands the SQLite-backed impl.
pub mod attached_blob;
/// Budget rollups over skills and spend.
pub mod budget;
/// Data classification tags (Stream `claude-data-classification`).
pub mod classification;
/// Operator feedback (Stream `claude-operator-feedback-channel`).
pub mod feedback;
mod forge_meta;
/// GDPR erasure helpers.
pub mod gdpr;
/// Aggregate query helpers over the `handoff` table for the
/// `argo handoff-stats` CLI surface (Stream `codex-handoff-stats-cli`).
pub mod handoff_stats_query;
mod gdpr_apply;
mod grants;
mod marketplace_meta;
mod message_query;
mod migrations;
mod models;
mod session_meta;
mod session_query;
/// Chronological skill invocation history query helpers.
pub mod skill_runs_query;
mod skill_runs_query_decode;
/// Skill discovery query helpers — vertical-scoped search and
/// session-attachment indirection (Stream
/// `claude-skill-search-by-vertical`).
pub mod skill_query;
/// Cross-tenant skill sharing query helpers (Stream
/// `claude-skill-share-cross-tenant-poc`, mission claim 4).
pub mod skill_share;
/// Operator-to-publisher skill-rating store (Stream
/// `mission-skill-ratings-publisher`, mission claim 4 #80) — one row
/// per `(skill_id, skill_version, rater_tenant)`, idempotent on the
/// composite primary key.
pub mod skill_rating;
/// Per-skill operator flags (Stream
/// `mission-forge-operator-feedback-eval-signal`, MISSION_BACKLOG
/// Section 3 Claim 4 #85). Generic flag store consulted by the
/// Forge ingestion path before pulling operator feedback as eval
/// cases — `no-forge-feedback` toggles ingestion off for noisy
/// skills.
pub mod skill_flag;
mod spend;
/// Spend anomaly detector — Welford rolling stats per (worker,
/// hour-of-day) + z-score check (Stream
/// `claude-spend-anomaly-alerts`).
pub mod spend_anomaly;
/// Hard `[budget.ceiling]` enforcement read side —
/// `CeilingConfig`, `CeilingCheck`, and the
/// `SessionStore::check_ceiling` query that feeds the
/// `argo-provider::spend_gate::SpendGate` enforcement-time wrapper
/// (Stream `mission-llm-budget-hard-ceiling`, MISSION_BACKLOG
/// Section 7 #225).
pub mod spend_ceiling;
mod stats;
/// Per-executor-kind action latency histogram — power-of-2 ms
/// buckets, p50/p95/p99 readout (Stream
/// `claude-latency-percentiles-tracking`).
pub mod latency_tracker;
/// Persistent learned-plan-template store — Dreams-distilled
/// workflow YAML body + TrustGrade ladder (Stream
/// `claude-dreams-plan-templates`).
pub mod learned_plan_template;
/// Typed query helpers backed by lazy raw-SQL schemas
/// (Stream `codex-mcp-server-registry-cli`).
pub mod queries;
/// Per-worker persona revision log — append-only diff history with
/// sha256 audit-chain hashes + one-step revert (Stream
/// `mission-persona-evolution-audit-revert`, MISSION_BACKLOG Section
/// 3 Claim 1 #46).
pub mod persona_revision;
/// Recovery-cache CRUD helpers — `argo recover cache list/show/clear`.
pub mod recovery_cache;
/// Persistent subtask ledger — durable record of Planner-emitted
/// decompositions (Stream `claude-subtask-ledger-store`).
pub mod subtask;
/// Per-subtask budget tree — declared cap, accumulated spend, and a
/// recursive walk_tree CTE that rolls aggregated spend up to the
/// root. New sibling functions only; the original `subtask` module
/// is unchanged (Stream `mission-delegation-budget-tree`,
/// MISSION_BACKLOG Section 3 Claim 2 #57).
pub mod subtask_budget;
/// Operator-facing team primitive — bridges `org_unit` + `roster`
/// + `roster_member` for the `argo team` CLI surface (Stream
/// `claude-worker-team-roster-cli`).
pub mod team_roster;
/// Mutation surface for the team-roster primitive (split from
/// `team_roster.rs` for the 500-LOC budget). Public symbols are
/// re-exported from `team_roster` so external imports remain
/// `argo_store::team_roster::create_team`.
pub mod team_roster_mutations;
/// Elevated-view grant store (Stream J16) — per-principal,
/// per-category, time-bounded "show me the raw value" override
/// consulted by the J2 redact-policy resolver. Rows are inserted by
/// the `POST /api/v1/audit/elevated-view-request` handler and revoked
/// by `DELETE /api/v1/audit/elevated-view-grants/<id>`.
pub mod elevated_view_grant;
/// Per-tenant resource quotas for budget and auto-approval gates.
pub mod tenant_quota;
/// Quota kind enums + error type for the tenant_quota gate
/// (split from `tenant_quota.rs` for the 500-LOC budget).
pub mod tenant_quota_kinds;
/// Usage accounting + headroom-check methods for `TenantQuotaStore`
/// (split from `tenant_quota.rs` for the 500-LOC budget).
pub mod tenant_quota_usage;
/// sqlx connection-pool tuning knobs (env-driven).
pub mod pool_config;
/// Opinionated SQLite WAL tuning profiles
/// (Stream `mission-sqlite-wal-tuning`).
pub mod wal_tuning;
/// Worker archive (sleep / wake) — frozen JSON snapshot store
/// (Stream `claude-worker-archive-rehydrate`).
pub mod worker_archive;
/// Worker dependency graph — declared inter-worker dependencies with
/// per-kind cycle detection and downstream impact closure
/// (Stream `claude-worker-dependency-tracking`).
pub mod worker_deps;
/// Per-worker KPI time-series store — append-only observations
/// recorded against KPI names from the worker's charter
/// (Stream `claude-worker-kpi-tracker`, mission claim 2).
pub mod worker_kpi;
/// Per-worker quarterly OKRs — objectives + weighted key-results
/// scored against `worker_kpi_value`
/// (Stream `claude-worker-okr-cli`, mission claim 2 worker-goals layer).
pub mod worker_okr;
/// Data types backing [`worker_okr`] — split out under the 500-LOC
/// per-file ceiling. Re-exported through `worker_okr::*`; reach for
/// that path from outside the crate.
pub(crate) mod worker_okr_types;
/// Per-worker spend cap write helpers — `argo worker budget set/show`
/// (Stream `claude-worker-budget-cli`). Caps live on the worker's
/// `auto_approval_json` column; the spend totals come from the
/// existing `worker_budget` ledger.
pub mod worker_budget_write;
/// Worker presence (`last_active_at`) and fleet listing helpers
/// (Stream `claude-worker-presence-tracking`).
pub mod worker_presence;
/// Per-tenant gateway rate-limit overrides (Stream
/// `mission-per-tenant-rate-limiting`, MISSION_BACKLOG Section 1
/// #21). Stores one row per tenant that supersedes the gateway's
/// config-driven `requests_per_minute` default.
pub mod tenant_rate_limit;
/// Per-operator risk-level overrides for the action classification
/// pipeline (Stream `mission-operator-specific-risk-overrides`,
/// MISSION_BACKLOG Section 3 Claim 6 #122). Stores one row per
/// `(operator_id, action_kind, vertical_id)` triple. Missing row →
/// caller falls back to the manifest-driven default.
pub mod operator_risk_overrides;
/// Tenant inspection and id-rename helpers.
pub mod tenant_query;
/// `show_tenant` + `rename_tenant` family — split out from
/// `tenant_query.rs` for the 500-LOC ceiling. Public symbols are
/// re-exported through `tenant_query`; external callers keep
/// importing from `argo_store::tenant_query::*`.
pub(crate) mod tenant_query_inspect;
/// Append-only cross-tenant trust observation log (Stream
/// `mission-cross-tenant-trust-graph`, MISSION_BACKLOG Section 3
/// Claim 4 #81). Each row is one quality observation made by a
/// consumer tenant against a skill shipped by a source tenant; the
/// in-memory `argo_skill::trust_graph::TrustGraph` consumes a Vec
/// of rows and materialises pairwise weights.
pub mod trust_observation;
/// Workflow checkpoint helpers (Stream `claude-checkpoint-resume`).
pub mod workflow_checkpoint;
mod workspace;

pub use grants::SessionGrant;
pub use recovery_cache::RecoveryCacheEntry;
pub use marketplace_meta::MarketplaceArtifactRow;
pub use models::{
    AuditRow, ForgeRunRow, JudgeRow, MessageRow, MessageSearchHit, SessionHeader, WorkerId,
};
pub use session_query::SessionCreate;
pub use skill_runs_query::{runs_for_skill, runs_for_tenant, SkillRunRow};
pub use spend::GlobalSpend;
pub use stats::{tool_errors_histogram, DbStats};

use argo_types::{ArgoError, SessionId, TenantId, WorkspaceId};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;

/// Result alias.
pub type StoreResult<T> = Result<T, ArgoError>;

/// Highest migration version this binary knows how to apply. The
/// gateway startup health check (`claude-startup-health-check`)
/// compares this to the on-disk `schema_version` and refuses to bind
/// when the two diverge. Bump this whenever a new `vNN` migration is
/// added.
pub const EXPECTED_SCHEMA_VERSION: u32 = 59;

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
        let path_str = path.as_ref().to_string_lossy().to_string();
        let opts = SqliteConnectOptions::from_str(&path_str)
            .map_err(|e| ArgoError::Provider(format!("sqlite open: {e}")))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool_cfg = pool_config::PoolConfig::from_env();
        let mut builder = SqlitePoolOptions::new()
            .max_connections(pool_cfg.max_connections)
            .min_connections(pool_cfg.min_connections)
            .acquire_timeout(pool_cfg.acquire_timeout);
        if let Some(idle) = pool_cfg.idle_timeout {
            builder = builder.idle_timeout(Some(idle));
        }
        let pool = builder
            .connect_with(opts)
            .await
            .map_err(|e| ArgoError::Provider(format!("sqlite pool: {e}")))?;

        wal_tuning::apply_pragmas(&pool, profile).await?;
        Ok(Self { pool })
    }

    /// Apply all pending migrations.
    pub async fn migrate(&self) -> StoreResult<()> {
        migrations::run(&self.pool).await?;
        ensure_session_worker_id_column(&self.pool).await?;
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

    /// Strictly compare on-disk schema to [`EXPECTED_SCHEMA_VERSION`].
    /// Returns `Ok(())` when they match exactly; otherwise returns a
    /// directional `ArgoError::Provider` distinguishing "schema is
    /// behind the binary" (ran an older `argo db migrate`?) from
    /// "schema is ahead of the binary" (binary needs rolling back).
    /// Used by the gateway startup health check.
    pub async fn verify_schema_version(&self) -> StoreResult<()> {
        let actual = self.schema_version().await?;
        match actual.cmp(&EXPECTED_SCHEMA_VERSION) {
            std::cmp::Ordering::Equal => Ok(()),
            std::cmp::Ordering::Less => Err(ArgoError::Provider(format!(
                "schema_version={actual} but binary expects {EXPECTED_SCHEMA_VERSION}; \
                 schema is behind the binary — run `argo db migrate`"
            ))),
            std::cmp::Ordering::Greater => Err(ArgoError::Provider(format!(
                "schema_version={actual} but binary expects {EXPECTED_SCHEMA_VERSION}; \
                 schema is ahead of the binary — binary is too old, roll back"
            ))),
        }
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
        self.pool.close().await;
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

/// PRAGMA-detect-then-ALTER for `session.worker_id` (Stream
/// `claude-session-worker-id-schema`). Mirrors the
/// `ensure_role_columns` / `ensure_worker_charter_columns` pattern
/// in argo-identity: the structured migration runner stops at v11,
/// later additive columns are applied idempotently from a sibling
/// SQL file (`migrations/38__session_worker_id.sql`).
async fn ensure_session_worker_id_column(pool: &Pool<Sqlite>) -> StoreResult<()> {
    let rows = sqlx::query("PRAGMA table_info(session)")
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("session table info: {e}")))?;
    let has_column = rows.iter().any(|row| {
        use sqlx::Row;
        let name: String = row.get("name");
        name == "worker_id"
    });
    if !has_column {
        sqlx::query("ALTER TABLE session ADD COLUMN worker_id BLOB")
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("add session.worker_id: {e}")))?;
    }
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_session_worker \
         ON session(worker_id) WHERE worker_id IS NOT NULL",
    )
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("create idx_session_worker: {e}")))?;
    Ok(())
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "migration_reconciliation_tests.rs"]
mod migration_reconciliation_tests;
#[cfg(test)]
#[path = "handoff_stats_query_tests.rs"]
mod handoff_stats_query_tests;
