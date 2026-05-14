//! `SessionStore` constructor + bootstrap helpers extracted under the
//! 500-LOC `lib.rs` ceiling and the 200-LOC impl-block ceiling.
//!
//! The functions here are the bodies of `SessionStore::open_with_profile`,
//! `SessionStore::verify_schema_version`, and the post-migration
//! `ensure_session_worker_id_column` PRAGMA-detect-then-ALTER. They stay
//! `pub(crate)` — `lib.rs` exposes the public surface through thin
//! delegating wrappers.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use argo_types::ArgoError;

use crate::batched_writer::{self, BatchedWriter};
use crate::encryption::{self, KeySource};
use crate::message_query::flush_messages;
use crate::{pool_config, wal_tuning, SessionStore, StoreResult, EXPECTED_SCHEMA_VERSION};

/// Body of [`SessionStore::open_with_profile`] — no encryption pragma.
pub(crate) async fn open_with_profile(
    path: impl AsRef<Path>,
    profile: wal_tuning::WalTuningProfile,
) -> StoreResult<SessionStore> {
    open_inner(path, profile, &KeySource::None).await
}

/// Body of [`SessionStore::open_with_encryption`] — applies `PRAGMA key`
/// from `key_source` before any other SQL runs.
pub(crate) async fn open_with_encryption(
    path: impl AsRef<Path>,
    profile: wal_tuning::WalTuningProfile,
    key_source: &KeySource,
) -> StoreResult<SessionStore> {
    open_inner(path, profile, key_source).await
}

async fn open_inner(
    path: impl AsRef<Path>,
    profile: wal_tuning::WalTuningProfile,
    key_source: &KeySource,
) -> StoreResult<SessionStore> {
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

    // `PRAGMA key` must precede every other statement on a fresh
    // SQLCipher-linked pool — including the WAL/journal pragmas.
    if let Some(hex) = key_source.resolve()? {
        encryption::apply_encryption_pragma(&pool, &hex).await?;
    }
    wal_tuning::apply_pragmas(&pool, profile).await?;
    let pool_for_writer = pool.clone();
    let messages = Arc::new(BatchedWriter::new(
        move |batch| flush_messages(pool_for_writer.clone(), batch),
        batched_writer::batch_size_from_env(),
        batched_writer::batch_latency_from_env(),
    ));
    Ok(SessionStore {
        pool,
        messages,
        prompt_cache_affinity: None,
    })
}

/// Body of [`SessionStore::verify_schema_version`].
pub(crate) fn verify_schema_version(actual: u32) -> StoreResult<()> {
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

/// PRAGMA-detect-then-ALTER for `session.worker_id` (Stream
/// `claude-session-worker-id-schema`). Mirrors the
/// `ensure_role_columns` / `ensure_worker_charter_columns` pattern
/// in argo-identity: the structured migration runner stops at v11,
/// later additive columns are applied idempotently from a sibling
/// SQL file (`migrations/38__session_worker_id.sql`).
pub(crate) async fn ensure_session_worker_id_column(pool: &Pool<Sqlite>) -> StoreResult<()> {
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
    .map_err(|e| ArgoError::Io(format!("create idx_session_worker: {e}")))?;
    Ok(())
}
