//! Per-tenant resource quota storage and headroom checks.
//!
//! Schema is owned by `migrations/v38.rs`; callers must run
//! `SessionStore::migrate` before constructing a `TenantQuotaStore`.

pub use crate::tenant_quota_kinds::{QuotaActionKind, QuotaError, TenantQuotaKind};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Datelike, Utc};
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

/// Period-to-date tenant usage.
#[derive(Debug, Clone, Serialize)]
pub struct TenantUsage {
    /// Period used for the spend and action windows.
    pub period: QuotaPeriod,
    /// Period start as a Unix timestamp in seconds.
    pub period_start: i64,
    /// Period-to-date spend in USD micros.
    pub used_usd_micros: u64,
    /// Period-to-date spend in decimal USD.
    pub used_usd: f64,
    /// Period-to-date worker-budget charge rows.
    pub used_actions: u64,
    /// Current worker rows for the tenant.
    pub used_workers: u64,
}

/// Remaining quota headroom.
#[derive(Debug, Clone, Serialize)]
pub struct TenantQuotaRemaining {
    /// Remaining spend in USD micros, when capped.
    pub usd_micros: Option<u64>,
    /// Remaining spend in decimal USD, when capped.
    pub usd: Option<f64>,
    /// Remaining action count, when capped.
    pub actions: Option<u64>,
    /// Remaining worker slots, when capped.
    pub workers: Option<u64>,
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

/// Result of a quota headroom check.
#[derive(Debug, Clone, Serialize)]
pub struct TenantHeadroom {
    /// Whether the requested amount fits under the configured cap.
    pub allowed: bool,
    /// Tenant id checked.
    pub tenant_id: String,
    /// Resource kind checked.
    pub kind: String,
    /// Current used amount.
    pub used: u64,
    /// Requested additional amount.
    pub requested: u64,
    /// Configured limit, if any.
    pub limit: Option<u64>,
    /// Human-readable denial reason when not allowed.
    pub reason: Option<String>,
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

    /// Check whether a resource increment fits under the configured quota.
    pub async fn has_headroom_for(
        &self,
        tenant_id: &str,
        kind: TenantQuotaKind,
        amount: u64,
        now: DateTime<Utc>,
    ) -> ArgoResult<TenantHeadroom> {
        let summary = self.summary(tenant_id, now).await?;
        let (used, limit, label) = match kind {
            TenantQuotaKind::UsdMicros => (
                summary.usage.used_usd_micros,
                summary.quota.as_ref().and_then(|q| q.max_usd_micros),
                "max_usd",
            ),
            TenantQuotaKind::Actions => (
                summary.usage.used_actions,
                summary.quota.as_ref().and_then(|q| q.max_actions),
                "max_actions",
            ),
            TenantQuotaKind::Workers => (
                summary.usage.used_workers,
                summary.quota.as_ref().and_then(|q| q.max_workers),
                "max_workers",
            ),
        };
        let allowed = match limit {
            Some(cap) => used.saturating_add(amount) <= cap,
            None => true,
        };
        let reason = if allowed {
            None
        } else {
            Some(format!(
                "tenant_quota:{label} exceeded for {tenant_id}: used {used} + requested {amount} > limit {}",
                limit.unwrap_or(0)
            ))
        };
        Ok(TenantHeadroom {
            allowed,
            tenant_id: tenant_id.to_string(),
            kind: label.to_string(),
            used,
            requested: amount,
            limit,
            reason,
        })
    }

    /// Hard-enforce a quota gate. Returns `Exceeded` when
    /// `current + delta > limit`, `Ok(())` otherwise. Read-only —
    /// `record_usage` (via `worker_budget` inserts) remains the
    /// write path. Returns `Internal` if the summary read fails;
    /// the dispatcher decides whether to fail-open.
    pub async fn check_or_reject(
        &self,
        tenant_id: &str,
        kind: QuotaActionKind,
        delta: u64,
        now: DateTime<Utc>,
    ) -> Result<(), QuotaError> {
        let summary = self
            .summary(tenant_id, now)
            .await
            .map_err(|e| QuotaError::Internal(e.to_string()))?;
        let (current, limit) = kind.usage_pair(&summary);
        let Some(limit) = limit else { return Ok(()) };
        if current.saturating_add(delta) > limit {
            Err(QuotaError::Exceeded {
                kind: kind.as_str().to_string(),
                current,
                limit,
            })
        } else {
            Ok(())
        }
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

    async fn usage(
        &self,
        tenant_id: &str,
        period: QuotaPeriod,
        now: DateTime<Utc>,
    ) -> ArgoResult<TenantUsage> {
        let start = period_start(period, now).timestamp();
        let (used_usd, used_actions) = self.budget_usage(tenant_id, start).await?;
        let used_workers = self.worker_count(tenant_id).await?;
        Ok(TenantUsage {
            period,
            period_start: start,
            used_usd_micros: used_usd,
            used_usd: micros_to_usd(used_usd),
            used_actions,
            used_workers,
        })
    }

    async fn budget_usage(&self, tenant_id: &str, start: i64) -> ArgoResult<(u64, u64)> {
        if !table_exists(self.pool, "worker").await?
            || !table_exists(self.pool, "worker_budget").await?
        {
            return Ok((0, 0));
        }
        let row = sqlx::query(
            "SELECT COALESCE(SUM(b.usd_micros), 0) AS usd, COUNT(*) AS actions \
             FROM worker_budget b JOIN worker w ON w.id = b.worker_id \
             WHERE w.tenant_id = ? AND b.charged_at >= ?",
        )
        .bind(tenant_id)
        .bind(start)
        .fetch_one(self.pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant quota usage: {e}")))?;
        Ok((
            u64_from_i64(row.get("usd")),
            u64_from_i64(row.get("actions")),
        ))
    }

    async fn worker_count(&self, tenant_id: &str) -> ArgoResult<u64> {
        if !table_exists(self.pool, "worker").await? {
            return Ok(0);
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(self.pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("tenant quota workers: {e}")))?;
        Ok(u64_from_i64(count))
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

fn remaining(quota: Option<&TenantQuota>, usage: &TenantUsage) -> TenantQuotaRemaining {
    TenantQuotaRemaining {
        usd_micros: quota
            .and_then(|q| q.max_usd_micros)
            .map(|cap| cap.saturating_sub(usage.used_usd_micros)),
        usd: quota
            .and_then(|q| q.max_usd_micros)
            .map(|cap| micros_to_usd(cap.saturating_sub(usage.used_usd_micros))),
        actions: quota
            .and_then(|q| q.max_actions)
            .map(|cap| cap.saturating_sub(usage.used_actions)),
        workers: quota
            .and_then(|q| q.max_workers)
            .map(|cap| cap.saturating_sub(usage.used_workers)),
    }
}

async fn table_exists(pool: &SqlitePool, table: &str) -> ArgoResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("table lookup {table}: {e}")))?;
    Ok(count > 0)
}

fn period_start(period: QuotaPeriod, now: DateTime<Utc>) -> DateTime<Utc> {
    let date = match period {
        QuotaPeriod::Daily => now.date_naive(),
        QuotaPeriod::Monthly => {
            let Some(date) = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1) else {
                return now;
            };
            date
        }
    };
    let Some(naive) = date.and_hms_opt(0, 0, 0) else {
        return now;
    };
    DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
}

fn db_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn u64_from_i64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

fn micros_to_usd(micros: u64) -> f64 { micros as f64 / 1_000_000.0 }

#[cfg(test)]
#[path = "tenant_quota_tests.rs"]
mod tests;
