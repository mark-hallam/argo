//! `argo session list` + `argo session count` impls. Lifted out
//! of `session.rs` so that file stays under the 500-LOC budget
//! as more filter dimensions land.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

#[allow(clippy::too_many_arguments)]
pub(super) async fn list_sessions(
    limit: u32,
    json: bool,
    since_minutes: Option<i64>,
    older_than_minutes: Option<i64>,
    spent_min: Option<u64>,
    spent_max: Option<u64>,
    provider: Option<String>,
    model: Option<String>,
    status: String,
    title_contains: Option<String>,
) -> ArgoResult<()> {
    use super::session_aggregates::{spend_per_session, tool_calls_per_session};
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let mut sessions = store.list_all_sessions(limit).await?;
    if let Some(minutes) = since_minutes {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes);
        sessions.retain(|s| s.started_at >= cutoff);
    }
    if let Some(minutes) = older_than_minutes {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes);
        sessions.retain(|s| s.started_at < cutoff);
    }
    if let Some(want) = provider.as_deref() {
        sessions.retain(|s| s.provider == want);
    }
    if let Some(want) = model.as_deref() {
        sessions.retain(|s| s.model == want);
    }
    match status.as_str() {
        "open" => sessions.retain(|s| s.ended_at.is_none()),
        "ended" => sessions.retain(|s| s.ended_at.is_some()),
        "all" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown --status '{other}' — use open|ended|all"
            )));
        }
    }
    if let Some(needle) = title_contains.as_deref() {
        let lower = needle.to_lowercase();
        sessions.retain(|s| {
            s.title
                .as_deref()
                .map(|t| t.to_lowercase().contains(&lower))
                .unwrap_or(false)
        });
    }
    let spend_by_sid = spend_per_session(&store).await.unwrap_or_default();
    let tool_calls_by_sid = tool_calls_per_session(&store).await.unwrap_or_default();
    if spent_min.is_some() || spent_max.is_some() {
        sessions.retain(|s| {
            let spent = spend_by_sid
                .get(s.id.0.as_bytes().as_slice())
                .copied()
                .unwrap_or(0);
            if let Some(min) = spent_min {
                if spent < min {
                    return false;
                }
            }
            if let Some(max) = spent_max {
                if spent > max {
                    return false;
                }
            }
            true
        });
    }
    if json {
        for s in &sessions {
            let spent = spend_by_sid
                .get(s.id.0.as_bytes().as_slice())
                .copied()
                .unwrap_or(0);
            let tcs = tool_calls_by_sid
                .get(s.id.0.as_bytes().as_slice())
                .copied()
                .unwrap_or(0);
            let mut value = serde_json::to_value(s)
                .map_err(|e| ArgoError::Provider(format!("serialize session: {e}")))?;
            if let serde_json::Value::Object(ref mut m) = value {
                m.insert("spent_usd_micros".into(), serde_json::Value::from(spent));
                m.insert("tool_call_count".into(), serde_json::Value::from(tcs));
            }
            let line = serde_json::to_string(&value)
                .map_err(|e| ArgoError::Provider(format!("serialize session: {e}")))?;
            println!("{line}");
        }
        return Ok(());
    }
    if sessions.is_empty() {
        println!("(no sessions in {})", db.display());
        return Ok(());
    }
    println!(
        "  {sid:<38} {prov:<12} {model:<24} {spend:>10} {tools:>5} title",
        sid = "session_id",
        prov = "provider",
        model = "model",
        spend = "spend",
        tools = "tools",
    );
    for s in sessions {
        let spent = spend_by_sid
            .get(s.id.0.as_bytes().as_slice())
            .copied()
            .unwrap_or(0);
        let tcs = tool_calls_by_sid
            .get(s.id.0.as_bytes().as_slice())
            .copied()
            .unwrap_or(0);
        // ● for open (no ended_at), ○ for ended — mirrors the
        // TUI session-list marker so `argo session list` and the
        // viewer agree at a glance.
        let marker = if s.ended_at.is_none() { "●" } else { "○" };
        println!(
            "{marker} {:<38} {:<12} {:<24} {:>10.4} {:>5} {}",
            s.id.0,
            short(&s.provider, 12),
            short(&s.model, 24),
            spent as f64 / 1_000_000.0,
            tcs,
            s.title.as_deref().unwrap_or("")
        );
    }
    Ok(())
}

/// `argo session count [--status] [--since-minutes] [--json]` —
/// print just the count of sessions matching the filters.
/// Useful for monitoring scripts and shell one-liners. Caps the
/// fetch at 10,000 rows for safety; that's far above any
/// expected workspace size.
pub(super) async fn count_sessions(
    status: String,
    since_minutes: Option<i64>,
    json: bool,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let mut sessions = store.list_all_sessions(10_000).await?;
    if let Some(minutes) = since_minutes {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(minutes);
        sessions.retain(|s| s.started_at >= cutoff);
    }
    match status.as_str() {
        "open" => sessions.retain(|s| s.ended_at.is_none()),
        "ended" => sessions.retain(|s| s.ended_at.is_some()),
        "all" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown --status '{other}' — use open|ended|all"
            )));
        }
    }
    let count = sessions.len();
    if json {
        let payload = serde_json::json!({"count": count, "status": status});
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!("{count}");
    }
    Ok(())
}

fn short(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
