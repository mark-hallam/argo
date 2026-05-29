//! Generic per-tenant key/value store backing the `/settings` SPA
//! surface (theatre item `T1`).
//!
//! Three Settings panels — workspace, providers, notifications —
//! persist into one table, [`workspace_settings`], keyed by
//! `(tenant_id, group_key)`. Each row's `value_json` is the entire
//! panel object serialized as JSON; the gateway handler owns the panel
//! shapes + their defaults. This module deliberately stays
//! shape-agnostic: it stores and returns opaque JSON strings so adding
//! a fourth panel later needs no store change.
//!
//! Tenant scoping is structural — every query is keyed on
//! `(tenant_id, group_key)`, the table's composite primary key, so one
//! tenant can never read or overwrite another tenant's row.
//!
//! Schema lives in migration v85 (`migrations/v85.rs`). Crate-scoped
//! tests apply it with the versioned runner
//! (`super::migrations::run_up_to`); the live runtime applies it on
//! `SessionStore` open.
//!
//! [`workspace_settings`]: crate::workspace_settings
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_store::workspace_settings::{get_group, upsert_group};
//! use sqlx::sqlite::SqlitePoolOptions;
//! let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
//! // (the live runtime has already applied migration v85 here)
//! upsert_group(&pool, "tenant-a", "workspace", "{\"name\":\"Acme\"}", 1_700_000_000).await?;
//! let stored = get_group(&pool, "tenant-a", "workspace").await?;
//! assert_eq!(stored.as_deref(), Some("{\"name\":\"Acme\"}"));
//! # Ok(()) }
//! ```

use argo_types::{ArgoError, ArgoResult};
use sqlx::SqlitePool;

/// Fetch the stored `value_json` for one `(tenant, group)` panel.
///
/// Returns `Ok(None)` when the tenant has never saved that panel — the
/// caller then falls back to its bundled defaults so a GET always
/// yields a complete object. The lookup is keyed on the composite
/// primary key, so it is inherently tenant-scoped.
///
/// # Errors
/// Returns [`ArgoError::Provider`] if the underlying query fails (e.g.
/// the `workspace_settings` table is absent because migrations have
/// not run).
///
/// ```no_run
/// # async fn demo(pool: &sqlx::SqlitePool) -> argo_types::ArgoResult<()> {
/// use argo_store::workspace_settings::get_group;
/// let maybe = get_group(pool, "tenant-a", "providers").await?;
/// assert!(maybe.is_none() || maybe.is_some());
/// # Ok(()) }
/// ```
pub async fn get_group(
    pool: &SqlitePool,
    tenant_id: &str,
    group_key: &str,
) -> ArgoResult<Option<String>> {
    let value: Option<String> = sqlx::query_scalar(
        "SELECT value_json FROM workspace_settings \
         WHERE tenant_id = ? AND group_key = ?",
    )
    .bind(tenant_id)
    .bind(group_key)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workspace_settings get: {e}")))?;
    Ok(value)
}

/// Insert or replace the stored panel object for one `(tenant, group)`.
///
/// `value_json` must already be the fully-merged, serialized panel
/// object — this module does not merge or validate shape (that is the
/// gateway handler's job). The upsert keys on the composite primary
/// key so it can only ever touch the calling tenant's own row.
///
/// # Errors
/// Returns [`ArgoError::Provider`] if the upsert fails.
///
/// ```no_run
/// # async fn demo(pool: &sqlx::SqlitePool) -> argo_types::ArgoResult<()> {
/// use argo_store::workspace_settings::upsert_group;
/// upsert_group(pool, "tenant-a", "notifications", "{\"email_enabled\":true}", 1).await?;
/// # Ok(()) }
/// ```
pub async fn upsert_group(
    pool: &SqlitePool,
    tenant_id: &str,
    group_key: &str,
    value_json: &str,
    updated_at: i64,
) -> ArgoResult<()> {
    sqlx::query(
        "INSERT INTO workspace_settings (tenant_id, group_key, value_json, updated_at) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(tenant_id, group_key) \
         DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
    )
    .bind(tenant_id)
    .bind(group_key)
    .bind(value_json)
    .bind(updated_at)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workspace_settings upsert: {e}")))?;
    Ok(())
}

#[cfg(test)]
#[path = "workspace_settings_tests.rs"]
mod tests;
