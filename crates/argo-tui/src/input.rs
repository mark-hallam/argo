//! Crossterm event-loop for the TUI session viewer. Side-effecting
//! helpers (spawn agent turn, reap, delete, rename, clone) live in
//! the sibling `actions.rs` so this file stays under 500 LOC as
//! keybindings get added.
use crate::app::ViewerState;
use crate::input_overlay_dispatch::try_close_overlay;
use crate::input_run::{
    handle_composer_key, handle_filter_key, handle_navigation_key, handle_pending_delete,
    handle_rename_key, handle_steer_key, tick_refresh, KeyOutcome,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedReceiver;

pub(crate) async fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    store: Arc<SessionStore>,
    cwd: PathBuf,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    let mut last_refresh = Instant::now();
    let mut events_rx: Option<UnboundedReceiver<serde_json::Value>> = None;
    loop {
        tick_refresh(
            terminal,
            &store,
            &cwd,
            state,
            &mut last_refresh,
            &mut events_rx,
        )
        .await?;
        if !event::poll(Duration::from_millis(200))
            .map_err(|e| ArgoError::Provider(format!("tui poll: {e}")))?
        {
            continue;
        }
        let ev = event::read().map_err(|e| ArgoError::Provider(format!("tui read: {e}")))?;
        let Event::Key(key) = ev else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match dispatch_key_event(
            terminal,
            &store,
            &cwd,
            state,
            &mut last_refresh,
            &mut events_rx,
            key,
        )
        .await?
        {
            KeyOutcome::Quit => return Ok(()),
            KeyOutcome::Consumed => {}
        }
    }
}

/// Walk a key event through the modal-priority ladder: filter mode,
/// help overlays, approval tray, global one-shots, message-overlay
/// keys, spend-monitor keys, close-overlay dispatch, and finally
/// the navigation match. Returns `Consumed` when any handler
/// claimed the key (the caller `continue`s the event loop).
#[allow(clippy::too_many_arguments)]
async fn dispatch_key_event(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    store: &Arc<SessionStore>,
    cwd: &Path,
    state: &mut ViewerState,
    last_refresh: &mut Instant,
    events_rx: &mut Option<UnboundedReceiver<serde_json::Value>>,
    key: KeyEvent,
) -> ArgoResult<KeyOutcome> {
    if dispatch_text_modal(state, store, key).await? {
        return Ok(KeyOutcome::Consumed);
    }
    if dispatch_overlay_keys(terminal, store, cwd, state, key).await? {
        return Ok(KeyOutcome::Consumed);
    }
    if dispatch_input_mode(state, store, cwd, events_rx, key).await {
        return Ok(KeyOutcome::Consumed);
    }
    if crate::input_globals::dispatch_global_key(terminal, state, store, cwd, key).await? {
        return Ok(KeyOutcome::Consumed);
    }
    handle_navigation_key(key, store, cwd, state, last_refresh).await
}

/// Filter-mode + keybindings-help-overlay short-circuit. Returns
/// `true` when the key was consumed by one of those modals.
async fn dispatch_text_modal(
    state: &mut ViewerState,
    store: &Arc<SessionStore>,
    key: KeyEvent,
) -> ArgoResult<bool> {
    if state.filter_mode {
        handle_filter_key(key, store, state).await?;
        return Ok(true);
    }
    if state.keybindings_help_overlay {
        if matches!(
            crate::keybindings_help::handle_key(key.code),
            crate::keybindings_help::HelpOutcome::Close
        ) {
            state.keybindings_help_overlay = false;
        }
        return Ok(true);
    }
    Ok(false)
}

/// Approval tray, the `A` global one-shot, help overlay, message
/// overlay, spend-monitor key handling, and the overlay-close
/// dispatch (list overlays + aux overlays). Returns `true` on a hit.
async fn dispatch_overlay_keys(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    store: &Arc<SessionStore>,
    cwd: &Path,
    state: &mut ViewerState,
    key: KeyEvent,
) -> ArgoResult<bool> {
    if crate::approval_tray::try_handle_approval_tray(state, key).await? {
        return Ok(true);
    }
    if matches!(key.code, KeyCode::Char('A'))
        && key.modifiers == KeyModifiers::NONE
        && crate::input_globals::dispatch_global_key(terminal, state, store, cwd, key).await?
    {
        return Ok(true);
    }
    if state.help_overlay {
        state.help_overlay = false;
        return Ok(true);
    }
    if state.message_overlay.is_some() {
        crate::input_helpers::handle_message_overlay(key.code, state);
        return Ok(true);
    }
    if crate::spend_monitor::handle_spend_monitor_key(state, store, key.code).await? {
        return Ok(true);
    }
    if try_close_overlay(state, key.code) {
        return Ok(true);
    }
    Ok(crate::try_close_aux_overlay(state, key.code))
}

/// Pending-delete / rename / steer / composer modes. Returns
/// `true` when the active mode claims the key (the run loop then
/// `continue`s without falling through to the navigation match).
async fn dispatch_input_mode(
    state: &mut ViewerState,
    store: &Arc<SessionStore>,
    cwd: &Path,
    events_rx: &mut Option<UnboundedReceiver<serde_json::Value>>,
    key: KeyEvent,
) -> bool {
    if state.pending_delete.is_some() {
        handle_pending_delete(key, store, state).await;
        return true;
    }
    if state.rename_mode() {
        handle_rename_key(key, store, state).await;
        return true;
    }
    if state.steer_mode() {
        handle_steer_key(key, store, state).await;
        return true;
    }
    if state.composer_mode {
        if let Some(rx) = handle_composer_key(key, store, cwd, state).await {
            *events_rx = Some(rx);
        }
        return true;
    }
    false
}
