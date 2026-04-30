//! Workspace + tenant bootstrap queries.
//!
//! Used by `argo workspace init` to register a freshly-created
//! workspace in the DB (so `MemoryStore`, `SkillStore`, etc. have a
//! row to anchor their inserts to).
use argo_types::{ArgoError, TenantId, WorkspaceId};
use chrono::Utc;
use sqlx::{Pool, Sqlite};

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
