//! Per-tenant resource quota storage and headroom checks.
//!
//! Schema is owned by `migrations/v38.rs`; callers must run
//! `SessionStore::migrate` before constructing a `TenantQuotaStore`.
//!
//! Usage accounting and the headroom-check methods live in the
//! sibling `tenant_quota_usage` module; their public types are
//! re-exported here so external callers continue to import via
//! `argo_store::tenant_quota::*`.

pub use crate::tenant_quota_kinds::{QuotaActionKind, QuotaError, TenantQuotaKind};
pub use crate::tenant_quota_usage::{TenantHeadroom, TenantQuotaRemaining, TenantUsage};
use crate::tenant_quota_usage::{
    self, micros_to_usd, remaining, table_exists, u64_from_i64,
};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

/// The calendar window used to aggregate tenant usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaPeriod {
    /// UTC day boundary.
    Daily,
    /// UTC calendar month boundary.
    Monthly,
}

impl QuotaPeriod {
    /// Stable database label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Monthly => "monthly",
        }
    }
}

/// One tenant quota row.
#[derive(Debug, Clone, Serialize)]
pub struct TenantQuota {
    /// Tenant id as stored in the worker identity tables.
    pub tenant_id: String,
    /// Calendar window for spend and action counts.
    pub period: QuotaPeriod,
    /// Optional USD cap in micros.
    pub max_usd_micros: Option<u64>,
    /// Optional USD cap as a decimal dollar value for operator output.
    pub max_usd: Option<f64>,
    /// Optional action count cap for the period.
    pub max_actions: Option<u64>,
    /// Optional cap on active worker rows for the tenant.
    pub max_workers: Option<u64>,
    /// Unix timestamp, seconds, when the row was last updated.
    pub set_at: i64,
}

/// Patch used by the quota setter.
#[derive(Debug, Clone, Copy, Default)]
pub struct TenantQuotaUpdate {
    /// Replacement period. Existing period is preserved when absent.
    pub period: Option<QuotaPeriod>,
    /// Replacement USD cap in micros. Existing cap is preserved when absent.
    pub max_usd_micros: Option<u64>,
    /// Replacement action cap. Existing cap is preserved when absent.
    pub max_actions: Option<u64>,
    /// Replacement worker cap. Existing cap is preserved when absent.
    pub max_workers: Option<u64>,
}

/// A single quota field that can be cleared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantQuotaField {
    /// Clear `max_usd_micros`.
    MaxUsd,
    /// Clear `max_actions`.
    MaxActions,
    /// Clear `max_workers`.
    MaxWorkers,
}

/// Quota plus live usage for one tenant.
#[derive(Debug, Clone, Serialize)]
pub struct TenantQuotaSummary {
    /// Tenant id.
    pub tenant_id: String,
    /// Configured quota, if any.
    pub quota: Option<TenantQuota>,
    /// Current usage for the quota period.
    pub usage: TenantUsage,
    /// Remaining headroom for each configured cap.
    pub remaining: TenantQuotaRemaining,
}

/// Thin wrapper around the workspace SQLite pool.
pub struct TenantQuotaStore<'a> {
    pool: &'a SqlitePool,
}

impl<'a> TenantQuotaStore<'a> {
    /// Bind quota operations to a SQLite pool.
    #[must_use]
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert or update one tenant quota row.
    pub async fn set_quota(
        &self,
        tenant_id: &str,
        update: TenantQuotaUpdate,
        now: DateTime<Utc>,
    ) -> ArgoResult<TenantQuotaSummary> {
        let existing = self.quota(tenant_id).await?;
        let period = update
            .period
            .or_else(|| existing.as_ref().map(|q| q.period))
            .unwrap_or(QuotaPeriod::Monthly);
        let max_usd = update
            .max_usd_micros
            .or_else(|| existing.as_ref().and_then(|q| q.max_usd_micros));
        let max_actions = update
            .max_actions
            .or_else(|| existing.as_ref().and_then(|q| q.max_actions));
        let max_workers = update
            .max_workers
            .or_else(|| existing.as_ref().and_then(|q| q.max_workers));
        sqlx::query(
            "INSERT INTO tenant_quota \
             (tenant_id, period, max_usd_micros, max_actions, max_workers, set_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(tenant_id) DO UPDATE SET \
             period = excluded.period, \
             max_usd_micros = excluded.max_usd_micros, \
             max_actions = excluded.max_actions, \
             max_workers = excluded.max_workers, \
             set_at = excluded.set_at",
        )
        .bind(tenant_id)
        .bind(period.as_str())
        .bind(max_usd.map(db_i64))
        .bind(max_actions.map(db_i64))
        .bind(max_workers.map(db_i64))
        .bind(now.timestamp())
        .execute(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant quota set: {e}")))?;
        self.summary(tenant_id, now).await
    }

    /// Load the raw quota row for a tenant.
    pub async fn quota(&self, tenant_id: &str) -> ArgoResult<Option<TenantQuota>> {
        let row = sqlx::query(
            "SELECT tenant_id, period, max_usd_micros, max_actions, max_workers, set_at \
             FROM tenant_quota WHERE tenant_id = ?",
        )
        .bind(tenant_id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant quota get: {e}")))?;
        row.map(quota_from_row).transpose()
    }

    /// List every configured tenant quota with live usage.
    pub async fn list(&self, now: DateTime<Utc>) -> ArgoResult<Vec<TenantQuotaSummary>> {
        let rows = sqlx::query("SELECT tenant_id FROM tenant_quota ORDER BY tenant_id")
            .fetch_all(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("tenant quota list: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let tenant_id: String = row.get("tenant_id");
            out.push(self.summary(&tenant_id, now).await?);
        }
        Ok(out)
    }

    /// Remove an entire quota row or clear one configured field.
    pub async fn clear(
        &self,
        tenant_id: &str,
        field: Option<TenantQuotaField>,
        now: DateTime<Utc>,
    ) -> ArgoResult<TenantQuotaSummary> {
        match field {
            None => {
                sqlx::query("DELETE FROM tenant_quota WHERE tenant_id = ?")
                    .bind(tenant_id)
                    .execute(self.pool)
                    .await
                    .map_err(|e| ArgoError::Provider(format!("tenant quota clear: {e}")))?;
            }
            Some(field) => {
                let column = match field {
                    TenantQuotaField::MaxUsd => "max_usd_micros",
                    TenantQuotaField::MaxActions => "max_actions",
                    TenantQuotaField::MaxWorkers => "max_workers",
                };
                let sql = format!(
                    "UPDATE tenant_quota SET {column} = NULL, set_at = ? WHERE tenant_id = ?"
                );
                sqlx::query(&sql)
                    .bind(now.timestamp())
                    .bind(tenant_id)
                    .execute(self.pool)
                    .await
                    .map_err(|e| ArgoError::Provider(format!("tenant quota clear field: {e}")))?;
            }
        }
        self.summary(tenant_id, now).await
    }

    /// Load quota, usage, and remaining headroom for one tenant.
    pub async fn summary(
        &self,
        tenant_id: &str,
        now: DateTime<Utc>,
    ) -> ArgoResult<TenantQuotaSummary> {
        let quota = self.quota(tenant_id).await?;
        let period = quota.as_ref().map_or(QuotaPeriod::Monthly, |q| q.period);
        let usage = self.usage(tenant_id, period, now).await?;
        let remaining = remaining(quota.as_ref(), &usage);
        Ok(TenantQuotaSummary {
            tenant_id: tenant_id.to_string(),
            quota,
            usage,
            remaining,
        })
    }

    /// Period-to-date usage snapshot (no quota cap context). Thin
    /// wrapper that delegates to `tenant_quota_usage::usage` so the
    /// existing test surface (`quotas.usage(..)`) keeps working
    /// after the split.
    pub(crate) async fn usage(
        &self,
        tenant_id: &str,
        period: QuotaPeriod,
        now: DateTime<Utc>,
    ) -> ArgoResult<TenantUsage> {
        tenant_quota_usage::usage(self.pool, tenant_id, period, now).await
    }

    /// Resolve a worker id to its tenant id, if the identity table exists.
    pub async fn tenant_for_worker(&self, worker_id: &str) -> ArgoResult<Option<String>> {
        if !table_exists(self.pool, "worker").await? {
            return Ok(None);
        }
        sqlx::query_scalar("SELECT tenant_id FROM worker WHERE id = ? LIMIT 1")
            .bind(worker_id)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("tenant quota worker lookup: {e}")))
    }
}

fn quota_from_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<TenantQuota> {
    let period_raw: String = row.get("period");
    let period = match period_raw.as_str() {
        "daily" => QuotaPeriod::Daily,
        "monthly" => QuotaPeriod::Monthly,
        other => {
            return Err(ArgoError::Provider(format!(
                "tenant quota period '{other}'"
            )))
        }
    };
    let max_usd_micros = row
        .get::<Option<i64>, _>("max_usd_micros")
        .map(u64_from_i64);
    Ok(TenantQuota {
        tenant_id: row.get("tenant_id"),
        period,
        max_usd_micros,
        max_usd: max_usd_micros.map(micros_to_usd),
        max_actions: row.get::<Option<i64>, _>("max_actions").map(u64_from_i64),
        max_workers: row.get::<Option<i64>, _>("max_workers").map(u64_from_i64),
        set_at: row.get("set_at"),
    })
}

fn db_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
#[path = "tenant_quota_tests.rs"]
mod tests;
