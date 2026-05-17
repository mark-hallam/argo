//! Dead-letter queue for Stripe webhook events that argo-billing
//! could not apply (Stream `codex-billing-webhook-resilience`). The
//! webhook handler writes a row once Stripe's own retry schedule is
//! exhausted; the operator `argo billing replay` CLI re-applies
//! events from `raw_payload` and stamps `resolved_at` on success.
//!
//! Schema lives in migration `112__billing_event_dead_letter.sql`
//! (versioned slot). [`ensure_dlq_schema`] bootstraps the table
//! lazily so out-of-band callers (tests, `argo billing dlq list` on
//! a fresh DB) work before any webhook hit.

use crate::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Utc};
use sqlx::Row;

const ENSURE_DLQ_SQL: &str =
    include_str!("../migrations/112__billing_event_dead_letter.sql");

/// One dead-letter row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadLetterEntry {
    /// Stripe event id (`evt_…`) — primary key.
    pub event_id: String,
    /// Stripe event type, e.g. `customer.subscription.updated`.
    pub event_type: String,
    /// Tenant the event resolves to, when the classifier could
    /// determine it. `None` for events whose metadata never carried a
    /// `tenant_id`.
    pub tenant_id: Option<TenantId>,
    /// Canonical JSON Stripe POSTed. Replay reads from here so we
    /// never re-call Stripe.
    pub raw_payload: String,
    /// Unix seconds at which the webhook gave up retrying.
    pub failed_at: i64,
    /// Short operator-readable failure reason from the apply path.
    pub failure_reason: String,
    /// Number of times the webhook re-attempted apply before giving
    /// up. Bumped on every Stripe redelivery that fails.
    pub retry_count: i64,
    /// Unix seconds at which a replay (or `dlq resolve`) cleared the
    /// row. `None` while the entry is still outstanding.
    pub resolved_at: Option<i64>,
}

/// Idempotent bootstrap for the DLQ table. Safe to call on every
/// webhook hit — `CREATE TABLE IF NOT EXISTS` semantics.
pub async fn ensure_dlq_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in split_sql_statements(ENSURE_DLQ_SQL) {
        sqlx::query(&stmt)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("ensure_dlq_schema: {e}")))?;
    }
    Ok(())
}

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

/// Insert a new dead-letter row (or bump `retry_count` + reason if
/// the event already failed before). `failed_at` is always updated to
/// `now`. Returns the final `retry_count` after the upsert.
pub async fn upsert_dead_letter(
    store: &SessionStore,
    event_id: &str,
    event_type: &str,
    tenant_id: Option<TenantId>,
    raw_payload: &str,
    now: DateTime<Utc>,
    failure_reason: &str,
) -> ArgoResult<i64> {
    ensure_dlq_schema(store).await?;
    let tenant_blob: Option<Vec<u8>> = tenant_id.map(|t| t.0.as_bytes().to_vec());
    sqlx::query(
        "INSERT INTO billing_event_dead_letter \
            (event_id, event_type, tenant_id, raw_payload, \
             failed_at, failure_reason, retry_count) \
         VALUES (?, ?, ?, ?, ?, ?, 1) \
         ON CONFLICT(event_id) DO UPDATE SET \
            failed_at = excluded.failed_at, \
            failure_reason = excluded.failure_reason, \
            retry_count = billing_event_dead_letter.retry_count + 1",
    )
    .bind(event_id)
    .bind(event_type)
    .bind(tenant_blob.as_deref())
    .bind(raw_payload)
    .bind(now.timestamp())
    .bind(failure_reason)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("dlq upsert: {e}")))?;
    let row = sqlx::query(
        "SELECT retry_count FROM billing_event_dead_letter WHERE event_id = ?",
    )
    .bind(event_id)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("dlq retry_count read: {e}")))?;
    let count: i64 = row
        .try_get::<i64, _>("retry_count")
        .map_err(|e| ArgoError::Provider(format!("dlq retry_count decode: {e}")))?;
    Ok(count)
}

/// Fetch one dead-letter row by `event_id`. `None` when there is no
/// matching row (already replayed, or never existed).
pub async fn fetch_dead_letter(
    store: &SessionStore,
    event_id: &str,
) -> ArgoResult<Option<DeadLetterEntry>> {
    ensure_dlq_schema(store).await?;
    let row = sqlx::query(
        "SELECT event_id, event_type, tenant_id, raw_payload, \
                failed_at, failure_reason, retry_count, resolved_at \
         FROM billing_event_dead_letter WHERE event_id = ?",
    )
    .bind(event_id)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("dlq fetch: {e}")))?;
    row.map(row_to_entry).transpose()
}

/// List dead-letter rows. Pass `include_resolved = false` to suppress
/// rows that already have `resolved_at`. `failed_since` filters on
/// `failed_at >= ?` (Unix seconds); `None` means no lower bound.
/// `limit` caps the result set — the CLI exposes this as `--limit`.
pub async fn list_dead_letters(
    store: &SessionStore,
    failed_since: Option<i64>,
    include_resolved: bool,
    limit: i64,
) -> ArgoResult<Vec<DeadLetterEntry>> {
    ensure_dlq_schema(store).await?;
    let since = failed_since.unwrap_or(i64::MIN);
    let rows = if include_resolved {
        sqlx::query(
            "SELECT event_id, event_type, tenant_id, raw_payload, \
                    failed_at, failure_reason, retry_count, resolved_at \
             FROM billing_event_dead_letter \
             WHERE failed_at >= ? \
             ORDER BY failed_at DESC LIMIT ?",
        )
        .bind(since)
        .bind(limit)
        .fetch_all(store.pool())
        .await
    } else {
        sqlx::query(
            "SELECT event_id, event_type, tenant_id, raw_payload, \
                    failed_at, failure_reason, retry_count, resolved_at \
             FROM billing_event_dead_letter \
             WHERE failed_at >= ? AND resolved_at IS NULL \
             ORDER BY failed_at DESC LIMIT ?",
        )
        .bind(since)
        .bind(limit)
        .fetch_all(store.pool())
        .await
    }
    .map_err(|e| ArgoError::Provider(format!("dlq list: {e}")))?;
    rows.into_iter().map(row_to_entry).collect()
}

/// Stamp `resolved_at = now` on `event_id`. Returns `true` when a row
/// was updated (operator can still resolve an already-resolved row;
/// it becomes a no-op and returns `false`).
pub async fn mark_resolved(
    store: &SessionStore,
    event_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<bool> {
    ensure_dlq_schema(store).await?;
    let result = sqlx::query(
        "UPDATE billing_event_dead_letter \
         SET resolved_at = ? \
         WHERE event_id = ? AND resolved_at IS NULL",
    )
    .bind(now.timestamp())
    .bind(event_id)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("dlq resolve: {e}")))?;
    Ok(result.rows_affected() > 0)
}

fn row_to_entry(row: sqlx::sqlite::SqliteRow) -> ArgoResult<DeadLetterEntry> {
    let event_id: String = row
        .try_get("event_id")
        .map_err(|e| ArgoError::Provider(format!("dlq event_id: {e}")))?;
    let event_type: String = row
        .try_get("event_type")
        .map_err(|e| ArgoError::Provider(format!("dlq event_type: {e}")))?;
    let tenant_bytes: Option<Vec<u8>> = row
        .try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("dlq tenant_id: {e}")))?;
    let tenant_id = tenant_bytes
        .map(|b| {
            uuid::Uuid::from_slice(&b)
                .map(TenantId)
                .map_err(|e| ArgoError::Provider(format!("dlq tenant_id decode: {e}")))
        })
        .transpose()?;
    let raw_payload: String = row
        .try_get("raw_payload")
        .map_err(|e| ArgoError::Provider(format!("dlq raw_payload: {e}")))?;
    let failed_at: i64 = row
        .try_get("failed_at")
        .map_err(|e| ArgoError::Provider(format!("dlq failed_at: {e}")))?;
    let failure_reason: String = row
        .try_get("failure_reason")
        .map_err(|e| ArgoError::Provider(format!("dlq failure_reason: {e}")))?;
    let retry_count: i64 = row
        .try_get("retry_count")
        .map_err(|e| ArgoError::Provider(format!("dlq retry_count: {e}")))?;
    let resolved_at: Option<i64> = row
        .try_get("resolved_at")
        .map_err(|e| ArgoError::Provider(format!("dlq resolved_at: {e}")))?;
    Ok(DeadLetterEntry {
        event_id,
        event_type,
        tenant_id,
        raw_payload,
        failed_at,
        failure_reason,
        retry_count,
        resolved_at,
    })
}

#[cfg(test)]
#[path = "billing_dlq_tests.rs"]
mod tests;
