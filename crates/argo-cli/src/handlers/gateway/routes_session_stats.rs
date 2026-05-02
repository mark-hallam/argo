//! `GET /sessions/<id>/stats[?format=markdown]` — per-session
//! summary view that goes beyond the bare counts on
//! `/sessions/<id>`. Reports the role histogram (user /
//! assistant / tool / system), the timing envelope (first /
//! last message + active duration), and the inter-message gap
//! statistics (avg + max) so callers can spot stuck or
//! pathologically slow sessions at a glance.
use super::store_unavailable;
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

pub(super) async fn session_stats_response(
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
    let rows = match store.list_messages(id).await {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"list_messages: {e}\"}}"),
            );
        }
    };
    let (mut user, mut assistant, mut tool, mut system, mut other): (
        u64,
        u64,
        u64,
        u64,
        u64,
    ) = (0, 0, 0, 0, 0);
    for r in &rows {
        match r.role.as_str() {
            "user" => user += 1,
            "assistant" => assistant += 1,
            "tool" => tool += 1,
            "system" => system += 1,
            _ => other += 1,
        }
    }
    let first_at = rows.first().map(|r| r.created_at);
    let last_at = rows.last().map(|r| r.created_at);
    let active_duration_seconds = match (first_at, last_at) {
        (Some(a), Some(b)) => (b - a) as f64 / 1_000_000.0,
        _ => 0.0,
    };
    let mut gaps: Vec<f64> = Vec::with_capacity(rows.len().saturating_sub(1));
    for w in rows.windows(2) {
        let dt = (w[1].created_at - w[0].created_at) as f64 / 1_000_000.0;
        gaps.push(dt);
    }
    let avg_gap_seconds = if gaps.is_empty() {
        0.0
    } else {
        gaps.iter().sum::<f64>() / gaps.len() as f64
    };
    let max_gap_seconds = gaps.iter().cloned().fold(0.0_f64, f64::max);
    let total = rows.len() as u64;
    if matches!(format, Some("markdown")) {
        let mut out = format!("# Stats for session `{sid}`\n\n");
        out.push_str(&format!("- **total_messages**: {total}\n"));
        out.push_str("\n## by role\n\n");
        out.push_str(&format!("- **user**: {user}\n"));
        out.push_str(&format!("- **assistant**: {assistant}\n"));
        out.push_str(&format!("- **tool**: {tool}\n"));
        out.push_str(&format!("- **system**: {system}\n"));
        if other > 0 {
            out.push_str(&format!("- **other**: {other}\n"));
        }
        out.push_str("\n## timing\n\n");
        if let Some(t) = first_at {
            out.push_str(&format!("- **first_message_at_micros**: {t}\n"));
        }
        if let Some(t) = last_at {
            out.push_str(&format!("- **last_message_at_micros**: {t}\n"));
        }
        out.push_str(&format!(
            "- **active_duration_seconds**: {active_duration_seconds:.3}\n"
        ));
        out.push_str(&format!("- **avg_gap_seconds**: {avg_gap_seconds:.3}\n"));
        out.push_str(&format!("- **max_gap_seconds**: {max_gap_seconds:.3}\n"));
        return ("200 OK", out);
    }
    let mut by_role = serde_json::Map::new();
    by_role.insert("user".into(), user.into());
    by_role.insert("assistant".into(), assistant.into());
    by_role.insert("tool".into(), tool.into());
    by_role.insert("system".into(), system.into());
    if other > 0 {
        by_role.insert("other".into(), other.into());
    }
    let payload = serde_json::json!({
        "session_id": sid,
        "total_messages": total,
        "by_role": serde_json::Value::Object(by_role),
        "first_message_at_micros": first_at,
        "last_message_at_micros": last_at,
        "active_duration_seconds": active_duration_seconds,
        "avg_gap_seconds": avg_gap_seconds,
        "max_gap_seconds": max_gap_seconds,
    });
    (
        "200 OK",
        serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// `GET /sessions/<id>/turns[?format=markdown]` — flatten the
/// session's message stream into user→assistant→tools turn
/// tuples. A turn starts on a `user` role message and ends
/// just before the next `user` message (or at session end).
/// Returns
/// `{turns: [{turn, user_seq, assistant_seq, tool_count,
/// started_at_micros, ended_at_micros, duration_ms}]}` —
/// suitable for chat UIs that want a turn-level view rather
/// than the raw message list.
pub(super) async fn session_turns_response(
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
    let rows = match store.list_messages(id).await {
        Ok(r) => r,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"list_messages: {e}\"}}"),
            );
        }
    };
    // Walk messages, accumulate turns. A turn opens on user;
    // closes on the next user (or end-of-session).
    #[derive(Default)]
    struct Turn {
        index: u64,
        user_seq: Option<i64>,
        assistant_seq: Option<i64>,
        tool_count: u64,
        started_at: Option<i64>,
        ended_at: Option<i64>,
    }
    let mut turns: Vec<Turn> = Vec::new();
    for r in &rows {
        match r.role.as_str() {
            "user" => {
                turns.push(Turn {
                    index: turns.len() as u64 + 1,
                    user_seq: Some(r.seq),
                    started_at: Some(r.created_at),
                    ended_at: Some(r.created_at),
                    ..Default::default()
                });
            }
            other => {
                if let Some(t) = turns.last_mut() {
                    if other == "assistant" {
                        t.assistant_seq.get_or_insert(r.seq);
                    } else if other == "tool" {
                        t.tool_count += 1;
                    }
                    t.ended_at = Some(r.created_at);
                }
                // assistant/tool/system before any user are
                // dropped — they don't fit the turn shape.
            }
        }
    }
    if matches!(format, Some("markdown")) {
        let mut out = format!(
            "# Turns on session `{sid}` ({} turn{})\n\n",
            turns.len(),
            if turns.len() == 1 { "" } else { "s" }
        );
        if !turns.is_empty() {
            out.push_str("| turn | user_seq | assistant_seq | tools | duration_ms |\n");
            out.push_str("| ---: | ---: | ---: | ---: | ---: |\n");
            for t in &turns {
                let dur = match (t.started_at, t.ended_at) {
                    (Some(a), Some(b)) => (b - a) / 1000,
                    _ => 0,
                };
                out.push_str(&format!(
                    "| {} | {} | {} | {} | {dur} |\n",
                    t.index,
                    t.user_seq.map(|s| s.to_string()).unwrap_or_else(|| "-".into()),
                    t.assistant_seq
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "-".into()),
                    t.tool_count,
                ));
            }
        }
        return ("200 OK", out);
    }
    let arr: Vec<serde_json::Value> = turns
        .iter()
        .map(|t| {
            let duration_ms = match (t.started_at, t.ended_at) {
                (Some(a), Some(b)) => (b - a) / 1000,
                _ => 0,
            };
            serde_json::json!({
                "turn": t.index,
                "user_seq": t.user_seq,
                "assistant_seq": t.assistant_seq,
                "tool_count": t.tool_count,
                "started_at_micros": t.started_at,
                "ended_at_micros": t.ended_at,
                "duration_ms": duration_ms,
            })
        })
        .collect();
    (
        "200 OK",
        serde_json::to_string(&serde_json::json!({"turns": arr}))
            .unwrap_or_else(|_| "{}".to_string()),
    )
}
