//! Workspace + tenant bootstrap queries.
//!
//! Used by `argo workspace init` to register a freshly-created
//! workspace in the DB (so `MemoryStore`, `SkillStore`, etc. have a
//! row to anchor their inserts to).
use argo_types::{ArgoError, TenantId, WorkspaceId};
use chrono::Utc;
use sqlx::{Pool, Sqlite};

use crate::models;
use crate::SessionStore;
use crate::StoreResult;

impl SessionStore {
    /// `WorkspaceId` of the first registered workspace, or
    /// `None` when none exists. Used by anything that wants to
    /// bootstrap memory/skill writes against "the" workspace.
    pub async fn first_workspace(&self) -> StoreResult<Option<WorkspaceId>> {
        let row: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT id FROM workspace ORDER BY created_at ASC LIMIT 1")
                .fetch_optional(self.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("first_workspace: {e}")))?;
        Ok(row.map(|b| WorkspaceId(models::uuid_from_blob(&b))))
    }

    /// `name` of the first registered workspace. Used by the
    /// TUI footer.
    pub async fn first_workspace_name(&self) -> StoreResult<Option<String>> {
        let row: Option<String> =
            sqlx::query_scalar("SELECT name FROM workspace ORDER BY created_at ASC LIMIT 1")
                .fetch_optional(self.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("first_workspace_name: {e}")))?;
        Ok(row)
    }

    /// Full info for the first registered workspace —
    /// `(workspace_id, name, root_path, created_at_micros)`.
    /// Used by `GET /workspace/info`.
    pub async fn first_workspace_info(
        &self,
    ) -> StoreResult<Option<(WorkspaceId, String, String, i64)>> {
        let row: Option<(Vec<u8>, String, String, i64)> = sqlx::query_as(
            "SELECT id, name, root_path, created_at \
             FROM workspace ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("first_workspace_info: {e}")))?;
        Ok(row.map(|(id, name, root, ts)| {
            (
                WorkspaceId(models::uuid_from_blob(&id)),
                name,
                root,
                ts,
            )
        }))
    }

    /// Full info for a specific workspace by id —
    /// `(workspace_id, name, root_path, created_at_micros)`.
    /// Used by gateway endpoints that need the metadata row for
    /// the cwd-resolved workspace (matches `ensure_workspace`'s
    /// path; #718). Returns `None` if the id doesn't exist.
    pub async fn workspace_info(
        &self,
        id: WorkspaceId,
    ) -> StoreResult<Option<(WorkspaceId, String, String, i64)>> {
        let row: Option<(Vec<u8>, String, String, i64)> = sqlx::query_as(
            "SELECT id, name, root_path, created_at FROM workspace WHERE id = ?",
        )
        .bind(id.0.as_bytes().as_slice())
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("workspace_info: {e}")))?;
        Ok(row.map(|(id, name, root, ts)| {
            (
                WorkspaceId(models::uuid_from_blob(&id)),
                name,
                root,
                ts,
            )
        }))
    }
}

/// Insert a tenant + workspace pair if no workspace already exists at
/// `root_path`. Returns the resolved `(TenantId, WorkspaceId)`.
///
/// Idempotent: re-running against the same `root_path` reuses the
/// existing row instead of creating a duplicate.
pub(super) async fn ensure_workspace(
    pool: &Pool<Sqlite>,
    name: &str,
    root_path: &str,
) -> Result<(TenantId, WorkspaceId), ArgoError> {
    if let Some((t, w)) = lookup_by_root(pool, root_path).await? {
        return Ok((t, w));
    }
    let tenant = TenantId::new();
    let workspace = WorkspaceId::new();
    let created_at = Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(name)
    .bind(root_path)
    .bind(created_at)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("ensure_workspace insert: {e}")))?;
    Ok((tenant, workspace))
}

async fn lookup_by_root(
    pool: &Pool<Sqlite>,
    root_path: &str,
) -> Result<Option<(TenantId, WorkspaceId)>, ArgoError> {
    let row: Option<(Vec<u8>, Vec<u8>)> =
        sqlx::query_as("SELECT id, tenant_id FROM workspace WHERE root_path = ? LIMIT 1")
            .bind(root_path)
            .fetch_optional(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("ensure_workspace lookup: {e}")))?;
    Ok(row.map(|(w, t)| {
        (
            TenantId(super::models::uuid_from_blob(&t)),
            WorkspaceId(super::models::uuid_from_blob(&w)),
        )
    }))
}
