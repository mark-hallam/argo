//! Tenant inspection and id-rename SQL helpers.
//!
//! `show_tenant` and `rename_tenant` live in
//! [`crate::tenant_query_inspect`] (split out for the 500-LOC ceiling)
//! and are re-exported here so external callers keep importing from
//! `argo_store::tenant_query::*`.

use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::SessionStore;

pub use crate::tenant_query_inspect::{rename_tenant, show_tenant};

const ENSURE_TIER_COLUMN_SQL: &str =
    include_str!("../migrations/104__tenant_tier.sql");
const ENSURE_TENANT_USAGE_SQL: &str =
    include_str!("../migrations/105__tenant_usage.sql");
const ENSURE_TENANT_TABLE_SQL: &str = include_str!("../migrations/106__tenant.sql");
const ENSURE_USER_ACCOUNT_SQL: &str = include_str!("../migrations/107__user_account.sql");
const ENSURE_USER_TENANT_SQL: &str = include_str!("../migrations/108__user_tenant.sql");

pub(crate) const BLOB_TENANT_TABLES: &[&str] = &[
    "agent",
    "memory_entry",
    "role",
    "role_binding",
    "role_duty",
    "session",
    "shared_team_memory",
    "spend_ledger",
    "worker_budget",
    "workspace",
];

pub(crate) const TEXT_TENANT_TABLES: &[&str] = &[
    "duty",
    "handoff_routing_rule",
    "org_member",
    "org_unit",
    "persona",
    "roster",
    "worker",
];

/// List tenants newest-first with worker and spend stats.
pub async fn list_tenants_with_stats(
    store: &SessionStore,
    include_archived: bool,
    now: DateTime<Utc>,
) -> ArgoResult<Vec<Value>> {
    let mut rows = Vec::new();
    for tenant in collect_tenants(store).await?.into_values() {
        let archived_at = workspace_i64(store, tenant, "archived_at").await?;
        if archived_at.is_some() && !include_archived {
            continue;
        }
        let created_at = tenant_created_at(store, tenant).await?;
        let today = spend_since(store, tenant, day_start_micros(now)).await?;
        let month = spend_since(store, tenant, month_start_micros(now)).await?;
        rows.push(json!({
            "id": tenant.to_string(),
            "name": workspace_text(store, tenant, "name").await?,
            "owner_email": workspace_text(store, tenant, "owner_email").await?,
            "created_at": created_at,
            "created_at_rfc3339": created_at.and_then(format_epoch_auto),
            "archived_at": archived_at,
            "archived_at_rfc3339": archived_at.and_then(format_epoch_auto),
            "worker_count": count_text(store, "worker", tenant).await?,
            "spend_today_usd_micros": today,
            "spend_this_month_usd_micros": month,
            "spend_today": micros_to_usd(today),
            "spend_this_month": micros_to_usd(month),
        }));
    }
    rows.sort_by(|a, b| {
        row_sort_key(b).cmp(&row_sort_key(a)).then_with(|| {
            a["id"]
                .as_str()
                .unwrap_or_default()
                .cmp(b["id"].as_str().unwrap_or_default())
        })
    });
    Ok(rows)
}

pub(crate) async fn collect_tenants(
    store: &SessionStore,
) -> ArgoResult<BTreeMap<String, TenantId>> {
    let mut out = BTreeMap::new();
    for table in BLOB_TENANT_TABLES {
        if has_column(store, table, "tenant_id").await? {
            // audit-allow: identifier substitution
            let sql = format!("SELECT DISTINCT tenant_id FROM {table} WHERE tenant_id IS NOT NULL");
            for row in query_all(store, &sql, table).await? {
                let raw: Vec<u8> = row.get("tenant_id");
                if let Ok(uuid) = Uuid::from_slice(&raw) {
                    let tenant = TenantId(uuid);
                    out.insert(tenant.to_string(), tenant);
                }
            }
        }
    }
    for table in TEXT_TENANT_TABLES {
        if has_column(store, table, "tenant_id").await? {
            // audit-allow: identifier substitution
            let sql = format!("SELECT DISTINCT tenant_id FROM {table} WHERE tenant_id IS NOT NULL");
            for row in query_all(store, &sql, table).await? {
                let raw: String = row.get("tenant_id");
                if let Ok(uuid) = Uuid::parse_str(&raw) {
                    let tenant = TenantId(uuid);
                    out.insert(tenant.to_string(), tenant);
                }
            }
        }
    }
    Ok(out)
}

async fn query_all(
    store: &SessionStore,
    sql: &str,
    label: &str,
) -> ArgoResult<Vec<sqlx::sqlite::SqliteRow>> {
    sqlx::query(sql)
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant query {label}: {e}")))
}

async fn tenant_created_at(store: &SessionStore, tenant: TenantId) -> ArgoResult<Option<i64>> {
    let mut values = Vec::new();
    values.push(min_blob(store, "workspace", "created_at", tenant).await?);
    values.push(min_blob(store, "session", "started_at", tenant).await?);
    values.push(min_text(store, "worker", "created_at", tenant).await?);
    Ok(values
        .into_iter()
        .flatten()
        .min_by_key(|v| normalize_epoch_micros(*v)))
}

pub(crate) async fn workspace_text(
    store: &SessionStore,
    tenant: TenantId,
    column: &str,
) -> ArgoResult<Option<String>> {
    if !has_column(store, "workspace", column).await? {
        return Ok(None);
    }
    // audit-allow: identifier substitution
    let sql = format!(
        "SELECT {column} FROM workspace WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1"
    );
    let row = sqlx::query(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant workspace {column}: {e}")))?;
    Ok(row.and_then(|row| row.get(column)))
}

async fn workspace_i64(
    store: &SessionStore,
    tenant: TenantId,
    column: &str,
) -> ArgoResult<Option<i64>> {
    if !has_column(store, "workspace", column).await? {
        return Ok(None);
    }
    // audit-allow: identifier substitution
    let sql = format!(
        "SELECT {column} FROM workspace WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1"
    );
    let row = sqlx::query(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant workspace {column}: {e}")))?;
    Ok(row.and_then(|row| row.get(column)))
}

pub(crate) async fn count_blob(
    store: &SessionStore,
    table: &str,
    tenant: TenantId,
) -> ArgoResult<i64> {
    if !has_column(store, table, "tenant_id").await? {
        return Ok(0);
    }
    // audit-allow: identifier substitution
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant count {table}: {e}")))
}

pub(crate) async fn count_text(
    store: &SessionStore,
    table: &str,
    tenant: TenantId,
) -> ArgoResult<i64> {
    if !has_column(store, table, "tenant_id").await? {
        return Ok(0);
    }
    // audit-allow: identifier substitution
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.to_string())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant count {table}: {e}")))
}

async fn min_blob(
    store: &SessionStore,
    table: &str,
    column: &str,
    tenant: TenantId,
) -> ArgoResult<Option<i64>> {
    if !has_column(store, table, "tenant_id").await? || !has_column(store, table, column).await? {
        return Ok(None);
    }
    // audit-allow: identifier substitution
    let sql = format!("SELECT MIN({column}) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant min {table}: {e}")))
}

async fn min_text(
    store: &SessionStore,
    table: &str,
    column: &str,
    tenant: TenantId,
) -> ArgoResult<Option<i64>> {
    if !has_column(store, table, "tenant_id").await? || !has_column(store, table, column).await? {
        return Ok(None);
    }
    // audit-allow: identifier substitution
    let sql = format!("SELECT MIN({column}) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.to_string())
        .fetch_one(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("tenant min {table}: {e}")))
}

async fn spend_since(store: &SessionStore, tenant: TenantId, since: i64) -> ArgoResult<i64> {
    if !has_column(store, "spend_ledger", "tenant_id").await? {
        return Ok(0);
    }
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \
         WHERE tenant_id = ? AND charged_at_micros >= ?",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(since)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant spend: {e}")))
}

pub(crate) async fn table_exists(store: &SessionStore, table: &str) -> ArgoResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("table exists {table}: {e}")))?;
    Ok(count > 0)
}

pub(crate) async fn has_column(
    store: &SessionStore,
    table: &str,
    column: &str,
) -> ArgoResult<bool> {
    if !table_exists(store, table).await? {
        return Ok(false);
    }
    // audit-allow: identifier substitution after sqlite_master table probe
    let rows = sqlx::query(&format!("PRAGMA table_info({table})"))
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("table info {table}: {e}")))?;
    Ok(rows
        .iter()
        .any(|row| row.get::<String, _>("name") == column))
}

fn row_sort_key(row: &Value) -> i64 {
    normalize_epoch_micros(row["created_at"].as_i64().unwrap_or_default())
}

fn day_start_micros(now: DateTime<Utc>) -> i64 {
    now.date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_utc().timestamp_micros())
        .unwrap_or_else(|| now.timestamp_micros())
}

fn month_start_micros(now: DateTime<Utc>) -> i64 {
    Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .map(|dt| dt.timestamp_micros())
        .unwrap_or_else(|| now.timestamp_micros())
}

fn normalize_epoch_micros(value: i64) -> i64 {
    let abs = value.unsigned_abs();
    if abs < 20_000_000_000 {
        value.saturating_mul(1_000_000)
    } else if abs < 20_000_000_000_000 {
        value.saturating_mul(1_000)
    } else {
        value
    }
}

pub(crate) fn format_epoch_auto(value: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_micros(normalize_epoch_micros(value)).map(|dt| dt.to_rfc3339())
}

fn micros_to_usd(micros: i64) -> String {
    format!("${:.4}", micros as f64 / 1_000_000.0)
}

/// claude-pricing-plans-config — idempotently apply the additive
/// `workspace.tier` column + `tenant_usage` table. Mirrors the
/// `ensure_team_roster_columns` PRAGMA-detect-then-ALTER pattern; the
/// versioned migration runner only registers slots ≤ 69, so newer
/// additive schema is bootstrapped lazily on first access.
pub async fn ensure_pricing_schema(store: &SessionStore) -> ArgoResult<()> {
    if !has_column(store, "workspace", "tier").await? && table_exists(store, "workspace").await? {
        for stmt in split_sql_statements(ENSURE_TIER_COLUMN_SQL) {
            sqlx::query(&stmt)
                .execute(store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("ensure_pricing_schema tier: {e}")))?;
        }
    }
    for stmt in split_sql_statements(ENSURE_TENANT_USAGE_SQL) {
        sqlx::query(&stmt)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("ensure_pricing_schema usage: {e}")))?;
    }
    Ok(())
}

/// Strip line-prefix `--` comments and split the remainder on `;`,
/// yielding non-empty trimmed statements. Robust to semicolons
/// appearing inside comments (which a naïve `split(';')` mis-handles).
fn split_sql_statements(sql: &str) -> Vec<String> {
    let stripped: String = sql
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    stripped
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Pricing tier currently assigned to `tenant`. Returns `"free"` when
/// the tenant has no workspace row yet or the schema is pre-tier.
pub async fn tenant_tier(store: &SessionStore, tenant: TenantId) -> ArgoResult<String> {
    ensure_pricing_schema(store).await?;
    Ok(workspace_text(store, tenant, "tier")
        .await?
        .unwrap_or_else(|| "free".to_string()))
}

/// Typed alias for [`tenant_tier`]. Caller-side parsing into
/// `argo_identity::Tier` would create a `argo-store → argo-identity`
/// crate cycle (argo-identity already depends on argo-store), so the
/// store-side helper continues to return the lowercase tier slug
/// (`"free"` / `"pro"` / `"enterprise"`). Gateway-side callers parse
/// via `Tier::from_str`. Kept as a separate symbol so consumers can
/// import `get_tenant_tier` per the runtime-enforcement brief spec
/// without losing the schema-lazy `ensure_pricing_schema` path.
pub async fn get_tenant_tier(store: &SessionStore, tenant: TenantId) -> ArgoResult<String> {
    tenant_tier(store, tenant).await
}

/// Period-to-date action count for `tenant`. Returns `(used_actions,
/// period_start_unix_seconds)`. `period_start` falls back to the
/// current calendar-month boundary when no row exists yet.
pub async fn tenant_usage_for_period(
    store: &SessionStore,
    tenant: TenantId,
    now: DateTime<Utc>,
) -> ArgoResult<(u64, i64)> {
    ensure_pricing_schema(store).await?;
    let period_start = month_start_micros(now) / 1_000_000;
    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT used_actions, period_start FROM tenant_usage WHERE tenant_id = ?",
    )
    .bind(tenant.to_string())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant_usage read: {e}")))?;
    let (used, start) = match row {
        Some((used, start)) if start >= period_start => (u64::try_from(used).unwrap_or(0), start),
        _ => (0, period_start),
    };
    Ok((used, start))
}

/// Atomically bump `used_actions` for `tenant` within the current
/// calendar-month window. Resets the counter to `delta` when the
/// stored period is older than the current one.
pub async fn increment_tenant_usage(
    store: &SessionStore,
    tenant: TenantId,
    delta: u64,
    now: DateTime<Utc>,
) -> ArgoResult<u64> {
    ensure_pricing_schema(store).await?;
    let period_start = month_start_micros(now) / 1_000_000;
    let (current_used, current_start) = tenant_usage_for_period(store, tenant, now).await?;
    let next = if current_start >= period_start {
        current_used.saturating_add(delta)
    } else {
        delta
    };
    let next_i64 = i64::try_from(next).unwrap_or(i64::MAX);
    sqlx::query(
        "INSERT INTO tenant_usage (tenant_id, period_start, used_actions, updated_at) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(tenant_id) DO UPDATE SET \
             period_start = excluded.period_start, \
             used_actions = excluded.used_actions, \
             updated_at = excluded.updated_at",
    )
    .bind(tenant.to_string())
    .bind(period_start)
    .bind(next_i64)
    .bind(now.timestamp())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tenant_usage write: {e}")))?;
    Ok(next)
}

/// mission-tenant-substrate-tables — lazily bootstrap the substrate
/// tables (`tenant`, `user_account`, `user_tenant`). Mirrors
/// `ensure_pricing_schema`'s `IF NOT EXISTS` pattern; the versioned
/// migration runner caps at slot 69, so slots 106–108 are applied here
/// the first time any substrate query runs.
pub async fn ensure_tenant_substrate_schema(store: &SessionStore) -> ArgoResult<()> {
    for (label, sql) in [
        ("tenant", ENSURE_TENANT_TABLE_SQL),
        ("user_account", ENSURE_USER_ACCOUNT_SQL),
        ("user_tenant", ENSURE_USER_TENANT_SQL),
    ] {
        for stmt in split_sql_statements(sql) {
            sqlx::query(&stmt).execute(store.pool()).await.map_err(|e| {
                ArgoError::Provider(format!("ensure_tenant_substrate_schema {label}: {e}"))
            })?;
        }
    }
    Ok(())
}
