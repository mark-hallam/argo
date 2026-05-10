//! `CuScreenshotStore` — persist CU screenshots through
//! `argo-store`'s `BlobStore` layer with a per-tenant SQL index
//! and TTL-driven retention sweep (Stream
//! `claude-cu-screenshot-storage`).
//!
//! ## Flow
//!
//! 1. `put` writes the PNG bytes through `BlobStore::put` (returns
//!    a `BlobRef`), then inserts an index row that captures the
//!    (`tenant`, `session`, `action`, `step`, `blob_id`, `captured_at`,
//!    `retention_until`) tuple plus the lightweight `BlobRef`
//!    metadata so subsequent reads/lists don't need a per-row
//!    round-trip into the blob store.
//! 2. `get` validates the requested `(tenant, blob_id)` matches an
//!    index row before pulling the bytes — this is what enforces
//!    cross-tenant isolation. A cross-tenant `get` returns
//!    `ArgoError::NotFound` (HTTP 404).
//! 3. `list_session` enumerates the per-session shots in
//!    `(action_id, step_n)` order so a replay client can
//!    deterministically render the trajectory.
//! 4. `sweep_expired` deletes index rows whose `retention_until`
//!    has passed and returns the deleted count. Bytes-level
//!    deletion is the underlying `BlobStore`'s job (the trait
//!    deliberately exposes no `delete`); production backends own
//!    that policy.
//!
//! The `ScreenshotPersister` impl at the bottom is the seam the
//! CU executor uses to push every step's screenshot through this
//! store without taking a cyclic dependency on `argo-cli`.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use argo_executor_computer_use::screenshot_persist::ScreenshotPersister;
use argo_store::attached_blob::{AttachedBlob, BlobId, BlobRef, BlobStore};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

const SCREENSHOT_MIME: &str = "image/png";

/// Embedded mirror of `crates/argo-store/migrations/45__cu_screenshot_index.sql`.
/// Held inline so `CuScreenshotStore::ensure_schema` can apply it
/// idempotently without `include_str!`-ing across crate boundaries.
/// Kept in lockstep with the `.sql` file under
/// `crates/argo-store/migrations/`.
const CU_SCREENSHOT_SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS cu_screenshot (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    step_n INTEGER NOT NULL,
    blob_id TEXT NOT NULL,
    blob_mime TEXT NOT NULL,
    blob_size_bytes INTEGER NOT NULL,
    blob_sha256 BLOB NOT NULL,
    captured_at INTEGER NOT NULL,
    retention_until INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_cu_screenshot_tenant_session
    ON cu_screenshot(tenant_id, session_id, action_id, step_n);
CREATE INDEX IF NOT EXISTS idx_cu_screenshot_retention
    ON cu_screenshot(retention_until);";

/// Persists CU screenshots and exposes the per-tenant query +
/// sweep API. Cloneable via the underlying `Arc<dyn BlobStore>` +
/// `SqlitePool`; safe to share across worker tasks.
///
/// `get` / `list_session` are kept `pub` for the in-flight replay
/// brief (`claude-cu-action-replay-cli`); they are not wired to a
/// CLI surface yet, so the bin-target `dead-code` lint would
/// otherwise flag them. The `#[allow]` is scoped to the impl
/// block so it lifts naturally once that brief lands.
#[derive(Clone)]
pub struct CuScreenshotStore {
    blobs: Arc<dyn BlobStore>,
    pool: SqlitePool,
    retention: Duration,
}

impl std::fmt::Debug for CuScreenshotStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CuScreenshotStore")
            .field("retention", &self.retention)
            .finish_non_exhaustive()
    }
}

impl CuScreenshotStore {
    /// Build a store wrapping the given `BlobStore` + SQLite pool.
    /// `retention` is the TTL the sweep uses; a typical production
    /// value is `Duration::from_secs(14 * 86_400)` (14 days).
    #[must_use]
    pub fn new(blobs: Arc<dyn BlobStore>, pool: SqlitePool, retention: Duration) -> Self {
        Self {
            blobs,
            pool,
            retention,
        }
    }

    /// Idempotently apply the `cu_screenshot` index schema. Safe
    /// to call on every entry point (mirrors the
    /// `worker_okr::ensure_schema` pattern).
    pub async fn ensure_schema(&self) -> ArgoResult<()> {
        for stmt in CU_SCREENSHOT_SCHEMA.split_terminator(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }
            sqlx::query(sql)
                .execute(&self.pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("cu_screenshot schema: {e}")))?;
        }
        Ok(())
    }

    /// Persist a screenshot for `(tenant, session, action, step)`.
    /// Writes the bytes through the underlying `BlobStore`, then
    /// indexes the returned `BlobRef`. `captured_at` is `now()`;
    /// `retention_until = captured_at + retention`.
    pub async fn put(
        &self,
        tenant_id: &str,
        session_id: &str,
        action_id: &str,
        step_n: u32,
        png_bytes: Vec<u8>,
    ) -> ArgoResult<BlobRef> {
        self.ensure_schema().await?;
        let blob = AttachedBlob::new(png_bytes, SCREENSHOT_MIME.to_string());
        let blob_ref = self.blobs.put(blob).await?;
        let captured_at = unix_seconds_now()?;
        let retention_secs = i64::try_from(self.retention.as_secs())
            .map_err(|_| ArgoError::Provider("cu_screenshot: retention overflow".into()))?;
        let retention_until = captured_at.saturating_add(retention_secs);
        sqlx::query(
            "INSERT INTO cu_screenshot \
             (tenant_id, session_id, action_id, step_n, blob_id, blob_mime, \
              blob_size_bytes, blob_sha256, captured_at, retention_until) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(tenant_id)
        .bind(session_id)
        .bind(action_id)
        .bind(i64::from(step_n))
        .bind(blob_ref.id.to_string())
        .bind(&blob_ref.mime)
        .bind(i64::try_from(blob_ref.size_bytes).unwrap_or(i64::MAX))
        .bind(&blob_ref.sha256[..])
        .bind(captured_at)
        .bind(retention_until)
        .execute(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("cu_screenshot insert: {e}")))?;
        Ok(blob_ref)
    }

    /// Fetch the bytes for `blob_id` if (and only if) an index row
    /// records that blob as belonging to `tenant_id`. Cross-tenant
    /// gets return `ArgoError::NotFound` — the row check is what
    /// enforces tenant isolation.
    #[allow(dead_code)] // wired by `claude-cu-action-replay-cli` follow-up
    pub async fn get(&self, tenant_id: &str, blob_id: BlobId) -> ArgoResult<Vec<u8>> {
        self.ensure_schema().await?;
        let id_str = blob_id.to_string();
        let row = sqlx::query("SELECT 1 FROM cu_screenshot WHERE tenant_id = ? AND blob_id = ?")
            .bind(tenant_id)
            .bind(&id_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("cu_screenshot lookup: {e}")))?;
        if row.is_none() {
            return Err(ArgoError::NotFound(format!(
                "cu_screenshot {id_str} (tenant={tenant_id})"
            )));
        }
        let blob = self.blobs.get(blob_id).await?;
        Ok(blob.bytes)
    }

    /// Enumerate the screenshots indexed for `(tenant_id,
    /// session_id)` in `(action_id, step_n)` order. Returns the
    /// lightweight `BlobRef`s reconstructed from the index row so
    /// the caller doesn't pay the blob-read cost per entry.
    #[allow(dead_code)] // wired by `claude-cu-action-replay-cli` follow-up
    pub async fn list_session(
        &self,
        tenant_id: &str,
        session_id: &str,
    ) -> ArgoResult<Vec<BlobRef>> {
        self.ensure_schema().await?;
        let rows = sqlx::query(
            "SELECT blob_id, blob_mime, blob_size_bytes, blob_sha256 \
             FROM cu_screenshot \
             WHERE tenant_id = ? AND session_id = ? \
             ORDER BY action_id ASC, step_n ASC",
        )
        .bind(tenant_id)
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("cu_screenshot list_session: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id_str: String = row.get("blob_id");
            let id = parse_blob_id(&id_str)?;
            let mime: String = row.get("blob_mime");
            let size_i64: i64 = row.get("blob_size_bytes");
            let size_bytes = u64::try_from(size_i64).unwrap_or(0);
            let sha_vec: Vec<u8> = row.get("blob_sha256");
            let sha256 = sha_vec.as_slice().try_into().map_err(|_| {
                ArgoError::Provider(format!(
                    "cu_screenshot list_session: sha256 length {} != 32",
                    sha_vec.len()
                ))
            })?;
            out.push(BlobRef {
                id,
                mime,
                size_bytes,
                sha256,
            });
        }
        Ok(out)
    }

    /// Delete every index row whose `retention_until` has passed,
    /// returning the deleted count. The trait exposes no
    /// blob-byte delete API, so this is index-row only — production
    /// blob backends own bytes-level lifecycle.
    pub async fn sweep_expired(
        &self,
        now: SystemTime,
        retention: Duration,
    ) -> ArgoResult<usize> {
        self.ensure_schema().await?;
        let now_secs = i64::try_from(
            now.duration_since(UNIX_EPOCH)
                .map_err(|e| ArgoError::Provider(format!("cu_screenshot sweep clock: {e}")))?
                .as_secs(),
        )
        .map_err(|_| ArgoError::Provider("cu_screenshot sweep: time overflow".into()))?;
        // Recompute the boundary using the supplied `retention` so
        // the operator can override the sweep horizon at call time
        // (e.g. to drain immediately).
        let retention_secs = i64::try_from(retention.as_secs())
            .map_err(|_| ArgoError::Provider("cu_screenshot sweep: retention overflow".into()))?;
        let boundary = now_secs.saturating_sub(retention_secs);
        let result = sqlx::query("DELETE FROM cu_screenshot WHERE captured_at <= ?")
            .bind(boundary)
            .execute(&self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("cu_screenshot sweep delete: {e}")))?;
        Ok(usize::try_from(result.rows_affected()).unwrap_or(0))
    }
}

#[async_trait]
impl ScreenshotPersister for CuScreenshotStore {
    async fn put_screenshot(
        &self,
        tenant_id: &str,
        session_id: &str,
        action_id: &str,
        step_n: u32,
        png: &[u8],
    ) -> ArgoResult<BlobRef> {
        self.put(tenant_id, session_id, action_id, step_n, png.to_vec())
            .await
    }
}

fn unix_seconds_now() -> ArgoResult<i64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| ArgoError::Provider(format!("cu_screenshot clock: {e}")))?;
    i64::try_from(now.as_secs())
        .map_err(|_| ArgoError::Provider("cu_screenshot clock: time overflow".into()))
}

#[allow(dead_code)]
fn parse_blob_id(s: &str) -> ArgoResult<BlobId> {
    let uuid = Uuid::parse_str(s)
        .map_err(|e| ArgoError::Provider(format!("cu_screenshot: bad blob_id '{s}': {e}")))?;
    Ok(BlobId(uuid))
}

#[cfg(test)]
#[path = "cu_screenshots_tests.rs"]
mod tests;
