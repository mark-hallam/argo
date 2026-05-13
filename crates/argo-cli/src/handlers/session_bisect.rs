//! `argo session bisect <id> <hypothesis>` — walk a session's
//! audit chain and report the first row whose payload contains
//! the hypothesis substring. Lives in its own module so
//! `session.rs` stays under the 500-LOC ceiling.
//!
//! Practical use: pass an error fragment that you saw in an
//! assistant reply and bisect tells you the first audit row
//! where that fragment showed up — usually a `ToolResult` or
//! `AssistantMessage`. From there `argo trace --turn` or
//! `argo session show` give surrounding context.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::{DateTime, Utc};

pub(crate) async fn run(raw_id: &str, hypothesis: &str) -> ArgoResult<()> {
    if hypothesis.trim().is_empty() {
        return Err(ArgoError::Provider(
            "argo session bisect: hypothesis cannot be empty".into(),
        ));
    }
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw_id)
        .map_err(|e| ArgoError::Schema(format!("invalid session id '{raw_id}': {e}")))?;
    let rows = store.list_audit_for_session(SessionId(id)).await?;
    if rows.is_empty() {
        return Err(ArgoError::Provider(format!(
            "no audit rows for session {raw_id}"
        )));
    }
    let needle = hypothesis;
    let hit = rows
        .iter()
        .enumerate()
        .find(|(_, r)| r.payload_json.contains(needle));
    let Some((idx, row)) = hit else {
        println!(
            "session bisect: '{needle}' not found across {} audit row(s) for session {raw_id}",
            rows.len()
        );
        return Err(ArgoError::Provider("hypothesis did not match".into()));
    };
    let dt = DateTime::<Utc>::from_timestamp_micros(row.created_at).unwrap_or_else(Utc::now);
    println!(
        "session bisect: first match at seq={seq} ({dt}, kind={kind})",
        seq = row.seq,
        kind = row.kind,
    );
    println!("  payload: {}", preview(&row.payload_json));
    // Print a small window of context (3 rows before, 3 after).
    let lo = idx.saturating_sub(3);
    let hi = (idx + 4).min(rows.len());
    println!("\n  context (seq, kind):");
    for r in &rows[lo..hi] {
        let marker = if r.seq == row.seq { "→" } else { " " };
        println!("    {marker} seq={:<5} {}", r.seq, r.kind);
    }
    Ok(())
}

fn preview(payload: &str) -> String {
    payload.chars().take(200).collect()
}
