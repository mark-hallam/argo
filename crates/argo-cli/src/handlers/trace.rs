//! `argo trace` — render an audit-log timeline (or one turn's
//! span tree) for a session.
//!
//! Two modes:
//! - `--session <id>`: full per-session audit timeline
//!   (chronological, summary stats at the bottom).
//! - `--turn <session-id>[:<turn-index>]`: hierarchical span
//!   tree for one specific turn (turn-index defaults to 0,
//!   the first turn). Renders TurnStart at the root with
//!   AssistantMessage rounds and ToolCall → ToolResult pairs
//!   underneath, and a one-line summary per row.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::{AuditRow, SessionStore};
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::{DateTime, Utc};

pub async fn run(turn: Option<String>, session: Option<String>) -> ArgoResult<()> {
    if let Some(spec) = turn {
        return run_turn(&spec).await;
    }
    let Some(raw) = session else {
        return Err(ArgoError::Provider(
            "argo trace: pass --session <id> for the full timeline, or --turn <session>[:<idx>] for one turn's span tree.".into(),
        ));
    };
    run_session_timeline(&raw).await
}

async fn run_session_timeline(raw: &str) -> ArgoResult<()> {
    let store = open_store().await?;
    let id = parse_session(raw)?;
    let rows = store.list_audit_for_session(SessionId(id)).await?;
    if rows.is_empty() {
        println!("(no audit rows for session {raw})");
        return Ok(());
    }
    let first = DateTime::<Utc>::from_timestamp_micros(rows[0].created_at).unwrap_or_else(Utc::now);
    let last = DateTime::<Utc>::from_timestamp_micros(rows[rows.len() - 1].created_at)
        .unwrap_or_else(Utc::now);
    let total_ms = (rows[rows.len() - 1].created_at - rows[0].created_at) / 1000;
    println!("trace for {raw} ({} rows; start={})", rows.len(), first);
    let start = rows[0].created_at;
    let mut turn_starts = 0_u32;
    let mut tool_calls = 0_u32;
    let mut tool_errors = 0_u32;
    for r in &rows {
        let dt = DateTime::<Utc>::from_timestamp_micros(r.created_at).unwrap_or_else(Utc::now);
        let dt_ms = (r.created_at - start) / 1000;
        let preview = preview(&r.payload_json);
        match r.kind.as_str() {
            "TurnStart" => turn_starts += 1,
            "ToolCall" => tool_calls += 1,
            "ToolResult" if r.payload_json.contains("\"is_error\":true") => tool_errors += 1,
            _ => {}
        }
        println!(
            "  +{ms:>6}ms  seq={seq:<5} {dt}  {kind:<24} {preview}",
            ms = dt_ms,
            seq = r.seq,
            dt = dt.format("%H:%M:%S%.3f"),
            kind = r.kind,
            preview = preview,
        );
    }
    println!(
        "\nsummary: {turn_starts} turn(s), {tool_calls} tool call(s) ({tool_errors} error(s)), \
         span={total_ms} ms (last={last})"
    );
    Ok(())
}

/// `--turn` mode. Expects `<session_id>` (turn 0) or
/// `<session_id>:<idx>` (specific turn index).
async fn run_turn(spec: &str) -> ArgoResult<()> {
    let (raw_session, idx) = match spec.split_once(':') {
        Some((s, i)) => {
            let parsed: usize = i.parse().map_err(|e| {
                ArgoError::Provider(format!("--turn: invalid index in '{spec}': {e}"))
            })?;
            (s, parsed)
        }
        None => (spec, 0),
    };
    let store = open_store().await?;
    let session_id = parse_session(raw_session)?;
    let rows = store.list_audit_for_session(SessionId(session_id)).await?;
    let span = extract_turn(&rows, idx).ok_or_else(|| {
        ArgoError::Provider(format!(
            "session {raw_session} has fewer than {} turn(s)",
            idx + 1
        ))
    })?;
    let start_micros = span[0].created_at;
    let span_ms = (span[span.len() - 1].created_at - start_micros) / 1000;
    println!(
        "trace --turn {raw_session}:{idx} ({} rows, span={span_ms}ms)",
        span.len()
    );
    for r in span {
        let indent = match r.kind.as_str() {
            "TurnStart" | "TurnEnd" | "Halt" => "  ",
            "AssistantMessage" => "    ",
            "ToolCall" => "      → ",
            "ToolResult" => "        ← ",
            _ => "    · ",
        };
        let dt_ms = (r.created_at - start_micros) / 1000;
        println!(
            "{indent}+{dt_ms:>6}ms  {kind:<18}  {preview}",
            kind = r.kind,
            preview = preview(&r.payload_json),
        );
    }
    Ok(())
}

/// Slice the audit rows for a session into the `idx`-th turn.
/// Returns `None` if there are fewer than `idx + 1` `TurnStart`
/// rows. If the matching `TurnStart` is unterminated and the
/// span runs past the end of the row list (no `TurnEnd`/`Halt`),
/// we still return what we have — useful for inspecting an
/// in-flight or crashed run.
fn extract_turn(rows: &[AuditRow], idx: usize) -> Option<&[AuditRow]> {
    let starts: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.kind == "TurnStart")
        .map(|(i, _)| i)
        .collect();
    let start_idx = *starts.get(idx)?;
    let next_start = starts.get(idx + 1).copied();
    let end_idx = rows[start_idx + 1..]
        .iter()
        .position(|r| matches!(r.kind.as_str(), "TurnEnd" | "Halt"))
        .map(|p| start_idx + 1 + p);
    let bound = match (end_idx, next_start) {
        (Some(e), Some(n)) => e.min(n - 1),
        (Some(e), None) => e,
        (None, Some(n)) => n - 1,
        (None, None) => rows.len() - 1,
    };
    Some(&rows[start_idx..=bound])
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

fn parse_session(raw: &str) -> ArgoResult<uuid::Uuid> {
    uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))
}

fn preview(payload: &str) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(payload).unwrap_or(serde_json::Value::Null);
    if let Some(obj) = parsed.as_object() {
        let mut pairs: Vec<String> = obj
            .iter()
            .take(4)
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                let val = val.chars().take(40).collect::<String>();
                format!("{k}={val}")
            })
            .collect();
        if obj.len() > 4 {
            pairs.push("…".into());
        }
        pairs.join(" ")
    } else {
        payload.chars().take(80).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(kind: &str, created_at: i64) -> AuditRow {
        AuditRow {
            seq: 0,
            kind: kind.to_string(),
            payload_json: "{}".to_string(),
            created_at,
        }
    }

    #[test]
    fn extract_turn_picks_the_nth_turn() {
        let rows = vec![
            row("TurnStart", 0),
            row("AssistantMessage", 1),
            row("TurnEnd", 2),
            row("TurnStart", 3),
            row("AssistantMessage", 4),
            row("ToolCall", 5),
            row("ToolResult", 6),
            row("TurnEnd", 7),
        ];
        let t0 = extract_turn(&rows, 0).unwrap();
        assert_eq!(t0.len(), 3);
        assert_eq!(t0.first().map(|r| r.kind.as_str()), Some("TurnStart"));
        assert_eq!(t0.last().map(|r| r.kind.as_str()), Some("TurnEnd"));
        let t1 = extract_turn(&rows, 1).unwrap();
        assert_eq!(t1.len(), 5);
        assert_eq!(t1[2].kind, "ToolCall");
    }

    #[test]
    fn extract_turn_returns_none_for_out_of_range_idx() {
        let rows = vec![row("TurnStart", 0), row("TurnEnd", 1)];
        assert!(extract_turn(&rows, 3).is_none());
    }

    #[test]
    fn extract_turn_handles_unterminated_in_flight() {
        let rows = vec![
            row("TurnStart", 0),
            row("AssistantMessage", 1),
            row("ToolCall", 2),
        ];
        let span = extract_turn(&rows, 0).unwrap();
        assert_eq!(span.len(), 3);
    }
}
