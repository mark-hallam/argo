//! Per-session audit rollup handlers
//! (`/sessions/<id>/audit/by-kind`, `/sessions/<id>/audit/by-tool`,
//! `/sessions/<id>/audit/tool-errors`) — extracted from
//! `routes_audit_rollups.rs` once that file went over the 500-LOC
//! budget on the third per-session drill-down (#628). The
//! workspace-wide rollups (`audit/by-session`, `audit/by-tool`,
//! `audit/tool-errors`) stay in the parent file.
use argo_store::SessionStore;
use std::sync::Arc;


/// `GET /sessions/<id>/audit/by-kind[?since-minutes=M&format=markdown]`
/// — per-session drill-down of `/audit/by-kind` (#570). For one
/// session, returns the histogram of audit-row kinds (turn_start,
/// tool_call, tool_result, …). Useful for "what did this session
/// actually do?" — pairs with `/sessions/<id>/stats`.
/// Returns `{by_kind: {<kind>: <count>}}` for JSON; markdown is
/// sorted desc by count.
pub(super) async fn session_audit_by_kind_response(
    sid: &str,
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => u,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    let pool = store.pool();
    let rows: Result<Vec<(String, i64)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT kind, COUNT(*) FROM audit_record \
                 WHERE session_id = ? AND created_at >= ? \
                 GROUP BY kind ORDER BY kind",
            )
            .bind(id.as_bytes().to_vec())
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT kind, COUNT(*) FROM audit_record \
                 WHERE session_id = ? \
                 GROUP BY kind ORDER BY kind",
            )
            .bind(id.as_bytes().to_vec())
            .fetch_all(pool)
            .await
        }
    };
    match rows {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let mut sorted = rows.clone();
                sorted.sort_by_key(|r| std::cmp::Reverse(r.1));
                let total: i64 = sorted.iter().map(|(_, n)| *n).sum();
                let mut out = format!(
                    "# Audit by kind for session `{sid}` ({total} rows across {} kinds)\n\n",
                    sorted.len()
                );
                if !sorted.is_empty() {
                    out.push_str("| kind | count |\n| --- | ---: |\n");
                    for (k, n) in &sorted {
                        out.push_str(&format!("| `{k}` | {n} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let mut by_kind = serde_json::Map::new();
            for (k, n) in rows {
                by_kind.insert(k, serde_json::json!(n));
            }
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({
                    "by_kind": serde_json::Value::Object(by_kind),
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"session_audit_by_kind: {e}\"}}"),
        ),
    }
}

/// `GET /sessions/<id>/audit/by-tool[?since-minutes=M&format=markdown]`
/// — per-session drill-down of `/audit/by-tool` (#613). For one
/// session, returns the histogram of `tool_call`+`tool_result`
/// rows grouped by tool name. Useful for "which tools did this
/// session call?".
pub(super) async fn session_audit_by_tool_response(
    sid: &str,
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => u,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    let pool = store.pool();
    let rows: Result<Vec<(String, i64)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE session_id = ? AND kind IN ('tool_call', 'tool_result') \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                   AND created_at >= ? \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .bind(id.as_bytes().to_vec())
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE session_id = ? AND kind IN ('tool_call', 'tool_result') \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .bind(id.as_bytes().to_vec())
            .fetch_all(pool)
            .await
        }
    };
    match rows {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let total: i64 = rows.iter().map(|(_, n)| *n).sum();
                let mut out = format!(
                    "# Audit by tool for session `{sid}` ({total} rows across {} tool{})\n\n",
                    rows.len(),
                    if rows.len() == 1 { "" } else { "s" }
                );
                if !rows.is_empty() {
                    out.push_str("| tool | count |\n| --- | ---: |\n");
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
            format!("{{\"error\":\"session_audit_by_tool: {e}\"}}"),
        ),
    }
}

/// `GET /sessions/<id>/audit/tool-errors[?since-minutes=M&format=markdown]`
/// — per-session drill-down of `/audit/tool-errors` (#617). For
/// one session, returns the histogram of `tool_result` rows where
/// `is_error=true`, grouped by tool name. Completes the per-session
/// audit drill-down trio (by-kind #621 + by-tool #621 + tool-errors
/// here). "Which tools failed in this session?".
pub(super) async fn session_audit_tool_errors_response(
    sid: &str,
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => u,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    let pool = store.pool();
    let rows: Result<Vec<(String, i64)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE session_id = ? AND kind = 'tool_result' \
                   AND json_extract(payload_json, '$.is_error') = 1 \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                   AND created_at >= ? \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .bind(id.as_bytes().to_vec())
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \
                 FROM audit_record \
                 WHERE session_id = ? AND kind = 'tool_result' \
                   AND json_extract(payload_json, '$.is_error') = 1 \
                   AND json_extract(payload_json, '$.tool') IS NOT NULL \
                 GROUP BY tool ORDER BY COUNT(*) DESC",
            )
            .bind(id.as_bytes().to_vec())
            .fetch_all(pool)
            .await
        }
    };
    match rows {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let total: i64 = rows.iter().map(|(_, n)| *n).sum();
                let mut out = format!(
                    "# Tool errors for session `{sid}` ({total} failures across {} tool{})\n\n",
                    rows.len(),
                    if rows.len() == 1 { "" } else { "s" }
                );
                if rows.is_empty() {
                    out.push_str(
                        "No `tool_result` rows with `is_error=true` for this session — clean run.\n",
                    );
                } else {
                    out.push_str("| tool | failures |\n| --- | ---: |\n");
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
            format!("{{\"error\":\"session_audit_tool_errors: {e}\"}}"),
        ),
    }
}
