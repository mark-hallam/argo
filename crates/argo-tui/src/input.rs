//! Crossterm event-loop for the TUI session viewer. Extracted
//! from `app.rs` to keep that file under the 500-LOC budget.
use crate::app::ViewerState;
use crate::viewer::{fetch_message_count, fetch_messages};
use crate::render::draw;
use argo_agent_runner::{run_turn_with_events, MAX_TOOL_ROUNDS};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

pub(crate) async fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    store: Arc<SessionStore>,
    cwd: PathBuf,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    let mut last_refresh = std::time::Instant::now();
    // Live-event channel for the in-flight agent task. `Some` only
    // while a task is running; reset to `None` after `reap_agent`.
    let mut events_rx: Option<UnboundedReceiver<serde_json::Value>> = None;
    loop {
        // Drain any events the spawned `run_turn` task has emitted
        // since the last tick. The receiver is closed when the task
        // drops its sender (i.e. when the run finishes).
        if let Some(rx) = events_rx.as_mut() {
            while let Ok(env) = rx.try_recv() {
                state.push_live_event(&env);
            }
        }
        // Reap a finished agent turn before drawing so the footer
        // shows the right "running"/"done" state and any error
        // surfaces immediately.
        if reap_agent(state).await {
            // Drop the receiver once the task completes; any
            // un-drained events were collected above.
            events_rx = None;
        }
        terminal
            .draw(|frame| draw(frame, state))
            .map_err(|e| ArgoError::Provider(format!("tui draw: {e}")))?;
        // Auto-refresh: every 1s, force a re-fetch of the
        // selected session's messages so newly-appended turns
        // (from `argo chat`, the gateway, or another TUI) appear
        // without manual reselect.
        if last_refresh.elapsed() >= Duration::from_secs(1) {
            state.messages_loaded = false;
            fetch_messages(&store, state).await?;
            last_refresh = std::time::Instant::now();
        }
        // 200ms poll keeps the loop responsive without a busy-spin;
        // ratatui's recommended idiom is poll-then-read.
        if !event::poll(Duration::from_millis(200))
            .map_err(|e| ArgoError::Provider(format!("tui poll: {e}")))?
        {
            continue;
        }
        let ev = event::read().map_err(|e| ArgoError::Provider(format!("tui read: {e}")))?;
        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            // While typing the filter, intercept keystrokes:
            // letters/digits/punct extend the filter, Backspace
            // removes a char, Enter applies + exits filter mode,
            // Esc clears + exits.
            if state.filter_mode {
                match key.code {
                    KeyCode::Esc => state.filter_clear(),
                    KeyCode::Enter => state.exit_filter_mode(),
                    KeyCode::Backspace => {
                        state.filter_pop();
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.filter_push(c);
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    _ => {}
                }
                continue;
            }
            // Rename mode: type freely, Enter persists the new
            // title via `SessionStore::set_session_title`, Esc
            // cancels. Captures keystrokes before composer/normal
            // so 'r' chars while in rename go to the buffer.
            if state.rename_mode() {
                match key.code {
                    KeyCode::Esc => state.rename_cancel(),
                    KeyCode::Enter => {
                        if let Some(new_title) = state.rename_take() {
                            apply_rename(&store, state, &new_title).await;
                        }
                    }
                    KeyCode::Backspace => state.rename_pop(),
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.rename_push(c);
                    }
                    _ => {}
                }
                continue;
            }
            // Composer mode: type freely, Enter spawns a live agent
            // turn (persisting the user message + driving the run
            // loop in a background task), Esc cancels.
            if state.composer_mode {
                match key.code {
                    KeyCode::Esc => state.composer_cancel(),
                    KeyCode::Enter => {
                        events_rx = spawn_agent_turn(&store, &cwd, state).await;
                    }
                    KeyCode::Backspace => state.composer_pop(),
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.composer_push(c);
                    }
                    _ => {}
                }
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                KeyCode::Char('/') => {
                    state.enter_filter_mode();
                }
                KeyCode::Char('i') => {
                    state.composer_mode = true;
                }
                KeyCode::Char('y') => {
                    // OSC-52: copy the selected session's last
                    // message body (or, if no messages yet, the
                    // session id) to the system clipboard.
                    if let Some(text) = pick_yank_text(state) {
                        crate::clipboard::write_to_clipboard(&text);
                    }
                }
                KeyCode::Char('n') => {
                    // Clone the selected session's provider+model
                    // into a fresh session row. No-op when nothing
                    // is selected — the user has nothing to copy
                    // from, so the footer hint nudges them to
                    // `argo new` instead.
                    if let Err(e) = create_cloned_session(&store, &cwd, state).await {
                        state.last_agent_error = Some(format!("new session: {e}"));
                    }
                }
                KeyCode::Char('r') if state.selected_session().is_some() => {
                    // Enter rename mode for the selected session.
                    // No-op when nothing is selected.
                    state.enter_rename_mode();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    state.next();
                    fetch_message_count(&store, state).await?;
                    fetch_messages(&store, state).await?;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.prev();
                    fetch_message_count(&store, state).await?;
                    fetch_messages(&store, state).await?;
                }
                KeyCode::PageDown => {
                    state.next_n(10);
                    fetch_message_count(&store, state).await?;
                    fetch_messages(&store, state).await?;
                }
                KeyCode::PageUp => {
                    state.prev_n(10);
                    fetch_message_count(&store, state).await?;
                    fetch_messages(&store, state).await?;
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    state.first();
                    fetch_message_count(&store, state).await?;
                    fetch_messages(&store, state).await?;
                }
                KeyCode::End | KeyCode::Char('G') => {
                    state.last();
                    fetch_message_count(&store, state).await?;
                    fetch_messages(&store, state).await?;
                }
                _ => {}
            }
        }
    }
}

/// Spawn `argo_agent_runner::run_turn_with_events` against the
/// selected session, using the composer draft as the user prompt.
/// Stores the `JoinHandle` on `state` so the event loop can reap
/// it; returns the receive side of the event channel so the
/// caller can drain envelopes between key presses.
///
/// Returns `None` when no task was spawned (no session selected,
/// empty composer, or a turn is already in flight).
async fn spawn_agent_turn(
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
    state.composer_send();
    Some(rx)
}

/// Poll the in-flight agent task; if it has finished, take the
/// handle, await it (non-blocking — it's already done), and
/// stash any error string on `state`. Returns `true` when a task
/// was reaped this tick (so the caller can drop the events
/// receiver), `false` otherwise.
async fn reap_agent(state: &mut ViewerState) -> bool {
    if !state.agent_handle.as_ref().is_some_and(|h| h.is_finished()) {
        return false;
    }
    let Some(handle) = state.agent_handle.take() else {
        return false;
    };
    match handle.await {
        Ok(Ok(())) => state.last_agent_error = None,
        Ok(Err(e)) => state.last_agent_error = Some(e.to_string()),
        Err(join) => state.last_agent_error = Some(format!("agent task panic: {join}")),
    }
    // Force a re-fetch so the assistant reply (and any tool rows)
    // appear in the messages pane right away rather than waiting
    // for the 1s tick.
    state.messages_loaded = false;
    true
}

/// Persist a new title against the selected session and refresh
/// the in-memory session list so the change is visible. No-op
/// when nothing is selected. Errors get stashed on
/// `state.last_agent_error` so the footer surfaces them.
async fn apply_rename(store: &Arc<SessionStore>, state: &mut ViewerState, new_title: &str) {
    let Some(session) = state.selected_session() else {
        return;
    };
    let sid = SessionId(session.id.0);
    if let Err(e) = store.set_session_title(sid, new_title).await {
        state.last_agent_error = Some(format!("rename: {e}"));
        return;
    }
    // Patch the in-memory list so we don't need a full re-fetch.
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
/// title is "untitled" — the user can rename via `argo session
/// rename` (CLI) or just compose into it. Returns the new
/// session id on success.
///
/// Returns `Ok(())` even when no session is selected — the
/// keybinding has nothing to clone from in that case and the
/// footer hint already nudges the user to use `argo new`.
async fn create_cloned_session(
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
    // Re-fetch the session list and select the freshly-created
    // row. `list_all_sessions` is reverse-chronological so the
    // new row should land at index 0.
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

/// Choose what `y` copies: prefer the most recent message body,
/// fall back to the selected session id when the messages pane
/// is empty.
fn pick_yank_text(state: &ViewerState) -> Option<String> {
    if let Some(last) = state.selected_messages.last() {
        let parsed: serde_json::Value =
            serde_json::from_str(&last.content_json).unwrap_or(serde_json::Value::Null);
        if let Some(t) = parsed
            .as_array()
            .and_then(|arr| arr.iter().find_map(|b| b.get("text").and_then(|v| v.as_str())))
        {
            return Some(t.to_string());
        }
    }
    state.selected_session().map(|s| s.id.0.to_string())
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

        // Re-open the row from the DB to confirm persistence.
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
}
