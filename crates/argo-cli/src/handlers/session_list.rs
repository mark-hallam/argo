//! `argo session list` + `argo session count` impls. Lifted out
//! of `session.rs` so that file stays under the 500-LOC budget
//! as more filter dimensions land. Per-phase filter / sort /
//! render helpers live in [`super::session_list_render`] so
//! `list_sessions` stays a thin orchestrator.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

#[allow(clippy::too_many_arguments)]
pub(super) async fn list_sessions(
    limit: u32,
    json: bool,
    format: &str,
    since_minutes: Option<i64>,
    older_than_minutes: Option<i64>,
    spent_min: Option<u64>,
    spent_max: Option<u64>,
    provider: Option<String>,
    model: Option<String>,
    status: String,
    title_contains: Option<String>,
    sort_by: String,
    vertical: Option<String>,
) -> ArgoResult<()> {
    use super::session_aggregates::{spend_per_session, tool_calls_per_session};
    use super::session_list_render as r;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let mut sessions = store.list_all_sessions(limit).await?;
    r::apply_session_filters(
        &mut sessions,
        since_minutes,
        older_than_minutes,
        provider.as_deref(),
        model.as_deref(),
        vertical.as_deref(),
        &status,
        title_contains.as_deref(),
    )?;
    let spend_by_sid = spend_per_session(&store).await.unwrap_or_default();
    let tool_calls_by_sid = tool_calls_per_session(&store).await.unwrap_or_default();
    r::apply_sort_and_spent(&mut sessions, &spend_by_sid, &sort_by, spent_min, spent_max)?;
    let pick = if json { "json" } else { format };
    match pick {
        "json" => {
            r::render_json(&sessions, &spend_by_sid, &tool_calls_by_sid)?;
            return Ok(());
        }
        "markdown" => {
            r::render_markdown(&sessions, &spend_by_sid, &tool_calls_by_sid);
            return Ok(());
        }
        "text" => {}
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown format '{other}' (expected 'text', 'json', or 'markdown')"
            )));
        }
    }
    if sessions.is_empty() {
        println!("(no sessions in {})", db.display());
        return Ok(());
    }
    r::render_text(&sessions, &spend_by_sid, &tool_calls_by_sid);
    Ok(())
}

/// `argo session count` with the same filter set as `list`.
/// Caps the fetch at 10,000 rows for safety.
#[allow(clippy::too_many_arguments)]
pub(super) async fn count_sessions(
    status: String,
    since_minutes: Option<i64>,
    older_than_minutes: Option<i64>,
    provider: Option<String>,
    model: Option<String>,
    title_contains: Option<String>,
    spent_min: Option<u64>,
    spent_max: Option<u64>,
    json: bool,
) -> ArgoResult<()> {
    use super::session_aggregates::spend_per_session;
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let mut sessions = store.list_all_sessions(10_000).await?;
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
    if let Some(needle) = title_contains.as_deref() {
        let lower = needle.to_lowercase();
        sessions.retain(|s| {
            s.title
                .as_deref()
                .map(|t| t.to_lowercase().contains(&lower))
                .unwrap_or(false)
        });
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
    if spent_min.is_some() || spent_max.is_some() {
        let spend_by_sid = spend_per_session(&store).await.unwrap_or_default();
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
    let count = sessions.len();
    if json {
        let payload = serde_json::json!({
            "count": count,
            "status": status,
            "provider": provider,
            "model": model,
            "title_contains": title_contains,
            "spent_min": spent_min,
            "spent_max": spent_max,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!("{count}");
    }
    Ok(())
}
