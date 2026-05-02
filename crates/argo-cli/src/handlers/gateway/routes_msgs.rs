//! `GET /sessions/<id>/messages` + `…/messages/last` response
//! builders. Lifted out of `routes.rs` so the dispatcher file
//! stays under the 500-LOC ceiling as more endpoints land.
use super::store_unavailable;
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

/// `GET /sessions/<id>/messages/last[?format=markdown]` — fetch
/// just the last message on a session. Useful for chat-UI
/// clients that only need the latest assistant reply. Returns
/// the same row shape as the messages-list response (JSON
/// default), a paste-able markdown block when `format=markdown`,
/// or 404 when the session is empty.
pub(super) async fn last_message_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
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
                if matches!(format, Some("markdown")) {
                    let mut out = format!("# Last message on session `{sid}`\n\n");
                    out.push_str(&format!("- **seq**: {}\n", m.seq));
                    out.push_str(&format!("- **role**: `{}`\n", m.role));
                    out.push_str(&format!(
                        "- **created_at_micros**: {}\n",
                        m.created_at
                    ));
                    out.push_str(&format!("- **id**: `{}`\n\n", hex::encode(&m.id)));
                    out.push_str("## content\n\n");
                    out.push_str("```json\n");
                    let pretty = serde_json::from_str::<serde_json::Value>(&m.content_json)
                        .ok()
                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                        .unwrap_or_else(|| m.content_json.clone());
                    out.push_str(&pretty);
                    out.push_str("\n```\n");
                    return ("200 OK", out);
                }
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

/// `GET /sessions/<id>/messages?limit&role&since-minutes&seq-min&seq-max&content-contains`
/// — list messages on a session. Optional filters:
/// - `role`: drop messages whose role doesn't match
/// - `limit`: truncate to the FIRST N rows
/// - `tail`: keep only the LAST N rows (mutually exclusive with
///   `limit`; the dispatcher caller decides which one wins)
/// - `since-minutes`: drop messages older than M minutes
/// - `seq-min`/`seq-max`: keep only rows in that seq window
/// - `content-contains`: case-insensitive substring match
///   against the raw `content_json` blob (catches user prompts,
///   assistant text, tool args/results all in one pass)
#[allow(clippy::too_many_arguments)]
pub(super) async fn list_messages_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    limit: Option<usize>,
    tail: Option<usize>,
    role: Option<String>,
    since_minutes: Option<i64>,
    seq_min: Option<i64>,
    seq_max: Option<i64>,
    content_contains: Option<String>,
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
    if let Some(want) = role.as_deref() {
        if !matches!(want, "user" | "assistant" | "tool" | "system") {
            return (
                "400 Bad Request",
                format!(
                    "{{\"error\":\"unknown role '{want}' — use user|assistant|tool|system\"}}"
                ),
            );
        }
    }
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
            if let Some(min) = seq_min {
                rows.retain(|row| row.seq >= min);
            }
            if let Some(max) = seq_max {
                rows.retain(|row| row.seq <= max);
            }
            if let Some(needle) = content_contains.as_deref() {
                let lower = needle.to_lowercase();
                rows.retain(|row| row.content_json.to_lowercase().contains(&lower));
            }
            if let Some(n) = limit {
                if rows.len() > n {
                    rows.truncate(n);
                }
            } else if let Some(n) = tail {
                if rows.len() > n {
                    let drop = rows.len() - n;
                    rows.drain(..drop);
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

/// `GET /sessions/<id>/messages/count` — count messages on a
/// session after applying the same filter family as the list
/// endpoint (role, since-minutes, seq-min/max, content-contains).
/// Returns `{count}` JSON by default; with `format=markdown` a
/// single-bullet summary doc paste-able into operator notes.
#[allow(clippy::too_many_arguments)]
pub(super) async fn count_messages_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    role: Option<String>,
    since_minutes: Option<i64>,
    seq_min: Option<i64>,
    seq_max: Option<i64>,
    content_contains: Option<String>,
    format: Option<&str>,
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
    if let Some(want) = role.as_deref() {
        if !matches!(want, "user" | "assistant" | "tool" | "system") {
            return (
                "400 Bad Request",
                format!(
                    "{{\"error\":\"unknown role '{want}' — use user|assistant|tool|system\"}}"
                ),
            );
        }
    }
    match store.list_messages(id).await {
        Ok(mut rows) => {
            if let Some(want) = role.as_deref() {
                rows.retain(|m| m.role == want);
            }
            if let Some(m) = since_minutes {
                let cutoff_micros =
                    (chrono::Utc::now() - chrono::Duration::minutes(m)).timestamp_micros();
                rows.retain(|row| row.created_at >= cutoff_micros);
            }
            if let Some(min) = seq_min {
                rows.retain(|row| row.seq >= min);
            }
            if let Some(max) = seq_max {
                rows.retain(|row| row.seq <= max);
            }
            if let Some(needle) = content_contains.as_deref() {
                let lower = needle.to_lowercase();
                rows.retain(|row| row.content_json.to_lowercase().contains(&lower));
            }
            let count = rows.len();
            if matches!(format, Some("markdown")) {
                let mut out = format!("# Messages on session `{sid}` ({count})\n\n");
                if role.is_some()
                    || since_minutes.is_some()
                    || seq_min.is_some()
                    || seq_max.is_some()
                    || content_contains.is_some()
                {
                    out.push_str("## filters\n\n");
                    if let Some(r) = role.as_deref() {
                        out.push_str(&format!("- **role**: `{r}`\n"));
                    }
                    if let Some(m) = since_minutes {
                        out.push_str(&format!("- **since-minutes**: {m}\n"));
                    }
                    if let Some(s) = seq_min {
                        out.push_str(&format!("- **seq-min**: {s}\n"));
                    }
                    if let Some(s) = seq_max {
                        out.push_str(&format!("- **seq-max**: {s}\n"));
                    }
                    if let Some(c) = content_contains.as_deref() {
                        out.push_str(&format!("- **content-contains**: `{c}`\n"));
                    }
                }
                return ("200 OK", out);
            }
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({ "count": count }))
                    .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list_messages: {e}\"}}"),
        ),
    }
}
