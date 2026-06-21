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

/// The `group_key` under which a tenant's autonomy opt-in is persisted
/// in [`workspace_settings`](crate::workspace_settings).
pub const AUTONOMY_GROUP_KEY: &str = "autonomy";

/// The JSON field, within the autonomy group's `value_json`, that opts a
/// tenant into scheduled-duty autofire.
pub const DUTY_AUTOFIRE_FIELD: &str = "duty_autofire";

/// Whether this tenant has opted INTO scheduled-duty autofire.
///
/// **Default OFF (operator standing rule).** Autonomy means a scheduled
/// duty FIRES without a human manually starting it — it is NOT
/// auto-approval of the resulting action; the fired action still passes
/// through the auto-approval / approvals gate. A tenant must explicitly
/// opt in: this returns `false` when the tenant has never saved the
/// autonomy panel, when the stored JSON omits the
/// [`DUTY_AUTOFIRE_FIELD`] field, or when the JSON is unparseable
/// (fail-closed).
///
/// # Errors
/// Returns [`ArgoError::Provider`] only if the underlying query itself
/// fails (e.g. migrations have not run). A malformed stored value is NOT
/// an error — it fail-closes to `false`.
///
/// ```no_run
/// # async fn demo(pool: &sqlx::SqlitePool) -> argo_types::ArgoResult<()> {
/// use argo_store::workspace_settings::tenant_duty_autofire_enabled;
/// // A tenant that never opted in is OFF by default:
/// let on = tenant_duty_autofire_enabled(pool, "tenant-a").await?;
/// assert!(!on);
/// # Ok(()) }
/// ```
pub async fn tenant_duty_autofire_enabled(
    pool: &SqlitePool,
    tenant_id: &str,
) -> ArgoResult<bool> {
    let Some(raw) = get_group(pool, tenant_id, AUTONOMY_GROUP_KEY).await? else {
        return Ok(false); // never saved → OFF
    };
    Ok(serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v.get(DUTY_AUTOFIRE_FIELD).and_then(serde_json::Value::as_bool))
        .unwrap_or(false)) // missing field / unparseable → OFF (fail-closed)
}

#[cfg(test)]
#[path = "workspace_settings_tests.rs"]
mod tests;
