//! `GET /sessions/<id>/messages` + `…/messages/last` response
//! builders. Lifted out of `routes.rs` so the dispatcher file
//! stays under the 500-LOC ceiling as more endpoints land.
use super::store_unavailable;
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

/// `GET /sessions/<id>/messages/last` — fetch just the last
/// message on a session. Useful for chat-UI clients that
/// only need the latest assistant reply. Returns the same row
/// shape as the messages-list response, or 404 when the
/// session is empty.
pub(super) async fn last_message_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    match store.list_messages(id).await {
        Ok(rows) => match rows.last() {
            Some(m) => {
                let payload = serde_json::json!({
                    "seq": m.seq,
                    "id": hex::encode(&m.id),
                    "role": m.role,
                    "content": serde_json::from_str::<serde_json::Value>(&m.content_json)
                        .unwrap_or(serde_json::Value::Null),
                    "created_at_micros": m.created_at,
                });
                (
                    "200 OK",
                    serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
                )
            }
            None => (
                "404 Not Found",
                format!("{{\"error\":\"no messages for session {sid}\"}}"),
            ),
        },
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list_messages: {e}\"}}"),
        ),
    }
}

/// `GET /sessions/<id>/messages?limit=N&role=<r>&since-minutes=M`
/// — list messages on a session. Optional filters: `role`,
/// `limit`, and `since-minutes` (returns only rows whose
/// `created_at` is within the last M minutes).
pub(super) async fn list_messages_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    limit: Option<usize>,
    role: Option<String>,
    since_minutes: Option<i64>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    match store.list_messages(id).await {
        Ok(mut rows) => {
            if let Some(want) = role.as_deref() {
                rows.retain(|m| m.role == want);
            }
            if let Some(m) = since_minutes {
                let cutoff_micros = (chrono::Utc::now() - chrono::Duration::minutes(m))
                    .timestamp_micros();
                rows.retain(|row| row.created_at >= cutoff_micros);
            }
            if let Some(n) = limit {
                if rows.len() > n {
                    rows.truncate(n);
                }
            }
            let arr: Vec<serde_json::Value> = rows
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "seq": m.seq,
                        "id": hex::encode(&m.id),
                        "role": m.role,
                        "content": serde_json::from_str::<serde_json::Value>(&m.content_json)
                            .unwrap_or(serde_json::Value::Null),
                        "created_at_micros": m.created_at,
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
            format!("{{\"error\":\"list_messages: {e}\"}}"),
        ),
    }
}
