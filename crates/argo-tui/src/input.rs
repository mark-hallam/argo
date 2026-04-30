//! Crossterm event-loop for the TUI session viewer. Side-effecting
//! helpers (spawn agent turn, reap, delete, rename, clone) live in
//! the sibling `actions.rs` so this file stays under 500 LOC as
//! keybindings get added.
use crate::actions::{
    apply_delete, apply_rename, apply_steer, create_cloned_session, reap_agent, spawn_agent_turn,
};
use crate::app::{PaneFocus, ViewerState};
use crate::render::draw;
use crate::viewer::{fetch_message_count, fetch_messages};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::path::PathBuf;
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
    let mut events_rx: Option<UnboundedReceiver<serde_json::Value>> = None;
    loop {
        if let Some(rx) = events_rx.as_mut() {
            while let Ok(env) = rx.try_recv() {
                state.push_live_event(&env);
            }
        }
        if reap_agent(state).await {
            events_rx = None;
        }
        terminal
            .draw(|frame| draw(frame, state))
            .map_err(|e| ArgoError::Provider(format!("tui draw: {e}")))?;
        if last_refresh.elapsed() >= Duration::from_secs(1) {
            state.messages_loaded = false;
            fetch_messages(&store, state).await?;
            last_refresh = std::time::Instant::now();
        }
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
            // Help overlay: any key closes it.
            if state.help_overlay {
                state.help_overlay = false;
                continue;
            }
            // Message-detail overlay: Esc/o/q close, anything
            // else is ignored.
            if state.message_overlay.is_some() {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('o') | KeyCode::Char('q')
                ) {
                    state.message_overlay = None;
                }
                continue;
            }
            // Forge-runs overlay: any key closes.
            if state.forge_runs.is_some() {
                state.forge_runs = None;
                continue;
            }
            // Pending-delete confirm: 'y' fires the delete, any
            // other key cancels.
            if state.pending_delete.is_some() {
                let sid = state.pending_delete.take();
                if matches!(key.code, KeyCode::Char('y')) {
                    if let Some(sid) = sid {
                        apply_delete(&store, state, sid).await;
                    }
                }
                continue;
            }
            // Rename mode: type freely, Enter persists, Esc cancels.
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
            // Steer mode: same shape as rename, Enter persists a
            // system-role `[steer] <text>` message.
            if state.steer_mode() {
                match key.code {
                    KeyCode::Esc => state.steer_cancel(),
                    KeyCode::Enter => {
                        if let Some(text) = state.steer_take() {
                            apply_steer(&store, state, &text).await;
                        }
                    }
                    KeyCode::Backspace => state.steer_pop(),
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.steer_push(c);
                    }
                    _ => {}
                }
                continue;
            }
            // Composer mode: Enter spawns a live agent turn.
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
                KeyCode::Tab => state.toggle_focus(),
                KeyCode::Char('?') => state.help_overlay = true,
                KeyCode::Char('F') => {
                    // Load the forge_run rows once and stash on
                    // state. Errors → empty list (UI shows
                    // "(no forge runs recorded yet)").
                    let rows = store.list_forge_runs(50).await.unwrap_or_default();
                    state.forge_runs = Some(rows);
                }
                KeyCode::Char('o')
                    if matches!(state.focus, PaneFocus::Messages)
                        && !state.selected_messages.is_empty() =>
                {
                    // Open the highlighted message in a detail
                    // overlay. The "highlight" is the topmost
                    // visible row — i.e. `messages_scroll` — so
                    // page up/down + j/k work as the cursor.
                    state.message_overlay = Some(
                        state
                            .messages_scroll
                            .min(state.selected_messages.len() - 1),
                    );
                }
                KeyCode::Char('/') => state.enter_filter_mode(),
                KeyCode::Char('i') => state.composer_mode = true,
                KeyCode::Char('y') => {
                    if let Some(text) = pick_yank_text(state) {
                        crate::clipboard::write_to_clipboard(&text);
                    }
                }
                KeyCode::Char('n') => {
                    if let Err(e) = create_cloned_session(&store, &cwd, state).await {
                        state.last_agent_error = Some(format!("new session: {e}"));
                    }
                }
                KeyCode::Char('r') if state.selected_session().is_some() => {
                    state.enter_rename_mode();
                }
                KeyCode::Char('s') if state.selected_session().is_some() => {
                    state.enter_steer_mode();
                }
                KeyCode::Char('d') => {
                    if let Some(s) = state.selected_session() {
                        state.pending_delete = Some(SessionId(s.id.0));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => match state.focus {
                    PaneFocus::Sessions => {
                        state.next();
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_down(),
                },
                KeyCode::Up | KeyCode::Char('k') => match state.focus {
                    PaneFocus::Sessions => {
                        state.prev();
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_up(),
                },
                KeyCode::PageDown => match state.focus {
                    PaneFocus::Sessions => {
                        state.next_n(10);
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_down_n(10),
                },
                KeyCode::PageUp => match state.focus {
                    PaneFocus::Sessions => {
                        state.prev_n(10);
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_up_n(10),
                },
                KeyCode::Home | KeyCode::Char('g') => match state.focus {
                    PaneFocus::Sessions => {
                        state.first();
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_first(),
                },
                KeyCode::End | KeyCode::Char('G') => match state.focus {
                    PaneFocus::Sessions => {
                        state.last();
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_last(),
                },
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
