//! Row CRUD for the `platform_staff` substrate table (DEC-057 Phase 1).
//!
//! Lives in `argo-identity` for the same dep-graph reason as
//! [`crate::tenant_store`] / [`crate::user_store`]: `argo-identity` already
//! depends on `argo-store`, the inverse would be a cycle, and the typed
//! [`PlatformStaff`] lives here. Schema bootstrap stays in
//! `argo-store::tenant_query::ensure_tenant_substrate_schema`.
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_identity::platform_staff::{PlatformRole, PlatformStaff};
//! use argo_identity::platform_staff_store::{upsert_staff, get_staff};
//! use argo_identity::user::UserId;
//! use argo_store::SessionStore;
//! let store = SessionStore::open(":memory:").await?;
//! store.migrate().await?;
//! let uid = UserId::new();
//! upsert_staff(&store, &PlatformStaff::new(uid, PlatformRole::SuperAdmin, None)).await?;
//! assert!(get_staff(&store, uid).await?.is_some());
//! # Ok(()) }
//! ```

use argo_store::tenant_query::ensure_tenant_substrate_schema;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::str::FromStr;
use uuid::Uuid;

use crate::platform_staff::{PlatformRole, PlatformStaff, PlatformStaffStatus};
use crate::user::UserId;

/// Insert or replace one `platform_staff` row (idempotent grant). Re-running
/// with a new role/status updates the existing grant for that user.
pub async fn upsert_staff(store: &SessionStore, s: &PlatformStaff) -> ArgoResult<()> {
    ensure_tenant_substrate_schema(store).await?;
    sqlx::query(
        "INSERT INTO platform_staff (user_id, role, status, created_at, created_by) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(user_id) DO UPDATE SET role = excluded.role, status = excluded.status",
    )
    .bind(s.user_id.0.as_bytes().as_slice())
    .bind(s.role.as_str())
    .bind(s.status.as_str())
    .bind(s.created_at)
    .bind(s.created_by.as_deref())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("upsert_staff: {e}")))?;
    Ok(())
}

/// Fetch one staff row by user id, or `None` when the user is not staff.
/// `None` is the **default-deny** signal — callers MUST treat absence as
/// "not authorized".
pub async fn get_staff(store: &SessionStore, user_id: UserId) -> ArgoResult<Option<PlatformStaff>> {
    ensure_tenant_substrate_schema(store).await?;
    let row: Option<StaffRow> = sqlx::query_as(
        "SELECT user_id, role, status, created_at, created_by \
         FROM platform_staff WHERE user_id = ?",
    )
    .bind(user_id.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("get_staff: {e}")))?;
    row.map(staff_from_row).transpose()
}

/// List every platform-staff row, newest grant first.
pub async fn list_staff(store: &SessionStore) -> ArgoResult<Vec<PlatformStaff>> {
    ensure_tenant_substrate_schema(store).await?;
    let rows: Vec<StaffRow> = sqlx::query_as(
        "SELECT user_id, role, status, created_at, created_by \
         FROM platform_staff ORDER BY created_at DESC",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list_staff: {e}")))?;
    rows.into_iter().map(staff_from_row).collect()
}

/// Remove one staff grant (hard revoke). Returns the row count affected
/// (0 when the user was not staff).
pub async fn remove_staff(store: &SessionStore, user_id: UserId) -> ArgoResult<u64> {
    ensure_tenant_substrate_schema(store).await?;
    let res = sqlx::query("DELETE FROM platform_staff WHERE user_id = ?")
        .bind(user_id.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("remove_staff: {e}")))?;
    Ok(res.rows_affected())
}

type StaffRow = (Vec<u8>, String, String, i64, Option<String>);

fn staff_from_row(
    (user_id, role, status, created_at, created_by): StaffRow,
) -> ArgoResult<PlatformStaff> {
    Ok(PlatformStaff {
        user_id: UserId(
            Uuid::from_slice(&user_id)
                .map_err(|e| ArgoError::Provider(format!("platform_staff user_id decode: {e}")))?,
        ),
        role: PlatformRole::from_str(&role)
            .map_err(|e| ArgoError::Provider(format!("platform_staff role decode: {e}")))?,
        status: PlatformStaffStatus::from_str(&status)
            .map_err(|e| ArgoError::Provider(format!("platform_staff status decode: {e}")))?,
        created_at,
        created_by,
    })
}

#[cfg(test)]
#[path = "platform_staff_store_tests.rs"]
mod tests;
