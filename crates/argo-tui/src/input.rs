//! Crossterm event-loop for the TUI session viewer. Extracted
//! from `app.rs` to keep that file under the 500-LOC budget.
use crate::app::{fetch_message_count, fetch_messages, ViewerState};
use crate::render::draw;
use argo_agent_runner::{run_turn, MAX_TOOL_ROUNDS};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub(crate) async fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    store: Arc<SessionStore>,
    cwd: PathBuf,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    let mut last_refresh = std::time::Instant::now();
    loop {
        // Reap a finished agent turn before drawing so the footer
        // shows the right "running"/"done" state and any error
        // surfaces immediately.
        reap_agent(state).await;
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
            // Composer mode: type freely, Enter spawns a live agent
            // turn (persisting the user message + driving the run
            // loop in a background task), Esc cancels.
            if state.composer_mode {
                match key.code {
                    KeyCode::Esc => state.composer_cancel(),
                    KeyCode::Enter => {
                        spawn_agent_turn(&store, &cwd, state).await;
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

/// Spawn `argo_agent_runner::run_turn` against the selected
/// session, using the composer draft as the user prompt. Stores
/// the `JoinHandle` on `state` so the event loop can reap it.
/// No-op if no session is selected, the composer is empty, or a
/// turn is already in flight.
async fn spawn_agent_turn(store: &Arc<SessionStore>, cwd: &Path, state: &mut ViewerState) {
    if state.agent_running() {
        return;
    }
    let Some(session) = state.selected_session() else {
        state.composer_send();
        return;
    };
    let sid = SessionId(session.id.0);
    let draft = state.composer.clone();
    if draft.is_empty() {
        state.composer_send();
        return;
    }
    let store_h = Arc::clone(store);
    let cwd_h = cwd.to_path_buf();
    let user_text = draft.clone();
    let handle = tokio::spawn(async move {
        run_turn(
            &store_h,
            &cwd_h,
            sid,
            Some(&user_text),
            false,
            MAX_TOOL_ROUNDS,
        )
        .await
        .map(|_| ())
    });
    state.last_agent_error = None;
    state.agent_handle = Some(handle);
    state.composer_send();
}

/// Poll the in-flight agent task; if it has finished, take the
/// handle, await it (non-blocking — it's already done), and
/// stash any error string on `state`.
async fn reap_agent(state: &mut ViewerState) {
    if !state.agent_handle.as_ref().is_some_and(|h| h.is_finished()) {
        return;
    }
    let Some(handle) = state.agent_handle.take() else {
        return;
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
