//! Row CRUD for the `user_tenant` substrate join table
//! (Stream `mission-tenant-substrate-tables`).
//!
//! Lives in `argo-identity` for the same dep-graph reason as
//! [`crate::tenant_store`] and [`crate::user_store`].
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_identity::tenant::Tenant;
//! use argo_identity::tenant_store::insert_tenant;
//! use argo_identity::user::UserAccount;
//! use argo_identity::user_store::insert_user;
//! use argo_identity::user_tenant::UserTenantRole;
//! use argo_identity::user_tenant_store::{link_user_to_tenant, list_tenants_for_user};
//! use argo_store::SessionStore;
//! let store = SessionStore::open(":memory:").await?;
//! store.migrate().await?;
//! let t = Tenant::new("Acme".into(), "billing@acme.com".into());
//! let u = UserAccount::new("alice@acme.com".into());
//! insert_tenant(&store, &t).await?;
//! insert_user(&store, &u).await?;
//! link_user_to_tenant(&store, u.id, t.id, UserTenantRole::Owner).await?;
//! assert_eq!(list_tenants_for_user(&store, u.id).await?.len(), 1);
//! # Ok(()) }
//! ```

use argo_store::tenant_query::ensure_tenant_substrate_schema;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::str::FromStr;
use uuid::Uuid;

use crate::user::UserId;
use crate::user_tenant::{UserTenant, UserTenantRole};

/// Link `user_id` to `tenant_id` with `role`. Fails on PRIMARY KEY
/// collision (one role per (user, tenant) pair).
pub async fn link_user_to_tenant(
    store: &SessionStore,
    user_id: UserId,
    tenant_id: TenantId,
    role: UserTenantRole,
) -> ArgoResult<UserTenant> {
    ensure_tenant_substrate_schema(store).await?;
    let link = UserTenant::new(user_id, tenant_id, role);
    sqlx::query(
        "INSERT INTO user_tenant (user_id, tenant_id, role, created_at) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(link.user_id.0.as_bytes().as_slice())
    .bind(link.tenant_id.0.as_bytes().as_slice())
    .bind(link.role.as_str())
    .bind(link.created_at)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("link_user_to_tenant: {e}")))?;
    Ok(link)
}

/// Every link row for the given user, newest first.
pub async fn list_tenants_for_user(
    store: &SessionStore,
    user_id: UserId,
) -> ArgoResult<Vec<UserTenant>> {
    ensure_tenant_substrate_schema(store).await?;
    let rows: Vec<LinkRow> = sqlx::query_as(
        "SELECT user_id, tenant_id, role, created_at FROM user_tenant \
         WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list_tenants_for_user: {e}")))?;
    rows.into_iter().map(link_from_row).collect()
}

/// Every link row for the given tenant, newest first.
pub async fn list_users_in_tenant(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Vec<UserTenant>> {
    ensure_tenant_substrate_schema(store).await?;
    let rows: Vec<LinkRow> = sqlx::query_as(
        "SELECT user_id, tenant_id, role, created_at FROM user_tenant \
         WHERE tenant_id = ? ORDER BY created_at DESC",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list_users_in_tenant: {e}")))?;
    rows.into_iter().map(link_from_row).collect()
}

type LinkRow = (Vec<u8>, Vec<u8>, String, i64);

fn link_from_row((user_id, tenant_id, role, created_at): LinkRow) -> ArgoResult<UserTenant> {
    Ok(UserTenant {
        user_id: UserId(
            Uuid::from_slice(&user_id)
                .map_err(|e| ArgoError::Provider(format!("user_tenant user_id decode: {e}")))?,
        ),
        tenant_id: TenantId(
            Uuid::from_slice(&tenant_id)
                .map_err(|e| ArgoError::Provider(format!("user_tenant tenant_id decode: {e}")))?,
        ),
        role: UserTenantRole::from_str(&role)
            .map_err(|e| ArgoError::Provider(format!("user_tenant role decode: {e}")))?,
        created_at,
    })
}

#[cfg(test)]
#[path = "user_tenant_store_tests.rs"]
mod tests;
