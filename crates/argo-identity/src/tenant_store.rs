//! Row CRUD for the `tenant` substrate table
//! (Stream `mission-tenant-substrate-tables`).
//!
//! Placement note: the brief's may-touch list named
//! `argo-store/src/tenant_query.rs` for these helpers, but
//! `argo-identity` already depends on `argo-store` (so the inverse
//! dependency would be a cycle), and the `Tenant` type lives here.
//! Schema bootstrap stays in `argo-store::tenant_query`; only the
//! typed CRUD moved.
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_identity::tenant::Tenant;
//! use argo_identity::tenant_store::{insert_tenant, get_tenant};
//! use argo_store::SessionStore;
//! let store = SessionStore::open(":memory:").await?;
//! store.migrate().await?;
//! let t = Tenant::new("Acme".into(), "billing@acme.com".into());
//! insert_tenant(&store, &t).await?;
//! assert_eq!(get_tenant(&store, t.id).await?.unwrap().name, "Acme");
//! # Ok(()) }
//! ```

use argo_store::tenant_query::ensure_tenant_substrate_schema;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use std::str::FromStr;
use uuid::Uuid;

use crate::tenant::{Tenant, TenantStatus};

/// Insert one `tenant` row. Fails on PRIMARY KEY collision.
pub async fn insert_tenant(store: &SessionStore, t: &Tenant) -> ArgoResult<()> {
    ensure_tenant_substrate_schema(store).await?;
    sqlx::query(
        "INSERT INTO tenant (id, name, billing_email, status, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(t.id.0.as_bytes().as_slice())
    .bind(&t.name)
    .bind(&t.billing_email)
    .bind(t.status.as_str())
    .bind(t.created_at)
    .bind(t.updated_at)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("insert_tenant: {e}")))?;
    Ok(())
}

/// Fetch one tenant by id, or `None` when no row exists.
pub async fn get_tenant(store: &SessionStore, id: TenantId) -> ArgoResult<Option<Tenant>> {
    ensure_tenant_substrate_schema(store).await?;
    let row: Option<TenantRow> = sqlx::query_as(
        "SELECT id, name, billing_email, status, created_at, updated_at \
         FROM tenant WHERE id = ?",
    )
    .bind(id.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("get_tenant: {e}")))?;
    row.map(tenant_from_row).transpose()
}

/// List every tenant row, newest first by `created_at`.
pub async fn list_tenants(store: &SessionStore) -> ArgoResult<Vec<Tenant>> {
    ensure_tenant_substrate_schema(store).await?;
    let rows: Vec<TenantRow> = sqlx::query_as(
        "SELECT id, name, billing_email, status, created_at, updated_at \
         FROM tenant ORDER BY created_at DESC",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list_tenants: {e}")))?;
    rows.into_iter().map(tenant_from_row).collect()
}

/// Replace `tenant.status` and bump `updated_at` to `now`. Returns the
/// row count affected (0 when `id` is unknown).
pub async fn set_tenant_status(
    store: &SessionStore,
    id: TenantId,
    status: TenantStatus,
    now: DateTime<Utc>,
) -> ArgoResult<u64> {
    ensure_tenant_substrate_schema(store).await?;
    let res = sqlx::query("UPDATE tenant SET status = ?, updated_at = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(now.timestamp())
        .bind(id.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("set_tenant_status: {e}")))?;
    Ok(res.rows_affected())
}

type TenantRow = (Vec<u8>, String, String, String, i64, i64);

fn tenant_from_row(
    (id, name, billing_email, status, created_at, updated_at): TenantRow,
) -> ArgoResult<Tenant> {
    Ok(Tenant {
        id: TenantId(
            Uuid::from_slice(&id)
                .map_err(|e| ArgoError::Provider(format!("tenant id decode: {e}")))?,
        ),
        name,
        billing_email,
        status: TenantStatus::from_str(&status)
            .map_err(|e| ArgoError::Provider(format!("tenant status decode: {e}")))?,
        created_at,
        updated_at,
    })
}

#[cfg(test)]
#[path = "tenant_store_tests.rs"]
mod tests;
