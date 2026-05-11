//! Per-tenant rate-limit override store (Stream
//! `mission-per-tenant-rate-limiting`, MISSION_BACKLOG Section 1
//! #21).
//!
//! The gateway token-buckets every tenant at a config-driven default
//! (`[gateway.rate_limit.tenant].requests_per_minute`). An operator
//! can replace that default for a single tenant by upserting a row
//! into `tenant_rate_limit`. Missing row → caller falls back to the
//! config default.
//!
//! Schema is owned by `migrations/v56.rs`; callers must run
//! `SessionStore::migrate` before constructing a [`TenantRateLimitStore`].
//!
//! Tenant id is stored as a `BLOB` (UUID bytes) for symmetry with
//! every other tenant-scoped table.
use crate::StoreResult;
use argo_types::{ArgoError, TenantId};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

/// One stored override row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TenantRateLimitOverride {
    /// Tenant whose default is being overridden.
    pub tenant_id: TenantId,
    /// Replacement per-minute budget (`> 0`).
    pub requests_per_minute: u32,
    /// Unix timestamp, seconds, when the override was last written.
    pub updated_at: i64,
}

/// Thin wrapper around the workspace SQLite pool.
pub struct TenantRateLimitStore<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> TenantRateLimitStore<'a> {
    /// Construct over an already-migrated workspace pool.
    #[must_use]
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Upsert one tenant's override. Returns the row's `updated_at`
    /// timestamp on success.
    ///
    /// # Errors
    ///
    /// Returns [`ArgoError::Provider`] on SQL execution failure or
    /// when `requests_per_minute == 0` (the limiter treats `0` as
    /// "disabled", which is reserved for the global config knob).
    ///
    /// ```no_run
    /// # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_store::StoreResult<()> {
    /// use argo_store::tenant_rate_limit::TenantRateLimitStore;
    /// use argo_types::TenantId;
    /// let store = TenantRateLimitStore::new(pool);
    /// store.set(TenantId::new(), 1500, 1_700_000_000).await?;
    /// # Ok(()) }
    /// ```
    pub async fn set(
        &self,
        tenant: TenantId,
        requests_per_minute: u32,
        now_unix: i64,
    ) -> StoreResult<i64> {
        if requests_per_minute == 0 {
            return Err(ArgoError::Provider(
                "tenant_rate_limit set: requests_per_minute must be > 0".into(),
            ));
        }
        sqlx::query(
            "INSERT INTO tenant_rate_limit (tenant_id, requests_per_minute, updated_at) \
             VALUES (?, ?, ?) \
             ON CONFLICT (tenant_id) DO UPDATE SET \
                 requests_per_minute = excluded.requests_per_minute, \
                 updated_at          = excluded.updated_at",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .bind(i64::from(requests_per_minute))
        .bind(now_unix)
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit set: {e}")))?;
        Ok(now_unix)
    }

    /// Read one tenant's override if present.
    ///
    /// ```no_run
    /// # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_store::StoreResult<()> {
    /// use argo_store::tenant_rate_limit::TenantRateLimitStore;
    /// use argo_types::TenantId;
    /// let _row = TenantRateLimitStore::new(pool).get(TenantId::new()).await?;
    /// # Ok(()) }
    /// ```
    pub async fn get(
        &self,
        tenant: TenantId,
    ) -> StoreResult<Option<TenantRateLimitOverride>> {
        let row: Option<(i64, i64)> = sqlx::query_as(
            "SELECT requests_per_minute, updated_at \
             FROM tenant_rate_limit WHERE tenant_id = ?",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_optional(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit get: {e}")))?;
        Ok(row.map(|(rpm, updated_at)| TenantRateLimitOverride {
            tenant_id: tenant,
            requests_per_minute: u32::try_from(rpm).unwrap_or(0),
            updated_at,
        }))
    }

    /// Stream every stored override. Order is unspecified — callers
    /// that need a stable view should sort client-side. Used by both
    /// the gateway's startup hydration of its in-memory map and by
    /// `argo tenant rate-limit list`.
    pub async fn list(&self) -> StoreResult<Vec<TenantRateLimitOverride>> {
        let rows: Vec<(Vec<u8>, i64, i64)> = sqlx::query_as(
            "SELECT tenant_id, requests_per_minute, updated_at \
             FROM tenant_rate_limit",
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit list: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for (tid_bytes, rpm, updated_at) in rows {
            let tenant_id = Uuid::from_slice(&tid_bytes)
                .map(TenantId)
                .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit row uuid: {e}")))?;
            out.push(TenantRateLimitOverride {
                tenant_id,
                requests_per_minute: u32::try_from(rpm).unwrap_or(0),
                updated_at,
            });
        }
        Ok(out)
    }

    /// Delete one tenant's override. Returns `true` when a row was
    /// removed (so `argo tenant rate-limit delete` can distinguish
    /// "cleared" from "no row").
    pub async fn delete(&self, tenant: TenantId) -> StoreResult<bool> {
        let result = sqlx::query("DELETE FROM tenant_rate_limit WHERE tenant_id = ?")
            .bind(tenant.0.as_bytes().as_slice())
            .execute(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit delete: {e}")))?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
#[path = "tenant_rate_limit_tests.rs"]
mod tests;
