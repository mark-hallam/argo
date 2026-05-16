//! Row CRUD for the `user_account` substrate table
//! (Stream `mission-tenant-substrate-tables`).
//!
//! Lives in `argo-identity` rather than `argo-store` (the brief's
//! original location) because `argo-identity` already depends on
//! `argo-store`; the inverse would be a cycle and the typed
//! [`UserAccount`] lives here.
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_identity::user::UserAccount;
//! use argo_identity::user_store::{insert_user, get_user_by_email};
//! use argo_store::SessionStore;
//! let store = SessionStore::open(":memory:").await?;
//! store.migrate().await?;
//! let u = UserAccount::new("alice@acme.com".into());
//! insert_user(&store, &u).await?;
//! assert!(get_user_by_email(&store, "alice@acme.com").await?.is_some());
//! # Ok(()) }
//! ```

use argo_store::tenant_query::ensure_tenant_substrate_schema;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use std::str::FromStr;
use uuid::Uuid;

use crate::user::{UserAccount, UserId, UserStatus};

/// Insert one `user_account` row. Fails on UNIQUE(email) collision.
pub async fn insert_user(store: &SessionStore, u: &UserAccount) -> ArgoResult<()> {
    ensure_tenant_substrate_schema(store).await?;
    sqlx::query(
        "INSERT INTO user_account (id, email, display_name, created_at, last_login_at, status) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(u.id.0.as_bytes().as_slice())
    .bind(&u.email)
    .bind(u.display_name.as_deref())
    .bind(u.created_at)
    .bind(u.last_login_at)
    .bind(u.status.as_str())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("insert_user: {e}")))?;
    Ok(())
}

/// Fetch one user by their verified email, or `None` when no row exists.
pub async fn get_user_by_email(
    store: &SessionStore,
    email: &str,
) -> ArgoResult<Option<UserAccount>> {
    ensure_tenant_substrate_schema(store).await?;
    let row: Option<UserRow> = sqlx::query_as(
        "SELECT id, email, display_name, created_at, last_login_at, status \
         FROM user_account WHERE email = ?",
    )
    .bind(email)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("get_user_by_email: {e}")))?;
    row.map(user_from_row).transpose()
}

/// Fetch one user by [`UserId`], or `None` when no row exists.
pub async fn get_user(store: &SessionStore, id: UserId) -> ArgoResult<Option<UserAccount>> {
    ensure_tenant_substrate_schema(store).await?;
    let row: Option<UserRow> = sqlx::query_as(
        "SELECT id, email, display_name, created_at, last_login_at, status \
         FROM user_account WHERE id = ?",
    )
    .bind(id.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("get_user: {e}")))?;
    row.map(user_from_row).transpose()
}

/// Set `last_login_at` to `now`. Returns the row count affected (0 when
/// `id` is unknown).
pub async fn update_last_login(
    store: &SessionStore,
    id: UserId,
    now: DateTime<Utc>,
) -> ArgoResult<u64> {
    ensure_tenant_substrate_schema(store).await?;
    let res = sqlx::query("UPDATE user_account SET last_login_at = ? WHERE id = ?")
        .bind(now.timestamp())
        .bind(id.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("update_last_login: {e}")))?;
    Ok(res.rows_affected())
}

type UserRow = (Vec<u8>, String, Option<String>, i64, Option<i64>, String);

fn user_from_row(
    (id, email, display_name, created_at, last_login_at, status): UserRow,
) -> ArgoResult<UserAccount> {
    Ok(UserAccount {
        id: UserId(
            Uuid::from_slice(&id)
                .map_err(|e| ArgoError::Provider(format!("user id decode: {e}")))?,
        ),
        email,
        display_name,
        created_at,
        last_login_at,
        status: UserStatus::from_str(&status)
            .map_err(|e| ArgoError::Provider(format!("user status decode: {e}")))?,
    })
}

#[cfg(test)]
#[path = "user_store_tests.rs"]
mod tests;
