//! Per-session aggregate queries used by `session show` /
//! `session list`. Extracted from `session.rs` to keep that file
//! under the 500-LOC budget.
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};

/// `session_id (blob) → COUNT(audit_record where kind='ToolCall')`
/// for every session that fired at least one tool call. One
/// indexed GROUP BY query — no per-session round-trip.
pub(crate) async fn tool_calls_per_session(
    store: &SessionStore,
) -> ArgoResult<std::collections::HashMap<Vec<u8>, u64>> {
    let rows: Vec<(Vec<u8>, i64)> = sqlx::query_as(
        "SELECT session_id, COUNT(*) FROM audit_record \
         WHERE kind = 'ToolCall' GROUP BY session_id",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("tool_calls_per_session: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(sid, count)| (sid, u64::try_from(count).unwrap_or(0)))
        .collect())
}

/// `session_id (blob) → SUM(amount_usd_micros)` for every session
/// that has at least one row in `spend_ledger`. One query, no
/// per-session round-trip.
pub(crate) async fn spend_per_session(
    store: &SessionStore,
) -> ArgoResult<std::collections::HashMap<Vec<u8>, u64>> {
    let rows: Vec<(Vec<u8>, i64)> = sqlx::query_as(
        "SELECT session_id, COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \
         GROUP BY session_id",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("spend_per_session: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(sid, total)| (sid, u64::try_from(total).unwrap_or(0)))
        .collect())
}

/// Sum of `amount_usd_micros` for one session — single-row variant.
pub(crate) async fn session_spend_micros(
    store: &SessionStore,
    sid: SessionId,
) -> ArgoResult<u64> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger WHERE session_id = ?",
    )
    .bind(sid.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("session spend sum: {e}")))?;
    Ok(u64::try_from(total).unwrap_or(0))
}

/// Walk `messages.content_json` and count `tool_use` content
/// blocks across the session.
pub(crate) fn count_tool_use_blocks(messages: &[argo_store::MessageRow]) -> u64 {
    let mut n = 0u64;
    for m in messages {
        let parsed: serde_json::Value =
            serde_json::from_str(&m.content_json).unwrap_or(serde_json::Value::Null);
        let Some(arr) = parsed.as_array() else {
            continue;
        };
        for b in arr {
            if b.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                n += 1;
            }
        }
    }
    n
}
