//! Marketplace artifact rows — written when `argo workspace push`
//! successfully uploads a bundle to an OCI registry. Lives in its
//! own file so `lib.rs` stays under the 500-LOC ceiling.
use crate::SessionStore;
use crate::StoreResult;
use argo_types::{ArgoError, WorkspaceId};
use chrono::Utc;
use sqlx::{FromRow, Pool, Sqlite};
use uuid::Uuid;

/// One `marketplace_artifact` row exposed for read-side rendering.
#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct MarketplaceArtifactRow {
    /// Canonical OCI reference (`<host>/<repo>:<tag>`).
    pub oci_ref: String,
    /// Bundle digest (`sha256:<hex>`).
    pub digest: String,
    /// Timestamp the row was written (UTC, micros).
    pub pushed_at: i64,
    /// Free-form attribution (CLI uses `cli:<os_user>`).
    pub pushed_by: String,
}

impl SessionStore {
    /// Insert a row into `marketplace_artifact` recording a
    /// successful push. Idempotent in the sense that re-pushing
    /// the same `(oci_ref, digest)` just adds another row —
    /// callers can dedupe by reading the latest by `pushed_at`.
    pub async fn record_marketplace_artifact(
        &self,
        workspace: WorkspaceId,
        oci_ref: &str,
        digest: &str,
        pushed_by: &str,
    ) -> StoreResult<()> {
        record_marketplace_artifact(self.pool(), workspace, oci_ref, digest, pushed_by)
            .await
            .map_err(|e| ArgoError::Provider(format!("record_marketplace_artifact: {e}")))
    }

    /// List recent marketplace artifacts for a workspace, newest
    /// first. `limit` caps the result row count.
    pub async fn list_marketplace_artifacts(
        &self,
        workspace: WorkspaceId,
        limit: u32,
    ) -> StoreResult<Vec<MarketplaceArtifactRow>> {
        list_marketplace_artifacts(self.pool(), workspace, limit)
            .await
            .map_err(|e| ArgoError::Provider(format!("list_marketplace_artifacts: {e}")))
    }
}

async fn record_marketplace_artifact(
    pool: &Pool<Sqlite>,
    workspace: WorkspaceId,
    oci_ref: &str,
    digest: &str,
    pushed_by: &str,
) -> Result<(), sqlx::Error> {
    let id = Uuid::now_v7();
    let pushed_at = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO marketplace_artifact (id, workspace_id, oci_ref, digest, pushed_at, pushed_by) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(workspace.0.as_bytes().as_slice())
    .bind(oci_ref)
    .bind(digest)
    .bind(pushed_at)
    .bind(pushed_by)
    .execute(pool)
    .await?;
    Ok(())
}

async fn list_marketplace_artifacts(
    pool: &Pool<Sqlite>,
    workspace: WorkspaceId,
    limit: u32,
) -> Result<Vec<MarketplaceArtifactRow>, sqlx::Error> {
    sqlx::query_as::<_, MarketplaceArtifactRow>(
        "SELECT oci_ref, digest, pushed_at, pushed_by \
         FROM marketplace_artifact \
         WHERE workspace_id = ? \
         ORDER BY pushed_at DESC \
         LIMIT ?",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(i64::from(limit))
    .fetch_all(pool)
    .await
}
