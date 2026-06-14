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

    /// `WorkspaceId` of the oldest workspace **belonging to `tenant`**,
    /// or `None` when that tenant has none.
    ///
    /// Unlike [`Self::first_workspace`] — which is global and always
    /// resolves to the oldest workspace overall (the demo / cwd one) —
    /// this is tenant-scoped. The WS1 write path uses it to bind a new
    /// session to the *signed-in* tenant's own workspace instead of the
    /// cwd/demo workspace. `tenant_provision::provision_for_user` writes
    /// exactly one such row per tenant, so this returns it for any
    /// provisioned user.
    pub async fn first_workspace_for_tenant(
        &self,
        tenant: TenantId,
    ) -> StoreResult<Option<WorkspaceId>> {
        let row: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT id FROM workspace WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("first_workspace_for_tenant: {e}")))?;
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
        Ok(row
            .map(|(id, name, root, ts)| (WorkspaceId(models::uuid_from_blob(&id)), name, root, ts)))
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
        let row: Option<(Vec<u8>, String, String, i64)> =
            sqlx::query_as("SELECT id, name, root_path, created_at FROM workspace WHERE id = ?")
                .bind(id.0.as_bytes().as_slice())
                .fetch_optional(self.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("workspace_info: {e}")))?;
        Ok(row
            .map(|(id, name, root, ts)| (WorkspaceId(models::uuid_from_blob(&id)), name, root, ts)))
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
    // Store the SAME canonical key the read path (`lookup_by_root`) binds, so
    // a later resolve of any spelling of this directory reuses this row
    // instead of minting a phantom tenant (win-store-canonicalize-workspace-root).
    let root_key = canonicalize_root_key(root_path);
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(name)
    .bind(&root_key)
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
    let key = canonicalize_root_key(root_path);
    let row: Option<(Vec<u8>, Vec<u8>)> =
        sqlx::query_as("SELECT id, tenant_id FROM workspace WHERE root_path = ? LIMIT 1")
            .bind(&key)
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

/// Canonicalize a workspace `root_path` into a stable comparison key before it
/// is bound to SQL, so the SAME directory always resolves to the SAME
/// `(TenantId, WorkspaceId)` regardless of how the caller spelled the path.
///
/// On **unix** this is a byte-for-byte passthrough — a given `cwd`'s
/// `to_string_lossy()` is already stable there, so every existing row and the
/// live workspace keep resolving exactly as before (no migration, no re-mint).
///
/// On **Windows** the SAME directory can arrive as `C:\..` vs `C:/..`, with a
/// lowercase drive letter, a `\\?\` verbatim prefix, or a trailing separator —
/// each a different string that would otherwise MISS [`lookup_by_root`] and
/// mint a phantom tenant, splitting one tenant's data across phantom tenants
/// (`win-store-canonicalize-workspace-root`). The Windows branch normalizes
/// those purely as strings — no filesystem access (`std::fs::canonicalize`
/// would hit disk, follow symlinks, and can fail; out of scope).
#[cfg(not(windows))]
fn canonicalize_root_key(root_path: &str) -> String {
    root_path.to_string()
}

/// Windows comparison-key normalizer — see [`canonicalize_root_key`]. Pure
/// string canonicalization, idempotent (`f(f(x)) == f(x)`).
#[cfg(windows)]
fn canonicalize_root_key(root_path: &str) -> String {
    // Strip a `\\?\` verbatim prefix BEFORE separator translation, while it is
    // still backslashes (e.g. `\\?\C:\Users\x` -> `C:\Users\x`).
    let stripped = root_path.strip_prefix(r"\\?\").unwrap_or(root_path);
    // Canonical separator: forward slash.
    let key = stripped.replace('\\', "/");
    // Lowercase a leading drive letter (`C:` -> `c:`) via char iteration so the
    // result is always valid UTF-8 without an unwrap.
    let key = match key.split_once(':') {
        Some((drive, rest))
            if drive.len() == 1 && drive.chars().all(|c| c.is_ascii_alphabetic()) =>
        {
            format!("{}:{rest}", drive.to_ascii_lowercase())
        }
        _ => key,
    };
    // Trim a single trailing separator, but never reduce a bare drive root
    // (`c:/`) to `c:` — only trim when a path component follows the drive.
    let mut key = key;
    if key.len() > 3 && key.ends_with('/') {
        key.pop();
    }
    key
}

#[cfg(test)]
#[path = "workspace_root_key_tests.rs"]
mod workspace_root_key_tests;
