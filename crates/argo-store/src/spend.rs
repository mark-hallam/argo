//! Spend-ledger queries (item C — hard $$$ cost ceilings).
//!
//! Sits on top of the v2 `spend_ledger` table. Callers feed
//! [`super::SessionStore::load_spend_counters`] into
//! [`argo_types::enforce_pre_spend`] before authorising a charge,
//! then [`super::SessionStore::record_spend`] after.
use argo_types::{ArgoError, SessionId, SpendCounters, TenantId};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use sqlx::{Pool, Sqlite};

pub(super) async fn record_spend(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    session_id: SessionId,
    charged_at_micros: i64,
    amount_usd_micros: u64,
    provider: &str,
    model: &str,
) -> Result<(), ArgoError> {
    sqlx::query(
        "INSERT INTO spend_ledger (tenant_id, session_id, charged_at_micros, \
         amount_usd_micros, provider, model) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(session_id.0.as_bytes().as_slice())
    .bind(charged_at_micros)
    .bind(i64::try_from(amount_usd_micros).unwrap_or(i64::MAX))
    .bind(provider)
    .bind(model)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("record_spend: {e}")))?;
    Ok(())
}

pub(super) async fn load_spend_counters(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    session_id: SessionId,
    now: DateTime<Utc>,
) -> Result<SpendCounters, ArgoError> {
    let day_start = day_start_micros(now);
    let month_start = month_start_micros(now);
    let session_used: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \
         WHERE tenant_id = ? AND session_id = ?",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(session_id.0.as_bytes().as_slice())
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("session sum: {e}")))?;
    let day_used: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \
         WHERE tenant_id = ? AND charged_at_micros >= ?",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(day_start)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("day sum: {e}")))?;
    let month_used: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \
         WHERE tenant_id = ? AND charged_at_micros >= ?",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(month_start)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("month sum: {e}")))?;
    Ok(SpendCounters {
        session_used: u64::try_from(session_used).unwrap_or(0),
        day_used: u64::try_from(day_used).unwrap_or(0),
        month_used: u64::try_from(month_used).unwrap_or(0),
    })
}

/// Global spend rollup — no tenant filter, just SUM(amount_usd_micros)
/// across the whole ledger for today (UTC), this month, and all-time.
/// Used by `argo costs show` and the TUI `$` overlay.
pub(super) async fn load_global_spend(
    pool: &Pool<Sqlite>,
    now: DateTime<Utc>,
) -> Result<GlobalSpend, ArgoError> {
    let day_start = day_start_micros(now);
    let month_start = month_start_micros(now);
    let today: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \
         WHERE charged_at_micros >= ?",
    )
    .bind(day_start)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("global today sum: {e}")))?;
    let this_month: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \
         WHERE charged_at_micros >= ?",
    )
    .bind(month_start)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("global month sum: {e}")))?;
    let all_time: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger")
            .fetch_one(pool)
            .await
            .map_err(|e| ArgoError::Provider(format!("global all-time sum: {e}")))?;
    Ok(GlobalSpend {
        today: u64::try_from(today).unwrap_or(0),
        this_month: u64::try_from(this_month).unwrap_or(0),
        all_time: u64::try_from(all_time).unwrap_or(0),
    })
}

/// Cross-tenant spend rollup snapshot.
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalSpend {
    /// Today's spend (UTC day boundary), in micros of USD.
    pub today: u64,
    /// Month-to-date spend (UTC), in micros of USD.
    pub this_month: u64,
    /// All-time total, in micros of USD.
    pub all_time: u64,
}

fn day_start_micros(now: DateTime<Utc>) -> i64 {
    Utc.with_ymd_and_hms(
        now.year_ce().1.try_into().unwrap_or(1),
        now.month(),
        now.day(),
        0,
        0,
        0,
    )
    .single()
    .map(|d| d.timestamp_micros())
    .unwrap_or(0)
}

fn month_start_micros(now: DateTime<Utc>) -> i64 {
    Utc.with_ymd_and_hms(
        now.year_ce().1.try_into().unwrap_or(1),
        now.month(),
        1,
        0,
        0,
        0,
    )
    .single()
    .map(|d| d.timestamp_micros())
    .unwrap_or(0)
}
