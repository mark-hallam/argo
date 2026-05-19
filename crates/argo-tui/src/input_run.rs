//! Helper functions extracted from `input::run` to keep that
//! function under the 80-LOC budget. Each helper owns one phase
//! of the per-iteration loop body.

use crate::actions::{
    apply_delete, apply_rename, apply_steer, create_cloned_session, reap_agent, spawn_agent_turn,
};
use crate::app::{PaneFocus, ViewerState};
use crate::input_helpers::{
    force_refresh_open_overlays, pick_yank_at_cursor, pick_yank_text, scan_dir_basenames,
};
use crate::render::draw;
use crate::viewer::{fetch_message_count, fetch_messages};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedReceiver;

/// Outcome of a key dispatch helper — `Consumed` means the caller
/// should `continue` the event loop. `Quit` ends the session viewer.
pub(crate) enum KeyOutcome {
    Consumed,
    Quit,
}

/// One full draw + 1-Hz refresh tick. Drains live events, reaps a
/// finished agent run (clearing `events_rx`), repaints, and on the
/// 1-second cadence re-fetches messages and any open list overlays.
pub(crate) async fn tick_refresh(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    store: &Arc<SessionStore>,
    cwd: &Path,
    state: &mut ViewerState,
    last_refresh: &mut Instant,
    events_rx: &mut Option<UnboundedReceiver<serde_json::Value>>,
) -> ArgoResult<()> {
    if let Some(rx) = events_rx.as_mut() {
        while let Ok(env) = rx.try_recv() {
            state.push_live_event(&env);
        }
    }
    if reap_agent(state).await {
        *events_rx = None;
    }
    terminal
        .draw(|frame| draw(frame, state))
        .map_err(|e| ArgoError::Provider(format!("tui draw: {e}")))?;
    if last_refresh.elapsed() >= Duration::from_secs(1) {
        state.messages_loaded = false;
        fetch_messages(store, state).await?;
        refresh_open_overlays_one_hz(store, cwd, state).await;
        crate::input_dashboard::refresh_dashboard(state, store).await;
        crate::approval_tray::refresh_open_tray(state).await;
        *last_refresh = Instant::now();
    }
    Ok(())
}

/// Re-pull the data behind any open list overlays so externally-
/// driven changes (e.g. `argo cron add` from a sibling shell) show
/// up on the next 1-Hz tick without the user closing + re-opening.
async fn refresh_open_overlays_one_hz(
    store: &Arc<SessionStore>,
    cwd: &Path,
    state: &mut ViewerState,
) {
    if state.forge_runs.is_some() {
        let rows = store.list_forge_runs(50).await.unwrap_or_default();
        state.forge_runs = Some(rows);
    }
    if state.cron_overlay.is_some() {
        state.cron_overlay = Some(scan_dir_basenames(&cwd.join("cron").join("jobs"), "json"));
    }
    if state.providers_overlay.is_some() {
        state.providers_overlay = Some(scan_dir_basenames(
            &cwd.join("manifests").join("providers"),
            "toml",
        ));
    }
}

/// Filter-mode keys: type characters to narrow the message list,
/// Enter to commit and exit filter mode, Esc to clear.
pub(crate) async fn handle_filter_key(
    key: KeyEvent,
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
) -> ArgoResult<()> {
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
    Ok(())
}

/// Rename-mode keys: type freely, Enter persists, Esc cancels.
pub(crate) async fn handle_rename_key(
    key: KeyEvent,
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
) {
    match key.code {
        KeyCode::Esc => state.rename_cancel(),
        KeyCode::Enter => {
            if let Some(new_title) = state.rename_take() {
                apply_rename(store, state, &new_title).await;
            }
        }
        KeyCode::Backspace => state.rename_pop(),
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.rename_push(c);
        }
        _ => {}
    }
}

/// Steer-mode keys: same shape as rename, Enter persists a
/// system-role `[steer] <text>` message.
pub(crate) async fn handle_steer_key(
    key: KeyEvent,
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
) {
    match key.code {
        KeyCode::Esc => state.steer_cancel(),
        KeyCode::Enter => {
            if let Some(text) = state.steer_take() {
                apply_steer(store, state, &text).await;
            }
        }
        KeyCode::Backspace => state.steer_pop(),
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.steer_push(c);
        }
        _ => {}
    }
}

/// Composer-mode keys: Enter spawns a live agent turn whose event
/// stream the caller then drains via `events_rx`.
pub(crate) async fn handle_composer_key(
    key: KeyEvent,
    store: &Arc<SessionStore>,
    cwd: &Path,
    state: &mut ViewerState,
) -> Option<UnboundedReceiver<serde_json::Value>> {
    match key.code {
        KeyCode::Esc => {
            state.composer_cancel();
            None
        }
        KeyCode::Enter => spawn_agent_turn(store, cwd, state).await,
        KeyCode::Backspace => {
            state.composer_pop();
            None
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.composer_push(c);
            None
        }
        _ => None,
    }
}

/// Bottom-tier navigation keys — quit, focus toggle, list scrolling,
/// session lifecycle (`d`, `e`, `n`, `r`, `s`), yank, overlay openers
/// that route to per-pane behaviour. Returns `Quit` for `q`/Esc/Ctrl-C
/// so the caller breaks out of the event loop.
pub(crate) async fn handle_navigation_key(
    key: KeyEvent,
    store: &Arc<SessionStore>,
    cwd: &Path,
    state: &mut ViewerState,
    last_refresh: &mut Instant,
) -> ArgoResult<KeyOutcome> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(KeyOutcome::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Ok(KeyOutcome::Quit);
        }
        KeyCode::Tab => state.toggle_focus(),
        KeyCode::Char('C') => state.clear_live_events(),
        KeyCode::Char('R') => {
            state.messages_loaded = false;
            fetch_message_count(store, state).await?;
            fetch_messages(store, state).await?;
            force_refresh_open_overlays(store, state).await;
            *last_refresh = Instant::now();
        }
        KeyCode::Char('o')
            if matches!(state.focus, PaneFocus::Messages) && !state.selected_messages.is_empty() =>
        {
            state.message_overlay =
                Some(state.messages_scroll.min(state.selected_messages.len() - 1));
        }
        KeyCode::Char('/') => state.enter_filter_mode(),
        KeyCode::Char('i') => state.composer_mode = true,
        KeyCode::Char('y') => {
            if let Some(text) = pick_yank_text(state) {
                crate::clipboard::write_to_clipboard(&text);
            }
        }
        KeyCode::Char('Y') => {
            if let Some(text) = pick_yank_at_cursor(state) {
                crate::clipboard::write_to_clipboard(&text);
            }
        }
        _ => handle_session_lifecycle_or_scroll(key, store, cwd, state).await?,
    }
    Ok(KeyOutcome::Consumed)
}

/// Tail of the navigation match — session lifecycle (`n`/`r`/`s`/
/// `d`/`e`) and the list-pane scrolling family (`j`/`k`/PgDn/PgUp/
/// Home/End). Extracted so the parent handler stays under the
/// 80-LOC budget.
async fn handle_session_lifecycle_or_scroll(
    key: KeyEvent,
    store: &Arc<SessionStore>,
    cwd: &Path,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    match key.code {
        KeyCode::Char('n') => {
            if let Err(e) = create_cloned_session(store, cwd, state).await {
                state.last_agent_error = Some(format!("new session: {e}"));
            }
        }
        KeyCode::Char('r') if state.selected_session().is_some() => state.enter_rename_mode(),
        KeyCode::Char('s') if state.selected_session().is_some() => state.enter_steer_mode(),
        KeyCode::Char('d') => {
            if let Some(s) = state.selected_session() {
                state.pending_delete = Some(SessionId(s.id.0));
            }
        }
        KeyCode::Char('e') => mark_session_ended(store, state).await,
        KeyCode::Down | KeyCode::Char('j') => scroll_down(key, store, state).await?,
        KeyCode::Up | KeyCode::Char('k') => scroll_up(key, store, state).await?,
        KeyCode::PageDown => scroll_down_n(store, state, 10).await?,
        KeyCode::PageUp => scroll_up_n(store, state, 10).await?,
        KeyCode::Home | KeyCode::Char('g') => scroll_first(store, state).await?,
        KeyCode::End | KeyCode::Char('G') => scroll_last(store, state).await?,
        _ => {}
    }
    Ok(())
}

/// Stamp `ended_at` on the selected session and force a refresh so
/// the marker (`●→○`) flips on the next draw. Idempotent — re-
/// pressing on an already-ended session re-stamps but causes no
/// other behaviour change.
async fn mark_session_ended(store: &Arc<SessionStore>, state: &mut ViewerState) {
    if let Some(s) = state.selected_session() {
        let sid = SessionId(s.id.0);
        if let Err(e) = store.end_session(sid).await {
            state.last_agent_error = Some(format!("end_session: {e}"));
        } else {
            state.sessions = store
                .list_all_sessions(crate::viewer::MAX_SESSIONS_LOAD)
                .await
                .unwrap_or_default();
        }
    }
}

async fn scroll_down(
    _key: KeyEvent,
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    match state.focus {
        PaneFocus::Sessions => {
            state.next();
            fetch_message_count(store, state).await?;
            fetch_messages(store, state).await?;
        }
        PaneFocus::Messages => state.scroll_messages_down(),
    }
    Ok(())
}

async fn scroll_up(
    _key: KeyEvent,
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    match state.focus {
        PaneFocus::Sessions => {
            state.prev();
            fetch_message_count(store, state).await?;
            fetch_messages(store, state).await?;
        }
        PaneFocus::Messages => state.scroll_messages_up(),
    }
    Ok(())
}

async fn scroll_down_n(
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
    n: usize,
) -> ArgoResult<()> {
    match state.focus {
        PaneFocus::Sessions => {
            state.next_n(n);
            fetch_message_count(store, state).await?;
            fetch_messages(store, state).await?;
        }
        PaneFocus::Messages => state.scroll_messages_down_n(n),
    }
    Ok(())
}

async fn scroll_up_n(
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
    n: usize,
) -> ArgoResult<()> {
    match state.focus {
        PaneFocus::Sessions => {
            state.prev_n(n);
            fetch_message_count(store, state).await?;
            fetch_messages(store, state).await?;
        }
        PaneFocus::Messages => state.scroll_messages_up_n(n),
    }
    Ok(())
}

async fn scroll_first(store: &Arc<SessionStore>, state: &mut ViewerState) -> ArgoResult<()> {
    match state.focus {
        PaneFocus::Sessions => {
            state.first();
            fetch_message_count(store, state).await?;
            fetch_messages(store, state).await?;
        }
        PaneFocus::Messages => state.scroll_messages_first(),
    }
    Ok(())
}

async fn scroll_last(store: &Arc<SessionStore>, state: &mut ViewerState) -> ArgoResult<()> {
    match state.focus {
        PaneFocus::Sessions => {
            state.last();
            fetch_message_count(store, state).await?;
            fetch_messages(store, state).await?;
        }
        PaneFocus::Messages => state.scroll_messages_last(),
    }
    Ok(())
}

/// Pending-delete confirm: `y` fires the delete, any other key
/// cancels. The pending session id is taken out of `state` so the
/// caller can hand a fresh attempt next iteration.
pub(crate) async fn handle_pending_delete(
    key: KeyEvent,
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
) {
    let sid = state.pending_delete.take();
    if matches!(key.code, KeyCode::Char('y')) {
        if let Some(sid) = sid {
            apply_delete(store, state, sid).await;
        }
    }
}
