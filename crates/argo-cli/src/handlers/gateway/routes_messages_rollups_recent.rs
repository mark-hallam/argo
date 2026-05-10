//! `/messages/recent` — workspace-wide most-recent message tail
//! across all sessions. Lifted out of `routes_messages_rollups.rs`
//! to keep that sibling under the 500-LOC ceiling; the recent
//! handler is the only non-aggregation route in the rollup family
//! (returns full message rows rather than a histogram), so it
//! splits cleanly along that semantic seam.
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

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
    // The 6-tuple mirrors the SELECT shape; aliasing it would
    // hide rather than clarify the column order.
    #[allow(clippy::type_complexity)]
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
