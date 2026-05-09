//! Single-message pick handlers — `…/messages/last`,
//! `…/messages/first`, and `…/messages/<seq>`. Lifted out of
//! `routes_msgs.rs` so the file stays under the 500-LOC ceiling
//! as more list/count/rollup endpoints land.
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
    pick_message_response(sid, store, format, MsgEnd::Last).await
}

/// `GET /sessions/<id>/messages/<seq>[?format=markdown]` — fetch
/// one message by its sequence number. Mirrors the row shape
/// of `messages/last` and `messages/first`. Returns 404 when
/// no row matches the seq.
pub(super) async fn message_at_seq_response(
    sid: &str,
    seq: i64,
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
        Ok(rows) => match rows.iter().find(|m| m.seq == seq) {
            Some(m) => {
                if matches!(format, Some("markdown")) {
                    let mut out = format!(
                        "# Message seq {seq} on session `{sid}`\n\n"
                    );
                    out.push_str(&format!("- **role**: `{}`\n", m.role));
                    out.push_str(&format!(
                        "- **created_at_micros**: {}\n",
                        m.created_at
                    ));
                    out.push_str(&format!("- **id**: `{}`\n\n", hex::encode(&m.id)));
                    out.push_str("## content\n\n");
                    out.push_str("```json\n");
                    let pretty = serde_json::from_str::<serde_json::Value>(
                        &m.content_json,
                    )
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
                format!("{{\"error\":\"no message with seq {seq} on session {sid}\"}}"),
            ),
        },
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list_messages: {e}\"}}"),
        ),
    }
}

/// `GET /sessions/<id>/messages/first[?format=markdown]` —
/// symmetric with `messages/last`. Returns the very first
/// message on a session (typically the kickoff user prompt).
/// Useful for one-line "what was this session about?"
/// summaries without paginating through the full transcript.
pub(super) async fn first_message_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    pick_message_response(sid, store, format, MsgEnd::First).await
}

#[derive(Copy, Clone)]
enum MsgEnd {
    First,
    Last,
}

async fn pick_message_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
    which: MsgEnd,
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
        Ok(rows) => {
            let pick = match which {
                MsgEnd::First => rows.first(),
                MsgEnd::Last => rows.last(),
            };
            match pick {
                Some(m) => {
                    let label = match which {
                        MsgEnd::First => "First",
                        MsgEnd::Last => "Last",
                    };
                    if matches!(format, Some("markdown")) {
                        let mut out = format!("# {label} message on session `{sid}`\n\n");
                        out.push_str(&format!("- **seq**: {}\n", m.seq));
                        out.push_str(&format!("- **role**: `{}`\n", m.role));
                        out.push_str(&format!(
                            "- **created_at_micros**: {}\n",
                            m.created_at
                        ));
                        out.push_str(&format!("- **id**: `{}`\n\n", hex::encode(&m.id)));
                        out.push_str("## content\n\n");
                        out.push_str("```json\n");
                        let pretty = serde_json::from_str::<serde_json::Value>(
                            &m.content_json,
                        )
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
            }
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list_messages: {e}\"}}"),
        ),
    }
}
