//! Messages-table rollup handlers (`/messages/by-role`,
//! `/messages/by-session`, `/messages/recent`) — lifted into a
//! sibling file so `routes_msgs.rs` stays under the 500-LOC
//! ceiling once the rollup family started growing past by-day.
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

/// `GET /messages/by-role[?since-minutes=M&format=markdown]`
/// — histogram of messages grouped by `role` (user / assistant /
/// tool / system). Useful for quickly understanding workspace
/// activity shape — "what's the user/assistant/tool ratio?" —
/// without paginating through any individual session.
/// Pairs with `/audit/by-kind` (#570) and `/messages/by-day`.
/// Returns `{by_role: {<role>: <count>}}` ordered by role for JSON;
/// markdown table is sorted desc by count.
pub(super) async fn messages_by_role_response(
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let pool = store.pool();
    let rows: Result<Vec<(String, i64)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT role, COUNT(*) FROM message WHERE created_at >= ? \
                 GROUP BY role ORDER BY role",
            )
            .bind(cutoff)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT role, COUNT(*) FROM message GROUP BY role ORDER BY role",
            )
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
                    "# Messages by role ({total} rows across {} role{})\n\n",
                    sorted.len(),
                    if sorted.len() == 1 { "" } else { "s" }
                );
                if !sorted.is_empty() {
                    out.push_str("| role | count |\n");
                    out.push_str("| --- | ---: |\n");
                    for (r, n) in &sorted {
                        out.push_str(&format!("| `{r}` | {n} |\n"));
                    }
                }
                return ("200 OK", out);
            }
            let mut by_role = serde_json::Map::new();
            for (r, n) in rows {
                by_role.insert(r, serde_json::json!(n));
            }
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({
                    "by_role": serde_json::Value::Object(by_role),
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"messages_by_role: {e}\"}}"),
        ),
    }
}

/// `GET /messages/by-session[?limit=N&since-minutes=M&format=markdown]`
/// — top-N sessions by message row count. Useful for spotting the
/// busiest sessions in the workspace at a glance — pairs with
/// `/audit/by-session` (#608) for the audit-row-count counterpart.
/// Default limit 50, capped at 500. Returns
/// `{by_session: [{session_id, count}]}` ordered desc.
pub(super) async fn messages_by_session_response(
    limit: u32,
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let limit = limit.clamp(1, 500);
    let pool = store.pool();
    let rows: Result<Vec<(Vec<u8>, i64)>, _> = match since_minutes {
        Some(m) => {
            let cutoff =
                (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
            sqlx::query_as(
                "SELECT session_id, COUNT(*) FROM message \
                 WHERE created_at >= ? \
                 GROUP BY session_id ORDER BY COUNT(*) DESC LIMIT ?",
            )
            .bind(cutoff)
            .bind(i64::from(limit))
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT session_id, COUNT(*) FROM message \
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
                    "# Messages by session ({total} rows across {} session{})\n\n",
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
            format!("{{\"error\":\"messages_by_session: {e}\"}}"),
        ),
    }
}

/// `GET /messages/recent[?limit=N&role=user&since-minutes=M&format=markdown]`
/// — workspace-wide most-recent message tail across all sessions.
/// Parallel to `/audit/recent` but for the message table. Useful
/// for quickly seeing "what's happening across all sessions" without
/// pre-knowing a session id. Default limit 50, cap 500. Returns a
/// JSON array of message rows ordered by `created_at DESC`.
pub(super) async fn messages_recent_response(
    limit: u32,
    role: Option<&str>,
    since_minutes: Option<i64>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let limit = limit.clamp(1, 500);
    let pool = store.pool();
    let since_micros = since_minutes
        .map(|m| (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros());
    let rows: Result<Vec<(i64, Vec<u8>, Vec<u8>, String, String, i64)>, _> = match (role, since_micros) {
        (Some(r), Some(s)) => sqlx::query_as(
            "SELECT seq, id, session_id, role, content_json, created_at \
             FROM message WHERE role = ? AND created_at >= ? \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(r)
        .bind(s)
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await,
        (Some(r), None) => sqlx::query_as(
            "SELECT seq, id, session_id, role, content_json, created_at \
             FROM message WHERE role = ? \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(r)
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await,
        (None, Some(s)) => sqlx::query_as(
            "SELECT seq, id, session_id, role, content_json, created_at \
             FROM message WHERE created_at >= ? \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(s)
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await,
        (None, None) => sqlx::query_as(
            "SELECT seq, id, session_id, role, content_json, created_at \
             FROM message ORDER BY created_at DESC LIMIT ?",
        )
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await,
    };
    match rows {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                let mut out =
                    format!("# Messages recent ({n} row{})\n\n", if n == 1 { "" } else { "s" });
                if !rows.is_empty() {
                    out.push_str("| seq | role | session_id | content (truncated) |\n");
                    out.push_str("| ---: | --- | --- | --- |\n");
                    for (seq, _id, sid_bytes, role, content, _ts) in &rows {
                        let sid = uuid::Uuid::from_slice(sid_bytes)
                            .map(|u| u.to_string())
                            .unwrap_or_else(|_| hex::encode(sid_bytes));
                        let preview: String = content.chars().take(80).collect();
                        let preview = preview
                            .replace('|', "\\|")
                            .replace('\n', " ");
                        out.push_str(&format!(
                            "| {seq} | `{role}` | `{sid}` | {preview} |\n"
                        ));
                    }
                }
                return ("200 OK", out);
            }
            let arr: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(seq, id, sid, role, content, ts)| {
                    let sid_hex = uuid::Uuid::from_slice(&sid)
                        .map(|u| u.to_string())
                        .unwrap_or_else(|_| hex::encode(&sid));
                    serde_json::json!({
                        "seq": seq,
                        "id": hex::encode(&id),
                        "session_id": sid_hex,
                        "role": role,
                        "content": serde_json::from_str::<serde_json::Value>(&content)
                            .unwrap_or(serde_json::Value::Null),
                        "created_at_micros": ts,
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"messages_recent: {e}\"}}"),
        ),
    }
}
