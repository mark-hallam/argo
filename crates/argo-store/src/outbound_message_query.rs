//! `outbound_message` idempotency table for the production
//! `send_message` dispatcher
//! (`mission-send-message-production-dispatcher`).
//!
//! Mirrors the `argo_store::tenant_query::ensure_*` pattern: the
//! versioned migration runner stops at slot 69, so callers bootstrap
//! the table lazily via [`ensure_outbound_message_schema`]. The
//! production dispatcher in `argo-channels` calls this on every
//! dispatch attempt; tests exercise the helpers against an in-memory
//! `SessionStore`.

use argo_types::{ArgoError, ArgoResult, TenantId};
use sqlx::Row;

use crate::SessionStore;

const ENSURE_OUTBOUND_MESSAGE_SQL: &str =
    include_str!("../migrations/113__outbound_message.sql");

/// One row in the `outbound_message` dedup + audit-mirror table.
///
/// Returned by [`lookup_recent_by_body_hash`] for idempotent replay
/// detection. `body_hash` is a SHA-256 hex digest of
/// `channel | recipient | body | attachments-manifest`; the dispatcher
/// computes it before calling the underlying channel transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundMessageRecord {
    /// Channel-assigned message id (or generated UUIDv7 when the
    /// channel does not return one).
    pub message_id: String,
    /// Owning tenant (UUID bytes).
    pub tenant_id: TenantId,
    /// `slack` / `email` / `sms` / `imessage` (or future channel name).
    pub channel: String,
    /// Channel-specific recipient address.
    pub recipient: String,
    /// SHA-256 hex digest of channel + recipient + body + attachments.
    pub body_hash: String,
    /// Send timestamp (unix seconds).
    pub sent_at: i64,
    /// JSON-encoded `SendOutcome` returned to the agent.
    pub outcome_json: String,
}

/// Idempotent bootstrap for the `outbound_message` table. Safe to
/// call on every dispatch — the embedded SQL uses
/// `CREATE TABLE IF NOT EXISTS` + `CREATE INDEX IF NOT EXISTS`.
pub async fn ensure_outbound_message_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in split_sql_statements(ENSURE_OUTBOUND_MESSAGE_SQL) {
        sqlx::query(&stmt)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("ensure_outbound_message: {e}")))?;
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

/// Insert a new outbound-message dedup row. `INSERT OR IGNORE` so a
/// concurrent retry with the same `message_id` becomes a no-op rather
/// than a primary-key violation.
#[allow(clippy::too_many_arguments)]
pub async fn insert_outbound_message(
    store: &SessionStore,
    message_id: &str,
    tenant_id: TenantId,
    channel: &str,
    recipient: &str,
    body_hash: &str,
    sent_at: i64,
    outcome_json: &str,
) -> ArgoResult<()> {
    ensure_outbound_message_schema(store).await?;
    sqlx::query(
        "INSERT OR IGNORE INTO outbound_message \
         (message_id, tenant_id, channel, recipient, body_hash, sent_at, outcome_json) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(message_id)
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(channel)
    .bind(recipient)
    .bind(body_hash)
    .bind(sent_at)
    .bind(outcome_json)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("outbound_message insert: {e}")))?;
    Ok(())
}

/// Return the most recent outbound-message row for `(tenant, body_hash)`
/// sent at or after `since_unix_secs`. The dispatcher uses this to
/// short-circuit a duplicate send within its idempotency window: a
/// match returns the cached `SendOutcome` (via [`OutboundMessageRecord::outcome_json`])
/// rather than re-invoking the transport.
pub async fn lookup_recent_by_body_hash(
    store: &SessionStore,
    tenant_id: TenantId,
    body_hash: &str,
    since_unix_secs: i64,
) -> ArgoResult<Option<OutboundMessageRecord>> {
    ensure_outbound_message_schema(store).await?;
    let row = sqlx::query(
        "SELECT message_id, channel, recipient, sent_at, outcome_json \
         FROM outbound_message \
         WHERE tenant_id = ? AND body_hash = ? AND sent_at >= ? \
         ORDER BY sent_at DESC LIMIT 1",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(body_hash)
    .bind(since_unix_secs)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("outbound_message lookup: {e}")))?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(OutboundMessageRecord {
        message_id: row
            .try_get::<String, _>("message_id")
            .map_err(|e| ArgoError::Provider(format!("outbound_message decode: {e}")))?,
        tenant_id,
        channel: row
            .try_get::<String, _>("channel")
            .map_err(|e| ArgoError::Provider(format!("outbound_message decode: {e}")))?,
        recipient: row
            .try_get::<String, _>("recipient")
            .map_err(|e| ArgoError::Provider(format!("outbound_message decode: {e}")))?,
        body_hash: body_hash.to_string(),
        sent_at: row
            .try_get::<i64, _>("sent_at")
            .map_err(|e| ArgoError::Provider(format!("outbound_message decode: {e}")))?,
        outcome_json: row
            .try_get::<String, _>("outcome_json")
            .map_err(|e| ArgoError::Provider(format!("outbound_message decode: {e}")))?,
    }))
}

#[cfg(test)]
#[path = "outbound_message_query_tests.rs"]
mod tests;
