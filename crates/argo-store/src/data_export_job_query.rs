//! Customer-facing data-export job tracker
//! (Stream `mission-data-export-customer-flow`).
//!
//! Each row is one GDPR/CCPA Article 15 request from an authenticated
//! user. Lifecycle: `queued` → `running` → `ready` (or `failed`); a
//! `ready` row's `expires_at` is consulted by the download path so
//! abandoned links time out after the 7-day window.
//!
//! Schema is owned by `migrations/116__data_export_job.sql`; the
//! runtime [`ensure_schema`] helper applies it idempotently, mirroring
//! the same lazy-bootstrap pattern as `magic_link_tokens` and
//! `outbound_message` (the versioned migration runner caps at v70).
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_store::data_export_job_query::{
//!     insert_job, mark_ready, ExportJobStatus, JobInsert,
//! };
//! use sqlx::sqlite::SqlitePoolOptions;
//! use uuid::Uuid;
//! let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
//! insert_job(&pool, &JobInsert {
//!     job_id: "job-1",
//!     user_id: Uuid::nil(),
//!     tenant_id: None,
//!     requested_at: 100,
//! }).await?;
//! mark_ready(&pool, "job-1", b"tar-bytes", "tokenhash", 100, 100 + 7 * 86_400).await?;
//! # Ok(()) }
//! ```

use argo_types::{ArgoError, ArgoResult};
use sqlx::SqlitePool;
use uuid::Uuid;

/// 7-day TTL for a completed export's download link.
pub const DOWNLOAD_TTL_SECS: i64 = 7 * 24 * 60 * 60;

/// 24h rate-limit window — one successful export per user per day.
pub const RATE_LIMIT_WINDOW_SECS: i64 = 24 * 60 * 60;

/// Lifecycle states for one row of `data_export_job`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportJobStatus {
    /// Insert-time default; the background task hasn't started yet.
    Queued,
    /// Background assembly in flight; not yet downloadable.
    Running,
    /// Tarball persisted; `download_token` + `expires_at` populated.
    Ready,
    /// `expires_at` has elapsed; download refused.
    Expired,
    /// Assembly failed; `error_message` populated for the operator.
    Failed,
}

impl ExportJobStatus {
    /// Stable lowercase label persisted in the `status` column.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Ready => "ready",
            Self::Expired => "expired",
            Self::Failed => "failed",
        }
    }

    /// Inverse of [`Self::as_str`] — `None` on unknown labels so the
    /// caller can surface a corruption-shaped error. Named
    /// `parse_label` (rather than `from_str`) to avoid clashing with
    /// [`std::str::FromStr`]'s signature; the SQL column is a plain
    /// TEXT enum, not a user-facing parse target.
    #[must_use]
    pub fn parse_label(s: &str) -> Option<Self> {
        Some(match s {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "ready" => Self::Ready,
            "expired" => Self::Expired,
            "failed" => Self::Failed,
            _ => return None,
        })
    }
}

/// Insert payload — the columns set when a job is first queued.
#[derive(Debug, Clone)]
pub struct JobInsert<'a> {
    /// Caller-generated job id (uuid-v7 stringified).
    pub job_id: &'a str,
    /// Authenticated user the job is scoped to.
    pub user_id: Uuid,
    /// Optional tenant scope (the user's primary tenant on request).
    pub tenant_id: Option<Uuid>,
    /// Unix-seconds timestamp of insert.
    pub requested_at: i64,
}

/// One row of `data_export_job` as returned to the polling handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportJobRow {
    /// Caller-issued job id.
    pub job_id: String,
    /// User the export belongs to.
    pub user_id: Uuid,
    /// Lifecycle state at read time.
    pub status: ExportJobStatus,
    /// `Some` once the job is `ready` — SHA-256 hex of the raw download token.
    pub download_token: Option<String>,
    /// `Some` on `failed` rows.
    pub error_message: Option<String>,
    /// Insert timestamp.
    pub requested_at: i64,
    /// Set when the row reaches a terminal state.
    pub completed_at: Option<i64>,
    /// Set on `ready` rows; consulted by the download handler.
    pub expires_at: Option<i64>,
}

const ENSURE_SQL: &str = include_str!("../migrations/116__data_export_job.sql");

/// Ensure the `data_export_job` table + supporting indices exist.
/// Idempotent; safe to call from every entrypoint.
pub async fn ensure_schema(pool: &SqlitePool) -> ArgoResult<()> {
    let stripped: String = ENSURE_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    for stmt in stripped.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt)
            .execute(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("data_export_job ensure: {e}")))?;
    }
    Ok(())
}

/// Insert one freshly-queued export job. Caller mints `job_id`
/// (uuid-v7 stringified) and `requested_at` so the insert is
/// deterministic for tests.
pub async fn insert_job(pool: &SqlitePool, insert: &JobInsert<'_>) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        "INSERT INTO data_export_job \
         (job_id, user_id, tenant_id, status, requested_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(insert.job_id)
    .bind(insert.user_id.as_bytes().as_slice())
    .bind(insert.tenant_id.as_ref().map(|t| t.as_bytes().to_vec()))
    .bind(ExportJobStatus::Queued.as_str())
    .bind(insert.requested_at)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("data_export_job insert: {e}")))?;
    Ok(())
}

/// Mark `Queued` → `Running` for the named job.
pub async fn mark_running(pool: &SqlitePool, job_id: &str) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    sqlx::query("UPDATE data_export_job SET status = ? WHERE job_id = ?")
        .bind(ExportJobStatus::Running.as_str())
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("data_export_job running: {e}")))?;
    Ok(())
}

/// Persist the assembled tarball + the SHA-256-hashed download token,
/// flip status to `Ready`, and stamp `completed_at`/`expires_at`.
pub async fn mark_ready(
    pool: &SqlitePool,
    job_id: &str,
    bundle_bytes: &[u8],
    token_hash: &str,
    completed_at: i64,
    expires_at: i64,
) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        "UPDATE data_export_job SET \
         status = ?, bundle_bytes = ?, download_token = ?, \
         completed_at = ?, expires_at = ? \
         WHERE job_id = ?",
    )
    .bind(ExportJobStatus::Ready.as_str())
    .bind(bundle_bytes)
    .bind(token_hash)
    .bind(completed_at)
    .bind(expires_at)
    .bind(job_id)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("data_export_job ready: {e}")))?;
    Ok(())
}

/// Mark a row `Failed` with an operator-facing error message.
pub async fn mark_failed(
    pool: &SqlitePool,
    job_id: &str,
    error_message: &str,
    completed_at: i64,
) -> ArgoResult<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        "UPDATE data_export_job SET status = ?, error_message = ?, completed_at = ? \
         WHERE job_id = ?",
    )
    .bind(ExportJobStatus::Failed.as_str())
    .bind(error_message)
    .bind(completed_at)
    .bind(job_id)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("data_export_job failed: {e}")))?;
    Ok(())
}

/// Fetch the row scoped to the requesting `user_id`. Returns `None`
/// when no row matches (or the row belongs to another user — same
/// shape so a brute-force scan can't distinguish "your stale job"
/// from "another user's job").
pub async fn get_job_for_user(
    pool: &SqlitePool,
    job_id: &str,
    user_id: Uuid,
) -> ArgoResult<Option<ExportJobRow>> {
    ensure_schema(pool).await?;
    let row: Option<JobRow> = sqlx::query_as(
        "SELECT job_id, user_id, status, download_token, error_message, \
         requested_at, completed_at, expires_at \
         FROM data_export_job WHERE job_id = ? AND user_id = ?",
    )
    .bind(job_id)
    .bind(user_id.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("data_export_job get: {e}")))?;
    row.map(decode_row).transpose()
}

type BundleRow = (
    String,
    Vec<u8>,
    String,
    Option<String>,
    Option<String>,
    i64,
    Option<i64>,
    Option<i64>,
    Vec<u8>,
);

/// Resolve a `Ready` row by the SHA-256-hashed download token, plus
/// its in-row bundle bytes. `None` when the hash is unknown OR the
/// row's `expires_at` has elapsed at `now` — the caller surfaces that
/// as 404 so token-existence doesn't leak.
pub async fn fetch_ready_bundle(
    pool: &SqlitePool,
    token_hash: &str,
    now: i64,
) -> ArgoResult<Option<(ExportJobRow, Vec<u8>)>> {
    ensure_schema(pool).await?;
    let row: Option<BundleRow> = sqlx::query_as(
        "SELECT job_id, user_id, status, download_token, error_message, \
         requested_at, completed_at, expires_at, bundle_bytes \
         FROM data_export_job WHERE download_token = ?",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("data_export_job fetch: {e}")))?;
    let Some((
        job_id,
        user_id,
        status,
        download_token,
        error_message,
        requested_at,
        completed_at,
        expires_at,
        bundle_bytes,
    )) = row
    else {
        return Ok(None);
    };
    if expires_at.is_some_and(|exp| exp <= now) {
        return Ok(None);
    }
    let job = decode_row((
        job_id,
        user_id,
        status,
        download_token,
        error_message,
        requested_at,
        completed_at,
        expires_at,
    ))?;
    Ok(Some((job, bundle_bytes)))
}

/// Count rows the user has requested inside the rate-limit window
/// (`since_unix_secs..`). Used to enforce 1 export per 24h.
pub async fn count_jobs_since(
    pool: &SqlitePool,
    user_id: Uuid,
    since_unix_secs: i64,
) -> ArgoResult<u32> {
    ensure_schema(pool).await?;
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM data_export_job \
         WHERE user_id = ? AND requested_at >= ?",
    )
    .bind(user_id.as_bytes().as_slice())
    .bind(since_unix_secs)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("data_export_job count: {e}")))?;
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

type JobRow = (
    String,
    Vec<u8>,
    String,
    Option<String>,
    Option<String>,
    i64,
    Option<i64>,
    Option<i64>,
);

fn decode_row(
    (job_id, user_id, status, download_token, error_message, requested_at, completed_at, expires_at): JobRow,
) -> ArgoResult<ExportJobRow> {
    Ok(ExportJobRow {
        job_id,
        user_id: Uuid::from_slice(&user_id)
            .map_err(|e| ArgoError::Provider(format!("data_export_job user_id decode: {e}")))?,
        status: ExportJobStatus::parse_label(&status)
            .ok_or_else(|| ArgoError::Provider(format!("unknown status '{status}'")))?,
        download_token,
        error_message,
        requested_at,
        completed_at,
        expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("memory pool")
    }

    fn insert(id: &str, user_id: Uuid, requested_at: i64) -> JobInsert<'_> {
        JobInsert {
            job_id: id,
            user_id,
            tenant_id: None,
            requested_at,
        }
    }

    #[tokio::test]
    async fn insert_then_get_round_trips() {
        let p = pool().await;
        let user = Uuid::now_v7();
        insert_job(&p, &insert("job-a", user, 100)).await.unwrap();
        let got = get_job_for_user(&p, "job-a", user).await.unwrap().unwrap();
        assert_eq!(got.status, ExportJobStatus::Queued);
        assert_eq!(got.requested_at, 100);
        assert!(got.download_token.is_none());
    }

    #[tokio::test]
    async fn other_users_jobs_are_invisible() {
        let p = pool().await;
        let alice = Uuid::now_v7();
        let bob = Uuid::now_v7();
        insert_job(&p, &insert("job-a", alice, 100)).await.unwrap();
        assert!(get_job_for_user(&p, "job-a", bob).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn ready_bundle_round_trips_then_expires() {
        let p = pool().await;
        let user = Uuid::now_v7();
        insert_job(&p, &insert("job-b", user, 100)).await.unwrap();
        mark_running(&p, "job-b").await.unwrap();
        mark_ready(&p, "job-b", b"tarball-bytes", "tokenhash", 200, 200 + DOWNLOAD_TTL_SECS)
            .await
            .unwrap();
        let (job, bytes) = fetch_ready_bundle(&p, "tokenhash", 300)
            .await
            .unwrap()
            .expect("ready bundle visible");
        assert_eq!(job.status, ExportJobStatus::Ready);
        assert_eq!(bytes, b"tarball-bytes");
        let expired = fetch_ready_bundle(&p, "tokenhash", 200 + DOWNLOAD_TTL_SECS + 1)
            .await
            .unwrap();
        assert!(expired.is_none(), "token must time out at expires_at");
    }

    #[tokio::test]
    async fn count_jobs_since_only_counts_window() {
        let p = pool().await;
        let user = Uuid::now_v7();
        insert_job(&p, &insert("job-old", user, 100)).await.unwrap();
        insert_job(&p, &insert("job-new", user, 1_000)).await.unwrap();
        assert_eq!(count_jobs_since(&p, user, 500).await.unwrap(), 1);
        assert_eq!(count_jobs_since(&p, user, 50).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn mark_failed_records_message() {
        let p = pool().await;
        let user = Uuid::now_v7();
        insert_job(&p, &insert("job-c", user, 100)).await.unwrap();
        mark_failed(&p, "job-c", "boom", 250).await.unwrap();
        let got = get_job_for_user(&p, "job-c", user).await.unwrap().unwrap();
        assert_eq!(got.status, ExportJobStatus::Failed);
        assert_eq!(got.error_message.as_deref(), Some("boom"));
        assert_eq!(got.completed_at, Some(250));
    }
}
