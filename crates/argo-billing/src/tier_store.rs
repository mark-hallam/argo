//! Persistence helpers — `stripe_customer`, `stripe_event`,
//! and `workspace.tier` writeback.
//!
//! `argo-store`'s `tenant_query.rs` is consume-only for this brief, so
//! the writeback queries live here and run directly against
//! `SessionStore::pool()`. Schema is bootstrapped lazily via
//! [`ensure_billing_schema`] (mirroring
//! `argo_store::tenant_query::ensure_pricing_schema`) since the
//! versioned migration runner stops at slot 69.

use crate::webhook::SubscriptionEvent;
use argo_identity::tier::Tier;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};

const ENSURE_STRIPE_CUSTOMER_SQL: &str =
    include_str!("../../argo-store/migrations/109__stripe_customer.sql");
const ENSURE_STRIPE_EVENT_SQL: &str =
    include_str!("../../argo-store/migrations/110__stripe_event.sql");

/// Idempotent bootstrap for the two billing tables. Safe to call on
/// every webhook hit — the inner statements use `CREATE TABLE IF NOT
/// EXISTS`.
pub async fn ensure_billing_schema(store: &SessionStore) -> ArgoResult<()> {
    for (label, sql) in [
        ("stripe_customer", ENSURE_STRIPE_CUSTOMER_SQL),
        ("stripe_event", ENSURE_STRIPE_EVENT_SQL),
    ] {
        for stmt in split_sql_statements(sql) {
            sqlx::query(&stmt).execute(store.pool()).await.map_err(|e| {
                ArgoError::Provider(format!("ensure_billing_schema {label}: {e}"))
            })?;
        }
    }
    Ok(())
}

/// Strip `--` line comments and split on `;`. Mirrors
/// `argo_store::tenant_query::split_sql_statements`.
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

/// `true` when `event_id` is already in the `stripe_event` dedup table.
pub async fn has_processed_event(store: &SessionStore, event_id: &str) -> ArgoResult<bool> {
    ensure_billing_schema(store).await?;
    let row: Option<(String,)> =
        sqlx::query_as("SELECT event_id FROM stripe_event WHERE event_id = ?")
            .bind(event_id)
            .fetch_optional(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("stripe_event lookup: {e}")))?;
    Ok(row.is_some())
}

/// Insert (or ignore) the dedup row for `event_id`.
pub async fn record_event(
    store: &SessionStore,
    event_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<()> {
    ensure_billing_schema(store).await?;
    sqlx::query(
        "INSERT OR IGNORE INTO stripe_event (event_id, received_at) VALUES (?, ?)",
    )
    .bind(event_id)
    .bind(now.timestamp())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("stripe_event insert: {e}")))?;
    Ok(())
}

/// Resolve the Stripe customer id we previously stored for `tenant`,
/// if any. `None` when this tenant has never started Checkout.
pub async fn lookup_stripe_customer(
    store: &SessionStore,
    tenant: TenantId,
) -> ArgoResult<Option<String>> {
    ensure_billing_schema(store).await?;
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT stripe_customer_id FROM stripe_customer WHERE tenant_id = ?",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("stripe_customer lookup: {e}")))?;
    Ok(row.map(|(id,)| id))
}

/// Insert / update the `tenant_id` ↔ `stripe_customer_id` mapping.
pub async fn upsert_stripe_customer(
    store: &SessionStore,
    tenant: TenantId,
    customer_id: &str,
    subscription_id: Option<&str>,
    now: DateTime<Utc>,
) -> ArgoResult<()> {
    ensure_billing_schema(store).await?;
    sqlx::query(
        "INSERT INTO stripe_customer \
            (tenant_id, stripe_customer_id, subscription_id, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT(tenant_id) DO UPDATE SET \
             stripe_customer_id = excluded.stripe_customer_id, \
             subscription_id    = excluded.subscription_id, \
             updated_at         = excluded.updated_at",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(customer_id)
    .bind(subscription_id)
    .bind(now.timestamp())
    .bind(now.timestamp())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("stripe_customer upsert: {e}")))?;
    Ok(())
}

/// Write the resolved [`Tier`] back onto `workspace.tier`. Returns the
/// number of rows touched — `0` means the workspace row doesn't exist
/// yet (signup hasn't materialised a workspace for this tenant).
pub async fn set_workspace_tier(
    store: &SessionStore,
    tenant: TenantId,
    tier: Tier,
) -> ArgoResult<u64> {
    argo_store::tenant_query::ensure_pricing_schema(store).await?;
    let result = sqlx::query("UPDATE workspace SET tier = ? WHERE tenant_id = ?")
        .bind(tier.as_str())
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("workspace.tier update: {e}")))?;
    Ok(result.rows_affected())
}

/// Apply one [`SubscriptionEvent`] atomically: write the dedup row,
/// upsert the customer mapping, and persist the new tier. The caller
/// must check [`has_processed_event`] first.
pub async fn apply_subscription_event(
    store: &SessionStore,
    event: &SubscriptionEvent,
    now: DateTime<Utc>,
) -> ArgoResult<TierApplyOutcome> {
    record_event(store, &event.event_id, now).await?;
    upsert_stripe_customer(
        store,
        event.tenant_id,
        &event.customer_id,
        event.subscription_id.as_deref(),
        now,
    )
    .await?;
    let touched = set_workspace_tier(store, event.tenant_id, event.tier).await?;
    Ok(TierApplyOutcome {
        rows_touched: touched,
        past_due_warning: event.past_due_warning,
    })
}

/// Result of [`apply_subscription_event`] — surfaces whether the
/// gateway should emit a past-due email and whether the underlying
/// workspace row even existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierApplyOutcome {
    /// Number of `workspace` rows updated. `0` is a soft warning
    /// (subscription event arrived before the workspace was
    /// provisioned).
    pub rows_touched: u64,
    /// Whether the gateway should also surface a past-due warning to
    /// the operator.
    pub past_due_warning: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fresh_store() -> SessionStore {
        let store = SessionStore::open(":memory:").await.expect("memory store");
        store.migrate().await.expect("migrate");
        store
    }

    #[tokio::test]
    async fn idempotent_event_dedup() {
        let store = fresh_store().await;
        let now = Utc::now();
        assert!(!has_processed_event(&store, "evt_1").await.expect("query"));
        record_event(&store, "evt_1", now).await.expect("insert");
        assert!(has_processed_event(&store, "evt_1").await.expect("query"));
        // Second insert is a no-op — does not error.
        record_event(&store, "evt_1", now).await.expect("re-insert");
    }

    #[tokio::test]
    async fn customer_upsert_round_trip() {
        let store = fresh_store().await;
        let tenant = TenantId::new();
        let now = Utc::now();
        assert_eq!(lookup_stripe_customer(&store, tenant).await.expect("lookup"), None);
        upsert_stripe_customer(&store, tenant, "cus_a", Some("sub_a"), now)
            .await
            .expect("insert");
        assert_eq!(
            lookup_stripe_customer(&store, tenant).await.expect("re-lookup"),
            Some("cus_a".to_string())
        );
        upsert_stripe_customer(&store, tenant, "cus_b", Some("sub_b"), now)
            .await
            .expect("update");
        assert_eq!(
            lookup_stripe_customer(&store, tenant).await.expect("post-update"),
            Some("cus_b".to_string())
        );
    }
}
