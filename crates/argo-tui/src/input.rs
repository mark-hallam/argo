//! Crossterm event-loop for the TUI session viewer. Extracted
//! from `app.rs` to keep that file under the 500-LOC budget.
use crate::app::{fetch_message_count, fetch_messages, ViewerState};
use crate::render::draw;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::time::Duration;

pub(crate) async fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    store: &SessionStore,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    let mut last_refresh = std::time::Instant::now();
    loop {
        terminal
            .draw(|frame| draw(frame, state))
            .map_err(|e| ArgoError::Provider(format!("tui draw: {e}")))?;
        // Auto-refresh: every 1s, force a re-fetch of the
        // selected session's messages so newly-appended turns
        // (from `argo chat`, the gateway, or another TUI) appear
        // without manual reselect.
        if last_refresh.elapsed() >= Duration::from_secs(1) {
            state.messages_loaded = false;
            fetch_messages(store, state).await?;
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
                        fetch_message_count(store, state).await?;
                        fetch_messages(store, state).await?;
                    }
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.filter_push(c);
                        fetch_message_count(store, state).await?;
                        fetch_messages(store, state).await?;
                    }
                    _ => {}
                }
                continue;
            }
            // Composer mode: type freely, Enter persists the draft
            // as a user-role message on the selected session, Esc
            // cancels. (Driving a full agent-loop turn from the
            // TUI needs the chat helpers + a tokio handle threaded
            // in; the persisted user message + reload pattern is
            // the daemon-friendly half of that flow.)
            if state.composer_mode {
                match key.code {
                    KeyCode::Esc => state.composer_cancel(),
                    KeyCode::Enter => {
                        if let Some(session) = state.selected_session() {
                            let sid = SessionId(session.id.0);
                            let draft = state.composer.clone();
                            if !draft.is_empty() {
                                let content = serde_json::json!([{"type": "text", "text": draft}]);
                                let _ = store
                                    .append_message(
                                        sid,
                                        argo_types::MessageRole::User,
                                        &content.to_string(),
                                    )
                                    .await;
                                state.messages_loaded = false;
                                fetch_messages(store, state).await?;
                            }
                        }
                        state.composer_send();
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
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.prev();
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                KeyCode::PageDown => {
                    state.next_n(10);
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                KeyCode::PageUp => {
                    state.prev_n(10);
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    state.first();
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                KeyCode::End | KeyCode::Char('G') => {
                    state.last();
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                _ => {}
            }
        }
    }
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
