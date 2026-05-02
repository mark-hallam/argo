//! Workspace-wide audit rollup handlers (`audit/by-session`,
//! `audit/by-tool`, `audit/tool-errors`). Per-session drill-downs
//! live in `routes_session_audit_rollups.rs` since #628 — split out
//! once the per-session family hit the third member.
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /audit/by-session[?limit=N&since-minutes=M&format=markdown]`
/// — top-N sessions by audit row count. Useful for spotting
/// the busiest sessions in the workspace at a glance — pairs
/// with /audit/by-kind (#570) and /audit/by-day (#571).
/// Default limit 50, capped at 500. Returns
/// `{by_session: [{session_id, count}]}` ordered desc.
pub(super) async fn audit_by_session_response(
    limit: u32,
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let limit = limit.clamp(1, 500);
    let pool = store.pool();
    let rows: Result<Vec<(Vec<u8>, i64)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT session_id, COUNT(*) FROM audit_record \
                 WHERE session_id IS NOT NULL AND created_at >= ? \
                 GROUP BY session_id ORDER BY COUNT(*) DESC LIMIT ?",
            )
            .bind(cutoff)
            .bind(i64::from(limit))
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT session_id, COUNT(*) FROM audit_record \
                 WHERE session_id IS NOT NULL \
                 GROUP BY session_id ORDER BY COUNT(*) DESC LIMIT ?",
            )
            .bind(i64::from(limit))
            .fetch_all(pool)
            .await
        }
    };
    match rows {
        Ok(rows) => {
            let mut buckets: Vec<(String, i64)> = Vec::with_capacity(rows.len());
            for (sid_bytes, n) in rows {
                let sid_hex = match uuid::Uuid::from_slice(&sid_bytes) {
                    Ok(u) => u.to_string(),
                    Err(_) => hex::encode(&sid_bytes),
                };
                buckets.push((sid_hex, n));
            }
            if matches!(format, Some("markdown")) {
                let total: i64 = buckets.iter().map(|(_, n)| *n).sum();
                let mut out = format!(
                    "# Audit by session ({total} rows across {} session{})\n\n",
                    buckets.len(),
                    if buckets.len() == 1 { "" } else { "s" }
                );
                if !buckets.is_empty() {
                    out.push_str("| session_id | rows |\n");
                    out.push_str("| --- | ---: |\n");
                    for (s, n) in &buckets {
                        out.push_str(&format!("| `{s}` | {n} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = buckets
                .into_iter()
                .map(|(s, n)| serde_json::json!({"session_id": s, "count": n}))
                .collect();
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({"by_session": arr}))
                    .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"audit_by_session: {e}\"}}"),
        ),
    }
}

/// `GET /audit/by-tool[?since-minutes=M&format=markdown]` —
/// histogram of `tool_call` + `tool_result` audit rows grouped by
/// tool name (extracted from `payload_json`'s `$.tool` field).
/// Useful for spotting the most-used tools in the workspace at a
/// glance — a stronger signal than `/audit/by-kind` which buckets
/// every tool call into one row. Pairs with `/audit/by-kind` (#570),
/// `/audit/by-day` (#571), `/audit/by-session` (#608).
/// Returns `{by_tool: [{tool, count}]}` ordered desc.
pub(super) async fn audit_by_tool_response(
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let pool = store.pool();
    let rows: Result<Vec<(String, i64)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE kind IN ('tool_call', 'tool_result') \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                   AND created_at >= ? \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE kind IN ('tool_call', 'tool_result') \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .fetch_all(pool)
            .await
        }
    };
    match rows {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let total: i64 = rows.iter().map(|(_, n)| *n).sum();
                let mut out = format!(
                    "# Audit by tool ({total} rows across {} tool{})\n\n",
                    rows.len(),
                    if rows.len() == 1 { "" } else { "s" }
                );
                if !rows.is_empty() {
                    out.push_str("| tool | count |\n");
                    out.push_str("| --- | ---: |\n");
                    for (t, n) in &rows {
                        out.push_str(&format!("| `{t}` | {n} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(t, n)| serde_json::json!({"tool": t, "count": n}))
                .collect();
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({"by_tool": arr}))
                    .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"audit_by_tool: {e}\"}}"),
        ),
    }
}

/// `GET /audit/tool-errors[?since-minutes=M&format=markdown]` —
/// histogram of `tool_result` audit rows where `is_error=true`,
/// grouped by tool name. Operational signal for spotting broken
/// tool integrations at a glance — much sharper than scrolling
/// `/audit/recent`. Pairs with `/audit/by-tool` (#613) which
/// counts ALL tool invocations (success+error).
/// Returns `{tool_errors: [{tool, count}]}` ordered desc.
pub(super) async fn audit_tool_errors_response(
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let pool = store.pool();
    let rows: Result<Vec<(String, i64)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE kind = 'tool_result' \
                   AND json_extract(payload_json, '$.is_error') = 1 \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                   AND created_at >= ? \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE kind = 'tool_result' \
                   AND json_extract(payload_json, '$.is_error') = 1 \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .fetch_all(pool)
            .await
        }
    };
    match rows {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let total: i64 = rows.iter().map(|(_, n)| *n).sum();
                let mut out = format!(
                    "# Tool errors ({total} failures across {} tool{})\n\n",
                    rows.len(),
                    if rows.len() == 1 { "" } else { "s" }
                );
                if rows.is_empty() {
                    out.push_str(
                        "No `tool_result` rows with `is_error=true` in this window — clean run.\n",
                    );
                } else {
                    out.push_str("| tool | failures |\n");
                    out.push_str("| --- | ---: |\n");
                    for (t, n) in &rows {
                        out.push_str(&format!("| `{t}` | {n} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(t, n)| serde_json::json!({"tool": t, "count": n}))
                .collect();
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({"tool_errors": arr}))
                    .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"audit_tool_errors: {e}\"}}"),
        ),
    }
}
