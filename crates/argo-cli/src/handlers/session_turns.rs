//! `argo session turns <id> [--json]` — turn-level view of a
//! recorded session. Mirrors the gateway's
//! `GET /sessions/<id>/turns` (#602).
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};

#[derive(Default)]
struct Turn {
    index: u64,
    user_seq: Option<i64>,
    assistant_seq: Option<i64>,
    tool_count: u64,
    started_at: Option<i64>,
    ended_at: Option<i64>,
}

pub async fn run(raw: &str, json: bool) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Schema(format!("invalid session id '{raw}': {e}")))?;
    let rows = store.list_messages(SessionId(id)).await?;
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
            }
        }
    }
    if json {
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
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"turns": arr})).unwrap_or_default()
        );
        return Ok(());
    }
    if turns.is_empty() {
        println!("(no turns)");
        return Ok(());
    }
    println!(
        "turns on session `{raw}` ({} turn{}):",
        turns.len(),
        if turns.len() == 1 { "" } else { "s" }
    );
    println!("  turn  user_seq  asst_seq  tools  duration_ms");
    for t in &turns {
        let dur = match (t.started_at, t.ended_at) {
            (Some(a), Some(b)) => (b - a) / 1000,
            _ => 0,
        };
        println!(
            "  {:>4}  {:>8}  {:>8}  {:>5}  {:>11}",
            t.index,
            t.user_seq.map(|s| s.to_string()).unwrap_or_else(|| "-".into()),
            t.assistant_seq
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".into()),
            t.tool_count,
            dur,
        );
    }
    Ok(())
}
