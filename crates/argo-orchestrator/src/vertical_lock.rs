//! Advisory lock for one worker running one vertical at a time.
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use sqlx::SqlitePool;

const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Runtime knobs for [`VerticalConcurrencyLock`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockConfig {
    /// Seconds before a held row is considered abandoned.
    pub lease_secs: u64,
    /// Seconds to wait for a busy lock before returning contention.
    pub acquire_timeout_secs: u64,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            lease_secs: 30,
            acquire_timeout_secs: 5,
        }
    }
}

/// Per-`(tenant, vertical, worker)` advisory lock backed by SQLite.
///
/// ```
/// # async fn doc() -> argo_types::ArgoResult<()> {
/// use std::sync::Arc;
/// use argo_orchestrator::VerticalConcurrencyLock;
/// use argo_store::SessionStore;
/// use argo_types::TenantId;
///
/// let store = Arc::new(SessionStore::open(":memory:").await?);
/// store.migrate().await?;
/// let lock = VerticalConcurrencyLock::new(store);
/// let _guard = lock.acquire(TenantId::new(), "support", "worker-a").await?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct VerticalConcurrencyLock {
    pool: SqlitePool,
    config: LockConfig,
}

impl VerticalConcurrencyLock {
    /// Build a lock using [`LockConfig::default`].
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self::with_config(store, LockConfig::default())
    }

    /// Build a lock with explicit runtime knobs.
    #[must_use]
    pub fn with_config(store: Arc<SessionStore>, config: LockConfig) -> Self {
        Self {
            pool: store.pool().clone(),
            config,
        }
    }

    /// Acquire the lock or return a provider error after the timeout.
    pub async fn acquire(
        &self,
        tenant_id: TenantId,
        vertical_id: &str,
        worker_id: &str,
    ) -> ArgoResult<LockGuard> {
        let deadline = Instant::now() + Duration::from_secs(self.config.acquire_timeout_secs);
        let tenant = tenant_id.to_string();
        loop {
            let acquired_at = unix_now()?;
            let lease_secs = i64::try_from(self.config.lease_secs).unwrap_or(i64::MAX);
            if try_acquire(
                &self.pool,
                &tenant,
                vertical_id,
                worker_id,
                i64::from(std::process::id()),
                acquired_at,
                lease_secs,
            )
            .await?
            {
                return Ok(LockGuard::new(
                    self.pool.clone(),
                    tenant,
                    vertical_id,
                    worker_id,
                    acquired_at,
                ));
            }
            if Instant::now() >= deadline {
                return Err(ArgoError::Provider(format!(
                    "vertical-lock contention timeout on (tenant={}, vertical={}, worker={}) after {}s",
                    tenant, vertical_id, worker_id, self.config.acquire_timeout_secs
                )));
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    /// Delete a lock row regardless of holder. Intended for operator repair.
    pub async fn force_release(
        &self,
        tenant_id: TenantId,
        vertical_id: &str,
        worker_id: &str,
    ) -> ArgoResult<u64> {
        let result = sqlx::query(
            "DELETE FROM vertical_locks
             WHERE tenant_id = ? AND vertical_id = ? AND worker_id = ?",
        )
        .bind(tenant_id.to_string())
        .bind(vertical_id)
        .bind(worker_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("vertical lock force_release: {e}")))?;
        Ok(result.rows_affected())
    }
}

/// RAII holder for an acquired vertical lock.
pub struct LockGuard {
    pool: SqlitePool,
    tenant_id: String,
    vertical_id: String,
    worker_id: String,
    holder_pid: i64,
    acquired_at: i64,
}

impl LockGuard {
    fn new(
        pool: SqlitePool,
        tenant_id: String,
        vertical_id: &str,
        worker_id: &str,
        acquired_at: i64,
    ) -> Self {
        Self {
            pool,
            tenant_id,
            vertical_id: vertical_id.to_owned(),
            worker_id: worker_id.to_owned(),
            holder_pid: i64::from(std::process::id()),
            acquired_at,
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let fut = release_owned(
            self.pool.clone(),
            self.tenant_id.clone(),
            self.vertical_id.clone(),
            self.worker_id.clone(),
            self.holder_pid,
            self.acquired_at,
        );
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(err) = fut.await {
                    tracing::warn!(error = %err, "vertical lock release failed");
                }
            });
            return;
        }
        match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => {
                if let Err(err) = rt.block_on(fut) {
                    tracing::warn!(error = %err, "vertical lock release failed");
                }
            }
            Err(err) => tracing::warn!(error = %err, "vertical lock release runtime failed"),
        }
    }
}

async fn try_acquire(
    pool: &SqlitePool,
    tenant_id: &str,
    vertical_id: &str,
    worker_id: &str,
    holder_pid: i64,
    acquired_at: i64,
    lease_secs: i64,
) -> ArgoResult<bool> {
    let result = sqlx::query(
        "INSERT INTO vertical_locks (
             tenant_id, vertical_id, worker_id, holder_pid, acquired_at, lease_secs
         ) VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(tenant_id, vertical_id, worker_id) DO UPDATE SET
             holder_pid = excluded.holder_pid,
             acquired_at = excluded.acquired_at,
             lease_secs = excluded.lease_secs
         WHERE vertical_locks.acquired_at + vertical_locks.lease_secs < ?",
    )
    .bind(tenant_id)
    .bind(vertical_id)
    .bind(worker_id)
    .bind(holder_pid)
    .bind(acquired_at)
    .bind(lease_secs)
    .bind(acquired_at)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("vertical lock acquire: {e}")))?;
    Ok(result.rows_affected() == 1)
}

async fn release_owned(
    pool: SqlitePool,
    tenant_id: String,
    vertical_id: String,
    worker_id: String,
    holder_pid: i64,
    acquired_at: i64,
) -> ArgoResult<()> {
    sqlx::query(
        "DELETE FROM vertical_locks
         WHERE tenant_id = ? AND vertical_id = ? AND worker_id = ?
           AND holder_pid = ? AND acquired_at = ?",
    )
    .bind(tenant_id)
    .bind(vertical_id)
    .bind(worker_id)
    .bind(holder_pid)
    .bind(acquired_at)
    .execute(&pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("vertical lock release: {e}")))?;
    Ok(())
}

fn unix_now() -> ArgoResult<i64> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| ArgoError::Invariant(format!("system clock before unix epoch: {e}")))?
        .as_secs();
    Ok(i64::try_from(secs).unwrap_or(i64::MAX))
}
