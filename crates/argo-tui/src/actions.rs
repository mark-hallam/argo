//! Side-effecting helpers shared by the input loop: spawn an
//! agent turn, reap a finished one, delete / rename / clone a
//! session. Lives in its own file so `input.rs` stays under the
//! 500-LOC budget as keybindings get added.
use crate::app::ViewerState;
use argo_agent_runner::{run_turn_with_events, MAX_TOOL_ROUNDS};
use argo_store::SessionStore;
use argo_types::{ArgoResult, MessageRole, SessionId};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

/// Spawn `argo_agent_runner::run_turn_with_events` against the
/// selected session, using the composer draft as the user prompt.
/// Stores the `JoinHandle` on `state` so the event loop can reap
/// it; returns the receive side of the event channel so the
/// caller can drain envelopes between key presses.
///
/// Returns `None` when no task was spawned (no session selected,
/// empty composer, or a turn is already in flight).
pub(crate) async fn spawn_agent_turn(
    store: &Arc<SessionStore>,
    cwd: &Path,
    state: &mut ViewerState,
) -> Option<UnboundedReceiver<serde_json::Value>> {
    if state.agent_running() {
        return None;
    }
    let Some(session) = state.selected_session() else {
        state.composer_send();
        return None;
    };
    let sid = SessionId(session.id.0);
    let draft = state.composer.clone();
    if draft.is_empty() {
        state.composer_send();
        return None;
    }
    let store_h = Arc::clone(store);
    let cwd_h = cwd.to_path_buf();
    let user_text = draft.clone();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = tokio::spawn(async move {
        run_turn_with_events(
            &store_h,
            &cwd_h,
            sid,
            Some(&user_text),
            false,
            MAX_TOOL_ROUNDS,
            Some(&tx),
        )
        .await
        .map(|_| ())
    });
    state.last_agent_error = None;
    state.clear_live_events();
    state.agent_handle = Some(handle);
    state.agent_started_at = Some(std::time::Instant::now());
    state.composer_send();
    Some(rx)
}

/// Poll the in-flight agent task; if it has finished, take the
/// handle, await it (non-blocking — it's already done), and stash
/// any error string on `state`. Returns `true` when a task was
/// reaped this tick (so the caller can drop the events receiver),
/// `false` otherwise.
pub(crate) async fn reap_agent(state: &mut ViewerState) -> bool {
    if !state.agent_handle.as_ref().is_some_and(|h| h.is_finished()) {
        return false;
    }
    let Some(handle) = state.agent_handle.take() else {
        return false;
    };
    state.agent_started_at = None;
    match handle.await {
        Ok(Ok(())) => state.last_agent_error = None,
        Ok(Err(e)) => state.last_agent_error = Some(e.to_string()),
        Err(join) => state.last_agent_error = Some(format!("agent task panic: {join}")),
    }
    state.messages_loaded = false;
    true
}

/// Delete the given session via `SessionStore::delete_session`,
/// re-fetch the session list, and clamp the selection cursor.
/// Errors get stashed on `state.last_agent_error` so the footer
/// surfaces them.
pub(crate) async fn apply_delete(
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
    sid: SessionId,
) {
    if let Err(e) = store.delete_session(sid).await {
        state.last_agent_error = Some(format!("delete: {e}"));
        return;
    }
    let sessions = match store.list_all_sessions(200).await {
        Ok(s) => s,
        Err(e) => {
            state.last_agent_error = Some(format!("delete refresh: {e}"));
            return;
        }
    };
    state.sessions = sessions;
    if !state.sessions.is_empty() {
        state.selected = state.selected.min(state.sessions.len() - 1);
    } else {
        state.selected = 0;
    }
    state.selected_message_count = None;
    state.selected_messages.clear();
    state.messages_loaded = false;
}

/// Append a system-role `[steer] <text>` message to the
/// selected session, mirroring `argo steer <id> <text>`. The
/// next agent turn will see it as a system-prefix steer. No-op
/// on empty input or when nothing is selected.
pub(crate) async fn apply_steer(
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
    text: &str,
) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    let Some(session) = state.selected_session() else {
        return;
    };
    let sid = SessionId(session.id.0);
    let body = format!("[steer] {trimmed}");
    let content = serde_json::json!([{ "type": "text", "text": body }]);
    if let Err(e) = store
        .append_message(sid, MessageRole::System, &content.to_string())
        .await
    {
        state.last_agent_error = Some(format!("steer: {e}"));
        return;
    }
    // Force a re-fetch so the new system row appears immediately.
    state.messages_loaded = false;
}

/// Persist a new title against the selected session and refresh
/// the in-memory session list so the change is visible. No-op
/// when nothing is selected. Errors get stashed on
/// `state.last_agent_error` so the footer surfaces them.
pub(crate) async fn apply_rename(
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
    new_title: &str,
) {
    let Some(session) = state.selected_session() else {
        return;
    };
    let sid = SessionId(session.id.0);
    if let Err(e) = store.set_session_title(sid, new_title).await {
        state.last_agent_error = Some(format!("rename: {e}"));
        return;
    }
    let trimmed = new_title.trim();
    let next: Option<String> = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };
    if let Some(idx) = state.sessions.iter().position(|s| s.id.0 == sid.0) {
        state.sessions[idx].title = next;
    }
}

/// Clone the selected session's `(provider, model)` into a fresh
/// session row, refresh the list, and select the new row. The
/// title is "untitled" — the user can rename via \`r\`.
///
/// Returns `Ok(())` even when no session is selected — the
/// keybinding has nothing to clone from in that case and the
/// footer hint already nudges the user to use `argo new`.
pub(crate) async fn create_cloned_session(
    store: &Arc<SessionStore>,
    cwd: &Path,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    let Some(template) = state.selected_session() else {
        state.last_agent_error =
            Some("no session selected to clone — use `argo new` first".to_string());
        return Ok(());
    };
    let model = template.model.clone();
    let provider = template.provider.clone();
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, ws) = store.ensure_workspace(name, &root).await?;
    let new_id = store
        .create_session(tenant, ws, &model, &provider, Some("untitled"))
        .await?;
    let sessions = store.list_all_sessions(200).await?;
    state.sessions = sessions;
    state.selected = state
        .sessions
        .iter()
        .position(|s| s.id == new_id)
        .unwrap_or(0);
    state.selected_message_count = None;
    state.selected_messages.clear();
    state.messages_loaded = false;
    state.last_agent_error = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_cloned_session_clones_provider_and_model() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let (tenant, ws) = store.ensure_workspace("ws", "/tmp/x").await.unwrap();
        let original = store
            .create_session(tenant, ws, "claude-x", "anthropic", Some("first"))
            .await
            .unwrap();
        let sessions = store.list_all_sessions(200).await.unwrap();
        let mut state = ViewerState::new(sessions);

        create_cloned_session(&store, std::path::Path::new("/tmp/x"), &mut state)
            .await
            .unwrap();

        assert_eq!(state.sessions.len(), 2, "list grows by one");
        let new = state.selected_session().expect("new session selected");
        assert_eq!(new.model, "claude-x");
        assert_eq!(new.provider, "anthropic");
        assert_eq!(new.title.as_deref(), Some("untitled"));
        assert_ne!(new.id, original, "new session has a fresh id");
    }

    #[tokio::test]
    async fn create_cloned_session_no_op_on_empty_list() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let mut state = ViewerState::new(Vec::new());
        create_cloned_session(&store, std::path::Path::new("/tmp/x"), &mut state)
            .await
            .unwrap();
        assert!(state.sessions.is_empty());
        assert!(state.last_agent_error.as_deref().is_some());
    }

    #[tokio::test]
    async fn apply_rename_persists_and_patches_in_memory_list() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let (tenant, ws) = store.ensure_workspace("ws", "/tmp/r").await.unwrap();
        store
            .create_session(tenant, ws, "m", "p", Some("old name"))
            .await
            .unwrap();
        let sessions = store.list_all_sessions(200).await.unwrap();
        let mut state = ViewerState::new(sessions);

        apply_rename(&store, &mut state, "shiny new name").await;

        let s = state.selected_session().unwrap();
        assert_eq!(s.title.as_deref(), Some("shiny new name"));
        assert!(state.last_agent_error.is_none());

        let fresh = store.get_session(s.id).await.unwrap().unwrap();
        assert_eq!(fresh.title.as_deref(), Some("shiny new name"));
    }

    #[tokio::test]
    async fn apply_rename_with_empty_string_clears_title() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let (tenant, ws) = store.ensure_workspace("ws", "/tmp/r2").await.unwrap();
        store
            .create_session(tenant, ws, "m", "p", Some("named"))
            .await
            .unwrap();
        let sessions = store.list_all_sessions(200).await.unwrap();
        let mut state = ViewerState::new(sessions);

        apply_rename(&store, &mut state, "   ").await;

        assert!(state.selected_session().unwrap().title.is_none());
    }

    #[tokio::test]
    async fn apply_delete_removes_row_and_clamps_selection() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let (tenant, ws) = store.ensure_workspace("ws", "/tmp/d").await.unwrap();
        let a = store
            .create_session(tenant, ws, "m", "p", Some("a"))
            .await
            .unwrap();
        let b = store
            .create_session(tenant, ws, "m", "p", Some("b"))
            .await
            .unwrap();
        let sessions = store.list_all_sessions(200).await.unwrap();
        let mut state = ViewerState::new(sessions);

        apply_delete(&store, &mut state, b).await;

        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.sessions[0].id, a);
        assert_eq!(state.selected, 0);
        assert!(state.last_agent_error.is_none());
    }

    #[tokio::test]
    async fn apply_steer_persists_system_role_message() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let (tenant, ws) = store.ensure_workspace("ws", "/tmp/s").await.unwrap();
        store
            .create_session(tenant, ws, "m", "p", Some("name"))
            .await
            .unwrap();
        let sessions = store.list_all_sessions(200).await.unwrap();
        let mut state = ViewerState::new(sessions);
        let sid = state.selected_session().unwrap().id;

        apply_steer(&store, &mut state, "be more terse").await;

        let msgs = store.list_messages(sid).await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, "system");
        assert!(msgs[0].content_json.contains("[steer] be more terse"));
        assert!(state.last_agent_error.is_none());
    }

    #[tokio::test]
    async fn apply_steer_no_op_on_empty() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let (tenant, ws) = store.ensure_workspace("ws", "/tmp/s2").await.unwrap();
        store
            .create_session(tenant, ws, "m", "p", Some("n"))
            .await
            .unwrap();
        let sessions = store.list_all_sessions(200).await.unwrap();
        let mut state = ViewerState::new(sessions);
        let sid = state.selected_session().unwrap().id;

        apply_steer(&store, &mut state, "   ").await;

        assert!(store.list_messages(sid).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn apply_delete_handles_last_row() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let (tenant, ws) = store.ensure_workspace("ws", "/tmp/d2").await.unwrap();
        let a = store
            .create_session(tenant, ws, "m", "p", Some("only"))
            .await
            .unwrap();
        let sessions = store.list_all_sessions(200).await.unwrap();
        let mut state = ViewerState::new(sessions);

        apply_delete(&store, &mut state, a).await;

        assert!(state.sessions.is_empty());
        assert!(state.selected_session().is_none());
    }
}
