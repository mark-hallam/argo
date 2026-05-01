//! Terminal lifecycle + session/message fetching for the viewer.
//! Extracted from `app.rs` so that the `ViewerState` machinery
//! has room to grow without busting the 500-LOC budget.
use crate::app::ViewerState;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Stdout};
use std::path::PathBuf;
use std::sync::Arc;

/// Drive the TUI event loop. Quits on `q` or `Ctrl-C`. The
/// returned result is `Ok(())` on a clean quit, `Err` if terminal
/// setup or rendering blew up.
///
/// `cwd` is the workspace root passed to
/// `argo_agent_runner::run_turn` when the composer fires off a
/// live agent turn — it points at the directory holding
/// `manifests/providers/` so the runner can resolve the session's
/// provider id to a real transport.
///
/// # Errors
/// Returns [`ArgoError::Provider`] for terminal setup, draw, or
/// store-query failures.
pub async fn run_session_viewer(store: Arc<SessionStore>, cwd: PathBuf) -> ArgoResult<()> {
    let sessions = store.list_all_sessions(200).await?;
    let mut state = ViewerState::new(sessions);
    state.workspace_label = store.first_workspace_name().await.unwrap_or(None);
    fetch_message_count(&store, &mut state).await?;
    fetch_messages(&store, &mut state).await?;
    let mut terminal = setup_terminal()?;
    let outcome = crate::input::run(&mut terminal, store, cwd, &mut state).await;
    teardown_terminal(&mut terminal)?;
    outcome
}

/// Lazily fetch `message_count` for the selected session. No-op
/// when nothing is selected or the count is already cached.
pub(crate) async fn fetch_message_count(
    store: &SessionStore,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    if state.selected_message_count.is_some() {
        return Ok(());
    }
    let Some(session) = state.selected_session() else {
        return Ok(());
    };
    let count = store.message_count(SessionId(session.id.0)).await?;
    state.selected_message_count = Some(count);
    Ok(())
}

/// Hard cap on rows the messages pane keeps in memory. Bumped
/// from 50 → 500 in #298 — at one row per real message that's
/// enough headroom for a multi-hour session, while still
/// bounding TUI memory on a runaway loop.
pub(crate) const MAX_VIEWER_MESSAGES: usize = 500;

/// Fetch up to `MAX_VIEWER_MESSAGES` messages for the selected
/// session. No-op when nothing is selected or `messages_loaded`
/// is already `true`.
pub(crate) async fn fetch_messages(
    store: &SessionStore,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    if state.messages_loaded {
        return Ok(());
    }
    let Some(session) = state.selected_session() else {
        return Ok(());
    };
    let mut rows = store.list_messages(SessionId(session.id.0)).await?;
    rows.truncate(MAX_VIEWER_MESSAGES);
    state.selected_messages = rows;
    state.messages_loaded = true;
    Ok(())
}

fn setup_terminal() -> ArgoResult<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().map_err(|e| ArgoError::Provider(format!("enable_raw_mode: {e}")))?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)
        .map_err(|e| ArgoError::Provider(format!("enter_alt_screen: {e}")))?;
    Terminal::new(CrosstermBackend::new(out))
        .map_err(|e| ArgoError::Provider(format!("Terminal::new: {e}")))
}

fn teardown_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> ArgoResult<()> {
    disable_raw_mode().map_err(|e| ArgoError::Provider(format!("disable_raw_mode: {e}")))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| ArgoError::Provider(format!("leave_alt_screen: {e}")))?;
    terminal
        .show_cursor()
        .map_err(|e| ArgoError::Provider(format!("show_cursor: {e}")))?;
    Ok(())
}
