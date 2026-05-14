//! `argo-store::worker_budget_write` — write-side helper for the
//! per-worker spend cap surface (`Stream claude-worker-budget-cli`).
//!
//! The cap is persisted on the existing `worker.auto_approval_json`
//! column; the daily field maps onto
//! [`argo_identity::AutoApprovalPolicy::daily_budget_usd_micros`] so
//! the live auto-approval gate honours it without any new wiring. The
//! monthly cap rides alongside as `monthly_budget_usd_micros` because
//! the policy struct does not yet model it; the value survives in
//! the JSON column for `argo worker budget show` to render but the
//! gating layer is free to ignore it (read-side reuses the existing
//! `worker_budget` ledger for period-to-date spend).
//!
//! All mutation is a read-mutate-write over the JSON value; we never
//! round-trip through `AutoApprovalPolicy` here so unknown fields are
//! preserved.

use crate::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Pool, Sqlite};

const DAILY_KEY: &str = "daily_budget_usd_micros";
const MONTHLY_KEY: &str = "monthly_budget_usd_micros";

/// Caps currently persisted on the worker row. `None` means the cap
/// is not set (delegates to tenant default at gate time).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerBudgetCaps {
    /// Daily ceiling in USD micros (1 USD = 1_000_000).
    pub daily_micros: Option<u64>,
    /// Monthly ceiling in USD micros.
    pub monthly_micros: Option<u64>,
}

/// Render-friendly bundle returned by [`summary`] — caps plus the
/// rolling spend totals from the `worker_budget` ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerBudgetSummary {
    /// Worker id the row was queried for (echoed for JSON callers).
    pub worker_id: String,
    /// Caps as currently persisted.
    pub caps: WorkerBudgetCaps,
    /// Charged-today total in USD micros.
    pub today_micros: u64,
    /// Charged-this-month total in USD micros (UTC calendar).
    pub this_month_micros: u64,
}

/// Persist the cap fields onto the worker's `auto_approval_json`
/// column. `None` for a dimension leaves whatever was there alone;
/// `Some(_)` overwrites. Writing the same value twice is a no-op
/// from the operator's perspective (UPDATE is idempotent on SQLite).
pub async fn set_cap(
    store: &SessionStore,
    worker_id: &str,
    daily_micros: Option<u64>,
    monthly_micros: Option<u64>,
) -> ArgoResult<()> {
    let mut value = load_policy_value(store.pool(), worker_id).await?;
    let map = ensure_object(&mut value, worker_id)?;
    if let Some(daily) = daily_micros {
        map.insert(DAILY_KEY.into(), json!(daily));
    }
    if let Some(monthly) = monthly_micros {
        map.insert(MONTHLY_KEY.into(), json!(monthly));
    }
    write_policy_value(store.pool(), worker_id, &value).await
}

/// Drop both the daily and monthly cap fields from the worker's
/// policy JSON. Other auto-approval settings are preserved.
pub async fn clear_cap(store: &SessionStore, worker_id: &str) -> ArgoResult<()> {
    let mut value = load_policy_value(store.pool(), worker_id).await?;
    let map = ensure_object(&mut value, worker_id)?;
    let mut changed = false;
    if map.remove(DAILY_KEY).is_some() {
        changed = true;
    }
    if map.remove(MONTHLY_KEY).is_some() {
        changed = true;
    }
    if !changed {
        return Ok(());
    }
    write_policy_value(store.pool(), worker_id, &value).await
}

/// Read the persisted caps + the rolling charged-today /
/// charged-this-month totals from the `worker_budget` ledger.
pub async fn summary(
    store: &SessionStore,
    worker_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<WorkerBudgetSummary> {
    let value = load_policy_value(store.pool(), worker_id).await?;
    let caps = caps_from_value(&value);
    let (today_micros, this_month_micros) = load_spend_totals(store.pool(), worker_id, now).await?;
    Ok(WorkerBudgetSummary {
        worker_id: worker_id.to_string(),
        caps,
        today_micros,
        this_month_micros,
    })
}

async fn load_policy_value(pool: &Pool<Sqlite>, worker_id: &str) -> ArgoResult<Value> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT auto_approval_json FROM worker WHERE id = ? LIMIT 1")
            .bind(worker_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| ArgoError::Io(format!("read worker policy: {e}")))?;
    let raw = row
        .map(|(s,)| s)
        .ok_or_else(|| ArgoError::Provider(format!("worker {worker_id} not found")))?;
    if raw.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str::<Value>(&raw)
        .map_err(|e| ArgoError::Provider(format!("worker {worker_id} policy is not JSON: {e}")))
}

async fn write_policy_value(
    pool: &Pool<Sqlite>,
    worker_id: &str,
    value: &Value,
) -> ArgoResult<()> {
    let serialised = serde_json::to_string(value)
        .map_err(|e| ArgoError::Provider(format!("serialise worker policy: {e}")))?;
    let result = sqlx::query("UPDATE worker SET auto_approval_json = ? WHERE id = ?")
        .bind(&serialised)
        .bind(worker_id)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("update worker policy: {e}")))?;
    if result.rows_affected() == 0 {
        return Err(ArgoError::Provider(format!("worker {worker_id} not found")));
    }
    Ok(())
}

fn ensure_object<'a>(
    value: &'a mut Value,
    worker_id: &str,
) -> ArgoResult<&'a mut serde_json::Map<String, Value>> {
    if !value.is_object() {
        *value = json!({});
    }
    value
        .as_object_mut()
        .ok_or_else(|| ArgoError::Provider(format!("worker {worker_id} policy is not an object")))
}

fn caps_from_value(value: &Value) -> WorkerBudgetCaps {
    let map = match value.as_object() {
        Some(m) => m,
        None => return WorkerBudgetCaps::default(),
    };
    WorkerBudgetCaps {
        daily_micros: read_u64(map, DAILY_KEY),
        monthly_micros: read_u64(map, MONTHLY_KEY),
    }
}

fn read_u64(map: &serde_json::Map<String, Value>, key: &str) -> Option<u64> {
    map.get(key).and_then(|v| match v {
        Value::Null => None,
        Value::Number(n) => n.as_u64(),
        _ => None,
    })
}

async fn load_spend_totals(
    pool: &Pool<Sqlite>,
    worker_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<(u64, u64)> {
    let day_start = day_start_seconds(now);
    let month_start = month_start_seconds(now);
    let today: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \
         WHERE worker_id = ? AND charged_at >= ?",
    )
    .bind(worker_id)
    .bind(day_start)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_budget today sum: {e}")))?;
    let month: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \
         WHERE worker_id = ? AND charged_at >= ?",
    )
    .bind(worker_id)
    .bind(month_start)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker_budget month sum: {e}")))?;
    Ok((
        u64::try_from(today).unwrap_or(0),
        u64::try_from(month).unwrap_or(0),
    ))
}

fn day_start_seconds(now: DateTime<Utc>) -> i64 {
    Utc.with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()
        .map(|d| d.timestamp())
        .unwrap_or(0)
}

fn month_start_seconds(now: DateTime<Utc>) -> i64 {
    Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .map(|d| d.timestamp())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fresh_store() -> SessionStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    async fn seed_worker(pool: &Pool<Sqlite>, id: &str, policy: &str) {
        sqlx::query(
            "INSERT INTO worker (id, tenant_id, vertical_id, persona_id, role_id, \
             memory_scope, auto_approval_json, channels_json, created_at) \
             VALUES (?, 'tenant-x', 'vert-x', 'persona-x', 'role-x', 'scope-x', ?, '[]', 0)",
        )
        .bind(id)
        .bind(policy)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn seed_charge(pool: &Pool<Sqlite>, worker_id: &str, micros: i64, charged_at: i64) {
        sqlx::query(
            "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) \
             VALUES (x'00', ?, ?, ?, ?)",
        )
        .bind(worker_id)
        .bind(format!("act-{charged_at}"))
        .bind(micros)
        .bind(charged_at)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn read_policy(pool: &Pool<Sqlite>, worker_id: &str) -> Value {
        let raw: String = sqlx::query_scalar("SELECT auto_approval_json FROM worker WHERE id = ?")
            .bind(worker_id)
            .fetch_one(pool)
            .await
            .unwrap();
        serde_json::from_str(&raw).unwrap()
    }

    #[tokio::test]
    async fn set_cap_writes_both_dimensions() {
        let store = fresh_store().await;
        seed_worker(store.pool(), "w-1", "{\"enabled\":false}").await;
        set_cap(&store, "w-1", Some(5_000_000), Some(100_000_000))
            .await
            .unwrap();
        let policy = read_policy(store.pool(), "w-1").await;
        assert_eq!(policy[DAILY_KEY], 5_000_000);
        assert_eq!(policy[MONTHLY_KEY], 100_000_000);
        assert_eq!(policy["enabled"], false);
    }

    #[tokio::test]
    async fn set_cap_zero_is_allowed() {
        let store = fresh_store().await;
        seed_worker(store.pool(), "w-2", "{\"enabled\":false}").await;
        set_cap(&store, "w-2", Some(0), None).await.unwrap();
        let policy = read_policy(store.pool(), "w-2").await;
        assert_eq!(policy[DAILY_KEY], 0);
        assert!(policy.get(MONTHLY_KEY).is_none());
    }

    #[tokio::test]
    async fn set_cap_is_idempotent() {
        let store = fresh_store().await;
        seed_worker(store.pool(), "w-3", "{\"enabled\":true}").await;
        set_cap(&store, "w-3", Some(7_500_000), Some(50_000_000))
            .await
            .unwrap();
        set_cap(&store, "w-3", Some(7_500_000), Some(50_000_000))
            .await
            .unwrap();
        let policy = read_policy(store.pool(), "w-3").await;
        assert_eq!(policy[DAILY_KEY], 7_500_000);
        assert_eq!(policy[MONTHLY_KEY], 50_000_000);
    }

    #[tokio::test]
    async fn set_cap_partial_update_preserves_other_dimension() {
        let store = fresh_store().await;
        seed_worker(
            store.pool(),
            "w-4",
            "{\"enabled\":false,\"daily_budget_usd_micros\":1000,\"monthly_budget_usd_micros\":2000}",
        )
        .await;
        set_cap(&store, "w-4", None, Some(9_000)).await.unwrap();
        let policy = read_policy(store.pool(), "w-4").await;
        assert_eq!(policy[DAILY_KEY], 1000);
        assert_eq!(policy[MONTHLY_KEY], 9000);
    }

    #[tokio::test]
    async fn clear_cap_removes_both_fields() {
        let store = fresh_store().await;
        seed_worker(
            store.pool(),
            "w-5",
            "{\"enabled\":true,\"daily_budget_usd_micros\":1,\"monthly_budget_usd_micros\":2}",
        )
        .await;
        clear_cap(&store, "w-5").await.unwrap();
        let policy = read_policy(store.pool(), "w-5").await;
        assert!(policy.get(DAILY_KEY).is_none());
        assert!(policy.get(MONTHLY_KEY).is_none());
        assert_eq!(policy["enabled"], true);
    }

    #[tokio::test]
    async fn missing_worker_errors() {
        let store = fresh_store().await;
        let err = set_cap(&store, "ghost", Some(1), None).await.unwrap_err();
        assert!(err.to_string().contains("ghost"));
    }

    #[tokio::test]
    async fn summary_reports_caps_and_spend() {
        let store = fresh_store().await;
        seed_worker(
            store.pool(),
            "w-6",
            "{\"enabled\":true,\"daily_budget_usd_micros\":7,\"monthly_budget_usd_micros\":42}",
        )
        .await;
        let now = Utc::now();
        let day_start = day_start_seconds(now);
        let month_start = month_start_seconds(now);
        seed_charge(store.pool(), "w-6", 1_000, day_start + 60).await;
        seed_charge(store.pool(), "w-6", 4_000, day_start + 120).await;
        // earlier in month, before today
        if month_start + 30 < day_start {
            seed_charge(store.pool(), "w-6", 500, month_start + 30).await;
        }
        let s = summary(&store, "w-6", now).await.unwrap();
        assert_eq!(s.caps.daily_micros, Some(7));
        assert_eq!(s.caps.monthly_micros, Some(42));
        assert_eq!(s.today_micros, 5_000);
        let expected_month = if month_start + 30 < day_start {
            5_500
        } else {
            5_000
        };
        assert_eq!(s.this_month_micros, expected_month);
    }

    #[tokio::test]
    async fn summary_with_no_caps_or_spend() {
        let store = fresh_store().await;
        seed_worker(store.pool(), "w-7", "{\"enabled\":false}").await;
        let s = summary(&store, "w-7", Utc::now()).await.unwrap();
        assert_eq!(s.caps, WorkerBudgetCaps::default());
        assert_eq!(s.today_micros, 0);
        assert_eq!(s.this_month_micros, 0);
    }

    #[tokio::test]
    async fn read_u64_handles_null_and_garbage() {
        let v: Value = json!({ "a": 5_u64, "b": null, "c": "hi" });
        let map = v.as_object().unwrap();
        assert_eq!(read_u64(map, "a"), Some(5));
        assert_eq!(read_u64(map, "b"), None);
        assert_eq!(read_u64(map, "c"), None);
        assert_eq!(read_u64(map, "missing"), None);
    }
}
