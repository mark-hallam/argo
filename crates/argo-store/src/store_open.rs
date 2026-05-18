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

/// Environment override used by [`SessionStore::open`] and
/// [`SessionStore::open_with_profile`] to resolve a per-deployment
/// SQLCipher key without threading argo-config through the store crate
/// (which would introduce a circular dep — argo-config already depends
/// on argo-core, which depends on argo-store). The variable carries a
/// base64-encoded 32-byte secret; gateway boot resolves it from the OS
/// keyring via `argo_config::cred::db_key::get_or_create_db_key` and
/// exports it before `SessionStore::open` runs. Same name as
/// `argo_config::cred::db_key::DB_KEY_ENV` — wire-format compatible.
pub const DB_KEY_ENV: &str = "SKYCOT_DB_KEY_BASE64";

/// In-memory database URLs (`:memory:` and the sqlite-uri form). These
/// paths have no on-disk surface, so `PRAGMA key` is meaningless;
/// every test-suite store using one of these strings must keep working
/// after the encrypted-by-default flip.
fn is_in_memory(path: &str) -> bool {
    let s = path.trim();
    s == ":memory:" || s == "sqlite::memory:" || s.starts_with("file::memory:")
}

/// Body of [`SessionStore::open_with_profile`] — the legacy entry
/// point. Now consults the `SKYCOT_DB_KEY_BASE64` env var on file-
/// backed paths and applies `PRAGMA key` when set. In-memory paths
/// stay unencrypted regardless of the env var so the ~666 existing
/// test callsites that pass `":memory:"` don't have to be rewritten.
pub(crate) async fn open_with_profile(
    path: impl AsRef<Path>,
    profile: wal_tuning::WalTuningProfile,
) -> StoreResult<SessionStore> {
    let path_str = path.as_ref().to_string_lossy().to_string();
    let key_hex = if is_in_memory(&path_str) {
        None
    } else {
        hex_key_from_env()?
    };
    open_inner(path, profile, key_hex).await
}

/// Body of [`SessionStore::open_unencrypted_for_tests`] — explicit
/// opt-out of the env-driven encryption flip. Test fixtures that
/// intentionally exercise the plaintext code path use this so they
/// stay deterministic even when a CI runner has
/// `SKYCOT_DB_KEY_BASE64` exported globally.
pub(crate) async fn open_unencrypted(
    path: impl AsRef<Path>,
    profile: wal_tuning::WalTuningProfile,
) -> StoreResult<SessionStore> {
    open_inner(path, profile, None).await
}

/// Body of [`SessionStore::open_with_encryption`] — applies `PRAGMA key`
/// from `key_source` before any other SQL runs.
pub(crate) async fn open_with_encryption(
    path: impl AsRef<Path>,
    profile: wal_tuning::WalTuningProfile,
    key_source: &KeySource,
) -> StoreResult<SessionStore> {
    open_inner(path, profile, key_source.resolve()?).await
}

/// Body of [`SessionStore::open_with_hex_key`] — applies the supplied
/// already-validated hex key. Callers that have resolved a per-tenant
/// secret via `argo_config::cred::db_key::get_or_create_db_key` decode
/// the base64 payload to hex with [`db_key_secret_to_hex`] and pass it
/// here.
pub(crate) async fn open_with_hex_key(
    path: impl AsRef<Path>,
    profile: wal_tuning::WalTuningProfile,
    hex_key: &str,
) -> StoreResult<SessionStore> {
    open_inner(path, profile, Some(hex_key.to_string())).await
}

async fn open_inner(
    path: impl AsRef<Path>,
    profile: wal_tuning::WalTuningProfile,
    key_hex: Option<String>,
) -> StoreResult<SessionStore> {
    let path_str = path.as_ref().to_string_lossy().to_string();
    let mut opts = SqliteConnectOptions::from_str(&path_str)
        .map_err(|e| ArgoError::Provider(format!("sqlite open: {e}")))?
        .create_if_missing(true);
    // `PRAGMA journal_mode = WAL` must run AFTER `PRAGMA key` on a
    // SQLCipher-linked pool — sqlx applies whatever
    // `SqliteConnectOptions::journal_mode` configures during the
    // pre-`after_connect` connection bring-up, which is the wrong
    // ordering for the encrypted path. For unencrypted callers we
    // keep the historical option-driven WAL setup; for encrypted
    // callers `after_connect` issues `PRAGMA key` first and then sets
    // WAL, matching the SQLCipher operator runbook.
    let key_stmt = if let Some(hex) = key_hex.as_deref() {
        Some(format!(
            "PRAGMA key = \"x'{}'\"",
            encryption::validate_hex_key(hex)?
        ))
    } else {
        opts = opts.journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        None
    };

    let pool_cfg = pool_config::StorePoolConfig::from_env();
    let mut builder = SqlitePoolOptions::new()
        .max_connections(pool_cfg.max_connections)
        .min_connections(pool_cfg.min_connections)
        .acquire_timeout(pool_cfg.acquire_timeout);
    if let Some(idle) = pool_cfg.idle_timeout {
        builder = builder.idle_timeout(Some(idle));
    }
    // `PRAGMA key` is per-connection in SQLCipher: every connection
    // sqlx hands the pool must be keyed before the first non-pragma
    // statement runs against it. `after_connect` is the only correct
    // entry point — running `apply_encryption_pragma` on `&pool` after
    // `connect_with` only keys the first acquired connection, so any
    // subsequent `acquire` returning a different cached connection
    // would fail with `file is not a database` on the next read.
    if let Some(stmt) = key_stmt {
        builder = builder.after_connect(move |conn, _meta| {
            let key_stmt = stmt.clone();
            Box::pin(async move {
                // audit-allow: validated SQLCipher PRAGMA literal
                // (hex-only, gatekept by `validate_hex_key`).
                sqlx::query(&key_stmt).execute(&mut *conn).await?;
                sqlx::query("PRAGMA journal_mode = WAL")
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        });
    }
    let pool = builder
        .connect_with(opts)
        .await
        .map_err(|e| ArgoError::Provider(format!("sqlite pool: {e}")))?;

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

/// Resolve `SKYCOT_DB_KEY_BASE64` into a 64-char lowercase hex string,
/// or `None` when the env var is unset/empty. A set-but-malformed value
/// is a hard error — silent fallback to unencrypted would be a
/// security regression on the encrypted-by-default path.
pub(crate) fn hex_key_from_env() -> StoreResult<Option<String>> {
    let raw = match std::env::var(DB_KEY_ENV) {
        Ok(v) => v,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(ArgoError::Schema(format!(
                "{DB_KEY_ENV} is set but not valid UTF-8"
            )));
        }
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    encryption::base64_to_hex(trimmed).map(Some)
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
