//! Usage accounting and headroom checks for tenant_quota.
//!
//! Split out of `tenant_quota.rs` for the 500-LOC budget. The
//! public CRUD surface and `TenantQuotaStore` definition live in
//! `tenant_quota.rs`; this file owns the read-side aggregation that
//! consumes `worker_budget` rows and the headroom-check methods
//! that consume the resulting summaries.

use crate::tenant_quota::{QuotaPeriod, TenantQuota, TenantQuotaStore};
use crate::tenant_quota_kinds::{QuotaActionKind, QuotaError, TenantQuotaKind};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Datelike, Utc};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

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

impl<'a> TenantQuotaStore<'a> {
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
}

pub(crate) async fn usage(
    pool: &SqlitePool,
    tenant_id: &str,
    period: QuotaPeriod,
    now: DateTime<Utc>,
) -> ArgoResult<TenantUsage> {
    let start = period_start(period, now).timestamp();
    let (used_usd, used_actions) = budget_usage(pool, tenant_id, start).await?;
    let used_workers = worker_count(pool, tenant_id).await?;
    Ok(TenantUsage {
        period,
        period_start: start,
        used_usd_micros: used_usd,
        used_usd: micros_to_usd(used_usd),
        used_actions,
        used_workers,
    })
}

async fn budget_usage(pool: &SqlitePool, tenant_id: &str, start: i64) -> ArgoResult<(u64, u64)> {
    if !table_exists(pool, "worker").await? || !table_exists(pool, "worker_budget").await? {
        return Ok((0, 0));
    }
    let row = sqlx::query(
        "SELECT COALESCE(SUM(b.usd_micros), 0) AS usd, COUNT(*) AS actions \
         FROM worker_budget b JOIN worker w ON w.id = b.worker_id \
         WHERE w.tenant_id = ? AND b.charged_at >= ?",
    )
    .bind(tenant_id)
    .bind(start)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant quota usage: {e}")))?;
    Ok((
        u64_from_i64(row.get("usd")),
        u64_from_i64(row.get("actions")),
    ))
}

async fn worker_count(pool: &SqlitePool, tenant_id: &str) -> ArgoResult<u64> {
    if !table_exists(pool, "worker").await? {
        return Ok(0);
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worker WHERE tenant_id = ?")
        .bind(tenant_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant quota workers: {e}")))?;
    Ok(u64_from_i64(count))
}

pub(crate) fn remaining(
    quota: Option<&TenantQuota>,
    usage: &TenantUsage,
) -> TenantQuotaRemaining {
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

pub(crate) async fn table_exists(pool: &SqlitePool, table: &str) -> ArgoResult<bool> {
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

pub(crate) fn u64_from_i64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

pub(crate) fn micros_to_usd(micros: u64) -> f64 {
    micros as f64 / 1_000_000.0
}
