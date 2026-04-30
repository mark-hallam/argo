//! `argo trace` — render an audit-log timeline for a session.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::{DateTime, Utc};

pub async fn run(turn: Option<String>, session: Option<String>) -> ArgoResult<()> {
    if turn.is_some() {
        eprintln!(
            "argo trace --turn <id>: per-turn span tree requires the daemon's tracing-collector binding (separate PR)."
        );
    }
    let Some(raw) = session else {
        return Err(ArgoError::Provider(
            "argo trace: pass --session <id> for the audit-log timeline (or --turn for the daemon-bound view).".into(),
        ));
    };
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(&raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    let rows = store.list_audit_for_session(SessionId(id)).await?;
    if rows.is_empty() {
        println!("(no audit rows for session {raw})");
        return Ok(());
    }
    let first = DateTime::<Utc>::from_timestamp_micros(rows[0].created_at).unwrap_or_else(Utc::now);
    println!("trace for {raw} ({} rows; start={})", rows.len(), first);
    let start = rows[0].created_at;
    for r in rows {
        let dt = DateTime::<Utc>::from_timestamp_micros(r.created_at).unwrap_or_else(Utc::now);
        let dt_ms = (r.created_at - start) / 1000;
        let preview = preview(&r.payload_json);
        println!(
            "  +{ms:>6}ms  seq={seq:<5} {dt}  {kind:<24} {preview}",
            ms = dt_ms,
            seq = r.seq,
            dt = dt.format("%H:%M:%S%.3f"),
            kind = r.kind,
            preview = preview,
        );
    }
    Ok(())
}

fn preview(payload: &str) -> String {
    // Render the payload as a one-line k=v summary if it parses as
    // a JSON object; otherwise fall back to the raw string trimmed
    // to 80 chars.
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
