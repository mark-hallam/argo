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
//! Connector-aware overrides are added by raw migration slot 113 and
//! lazily reconciled here for older workspaces.
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

/// One stored override row scoped to a connector.
///
/// # Examples
///
/// ```
/// use argo_store::tenant_rate_limit::TenantConnectorRateLimitOverride;
/// use argo_types::TenantId;
/// let row = TenantConnectorRateLimitOverride {
///     tenant_id: TenantId::new(),
///     connector: "gmail".into(),
///     requests_per_minute: 100,
///     updated_at: 1_700_000_000,
/// };
/// assert_eq!(row.connector, "gmail");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantConnectorRateLimitOverride {
    /// Tenant whose connector budget is overridden.
    pub tenant_id: TenantId,
    /// Connector id, or `*` for the tenant-wide default.
    pub connector: String,
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
        self.set_for_connector(tenant, "*", requests_per_minute, now_unix)
            .await
    }

    /// Upsert one tenant+connector override. `connector=""` maps to `*`.
    ///
    /// ```no_run
    /// # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_store::StoreResult<()> {
    /// use argo_store::tenant_rate_limit::TenantRateLimitStore;
    /// use argo_types::TenantId;
    /// let store = TenantRateLimitStore::new(pool);
    /// store.set_for_connector(TenantId::new(), "gmail", 100, 1_700_000_000).await?;
    /// # Ok(()) }
    /// ```
    pub async fn set_for_connector(
        &self,
        tenant: TenantId,
        connector: &str,
        requests_per_minute: u32,
        now_unix: i64,
    ) -> StoreResult<i64> {
        if requests_per_minute == 0 {
            return Err(ArgoError::Provider(
                "tenant_rate_limit set: requests_per_minute must be > 0".into(),
            ));
        }
        self.ensure_connector_schema().await?;
        let connector = normalize_connector(connector);
        sqlx::query(
            "INSERT INTO tenant_rate_limit \
                 (tenant_id, connector, requests_per_minute, updated_at) \
             VALUES (?, ?, ?, ?) \
             ON CONFLICT (tenant_id, connector) DO UPDATE SET \
                 requests_per_minute = excluded.requests_per_minute, \
                 updated_at          = excluded.updated_at",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .bind(&connector)
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
    pub async fn get(&self, tenant: TenantId) -> StoreResult<Option<TenantRateLimitOverride>> {
        Ok(self
            .get_for_connector(tenant, "*")
            .await?
            .map(|row| TenantRateLimitOverride {
                tenant_id: row.tenant_id,
                requests_per_minute: row.requests_per_minute,
                updated_at: row.updated_at,
            }))
    }

    /// Read an exact connector override, falling back to `*`.
    ///
    /// ```no_run
    /// # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_store::StoreResult<()> {
    /// use argo_store::tenant_rate_limit::TenantRateLimitStore;
    /// use argo_types::TenantId;
    /// let store = TenantRateLimitStore::new(pool);
    /// let _row = store.get_for_connector(TenantId::new(), "slack").await?;
    /// # Ok(()) }
    /// ```
    pub async fn get_for_connector(
        &self,
        tenant: TenantId,
        connector: &str,
    ) -> StoreResult<Option<TenantConnectorRateLimitOverride>> {
        self.ensure_connector_schema().await?;
        let connector = normalize_connector(connector);
        let row: Option<(String, i64, i64)> = sqlx::query_as(
            "SELECT connector, requests_per_minute, updated_at \
             FROM tenant_rate_limit \
             WHERE tenant_id = ? AND connector IN (?, '*') \
             ORDER BY CASE connector WHEN ? THEN 0 ELSE 1 END \
             LIMIT 1",
        )
        .bind(tenant.0.as_bytes().as_slice())
        .bind(&connector)
        .bind(&connector)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit get: {e}")))?;
        Ok(row.map(
            |(stored_connector, rpm, updated_at)| TenantConnectorRateLimitOverride {
                tenant_id: tenant,
                connector: stored_connector,
                requests_per_minute: u32::try_from(rpm).unwrap_or(0),
                updated_at,
            },
        ))
    }

    /// Stream every stored override. Order is unspecified — callers
    /// that need a stable view should sort client-side. Used by both
    /// the gateway's startup hydration of its in-memory map and by
    /// `argo tenant rate-limit list`.
    pub async fn list(&self) -> StoreResult<Vec<TenantRateLimitOverride>> {
        self.ensure_connector_schema().await?;
        let rows: Vec<(Vec<u8>, i64, i64)> = sqlx::query_as(
            "SELECT tenant_id, requests_per_minute, updated_at \
             FROM tenant_rate_limit WHERE connector = '*'",
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

    /// Stream every connector-aware override, including `*` rows.
    ///
    /// ```no_run
    /// # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_store::StoreResult<()> {
    /// use argo_store::tenant_rate_limit::TenantRateLimitStore;
    /// let store = TenantRateLimitStore::new(pool);
    /// let _rows = store.list_connector_overrides().await?;
    /// # Ok(()) }
    /// ```
    pub async fn list_connector_overrides(
        &self,
    ) -> StoreResult<Vec<TenantConnectorRateLimitOverride>> {
        self.ensure_connector_schema().await?;
        let rows: Vec<(Vec<u8>, String, i64, i64)> = sqlx::query_as(
            "SELECT tenant_id, connector, requests_per_minute, updated_at \
             FROM tenant_rate_limit",
        )
        .fetch_all(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit connector list: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for (tid_bytes, connector, rpm, updated_at) in rows {
            let tenant_id = Uuid::from_slice(&tid_bytes)
                .map(TenantId)
                .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit row uuid: {e}")))?;
            out.push(TenantConnectorRateLimitOverride {
                tenant_id,
                connector,
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
        self.delete_for_connector(tenant, "*").await
    }

    /// Delete one tenant+connector override.
    ///
    /// ```no_run
    /// # async fn demo(pool: &sqlx::Pool<sqlx::Sqlite>) -> argo_store::StoreResult<()> {
    /// use argo_store::tenant_rate_limit::TenantRateLimitStore;
    /// use argo_types::TenantId;
    /// let store = TenantRateLimitStore::new(pool);
    /// let _removed = store.delete_for_connector(TenantId::new(), "gmail").await?;
    /// # Ok(()) }
    /// ```
    pub async fn delete_for_connector(
        &self,
        tenant: TenantId,
        connector: &str,
    ) -> StoreResult<bool> {
        self.ensure_connector_schema().await?;
        let connector = normalize_connector(connector);
        let result =
            sqlx::query("DELETE FROM tenant_rate_limit WHERE tenant_id = ? AND connector = ?")
                .bind(tenant.0.as_bytes().as_slice())
                .bind(&connector)
                .execute(self.pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit delete: {e}")))?;
        Ok(result.rows_affected() > 0)
    }

    async fn ensure_connector_schema(&self) -> StoreResult<()> {
        let cols: Vec<(String, i64)> =
            sqlx::query_as("SELECT name, pk FROM pragma_table_info('tenant_rate_limit')")
                .fetch_all(self.pool)
                .await
                .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit schema: {e}")))?;
        let connector_pk = cols
            .iter()
            .find_map(|(name, pk)| (name == "connector").then_some(*pk))
            .unwrap_or(0);
        if connector_pk > 0 {
            return Ok(());
        }
        sqlx::query("DROP TABLE IF EXISTS tenant_rate_limit_v2")
            .execute(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit v2 drop: {e}")))?;
        sqlx::query(
            "CREATE TABLE tenant_rate_limit_v2 (
                tenant_id            BLOB    NOT NULL,
                connector            TEXT    NOT NULL DEFAULT '*',
                requests_per_minute  INTEGER NOT NULL,
                updated_at           INTEGER NOT NULL,
                PRIMARY KEY (tenant_id, connector)
            )",
        )
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit v2 create: {e}")))?;
        sqlx::query(
            "INSERT OR IGNORE INTO tenant_rate_limit_v2 \
                (tenant_id, connector, requests_per_minute, updated_at) \
             SELECT tenant_id, '*', requests_per_minute, updated_at \
             FROM tenant_rate_limit",
        )
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit v2 copy: {e}")))?;
        sqlx::query("DROP TABLE tenant_rate_limit")
            .execute(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit old drop: {e}")))?;
        sqlx::query("ALTER TABLE tenant_rate_limit_v2 RENAME TO tenant_rate_limit")
            .execute(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit v2 rename: {e}")))?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_tenant_rate_limit_connector \
             ON tenant_rate_limit(tenant_id, connector)",
        )
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant_rate_limit connector index: {e}")))?;
        Ok(())
    }
}

fn normalize_connector(connector: &str) -> String {
    let connector = connector.trim();
    if connector.is_empty() {
        "*".to_string()
    } else {
        connector.to_ascii_lowercase()
    }
}

#[cfg(test)]
#[path = "tenant_rate_limit_connector_tests.rs"]
mod connector_tests;
#[cfg(test)]
#[path = "tenant_rate_limit_tests.rs"]
mod tests;
